use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use fontdb::{Database, FaceInfo, Source, Style};

use crate::dto::{FontCatalogue, FontFaceSummary, FontFamilySummary};

#[derive(Debug)]
struct FontFamily {
    id: String,
    name: String,
    faces: Vec<FontFaceSummary>,
    files: BTreeSet<String>,
    styles: BTreeSet<String>,
    weights: BTreeSet<u16>,
    formats: BTreeSet<String>,
    sources: BTreeSet<String>,
    signatures: BTreeMap<String, BTreeSet<String>>,
    monospaced: bool,
}

impl FontFamily {
    fn new(name: String) -> Self {
        Self {
            id: family_id(&name),
            name,
            faces: Vec::new(),
            files: BTreeSet::new(),
            styles: BTreeSet::new(),
            weights: BTreeSet::new(),
            formats: BTreeSet::new(),
            sources: BTreeSet::new(),
            signatures: BTreeMap::new(),
            monospaced: true,
        }
    }

    fn add_face(&mut self, face: &FaceInfo) {
        let (source, file_name, file_key, format) = face_file_details(face);
        let style = style_value(face.style);
        let style_name = style_name(face.weight.0, face.style);
        let signature = format!("{}:{style}", face.weight.0);

        self.files.insert(file_key.clone());
        self.styles.insert(style_name.clone());
        self.weights.insert(face.weight.0);
        self.formats.insert(format.clone());
        self.sources.insert(source.clone());
        self.signatures
            .entry(signature)
            .or_default()
            .insert(file_key.clone());
        self.monospaced &= face.monospaced;

        self.faces.push(FontFaceSummary {
            id: format!("{}:{}:{}", self.id, face.index, face.post_script_name),
            post_script_name: face.post_script_name.clone(),
            style_name,
            style: style.to_owned(),
            weight: face.weight.0,
            format,
            source,
            file_name,
            face_index: face.index,
            monospaced: face.monospaced,
        });
    }

    fn finish(mut self) -> FontFamilySummary {
        self.faces.sort_by(|left, right| {
            left.weight
                .cmp(&right.weight)
                .then_with(|| left.style.cmp(&right.style))
                .then_with(|| left.file_name.cmp(&right.file_name))
        });

        let has_conflict = self
            .signatures
            .values()
            .any(|source_files| source_files.len() > 1);

        FontFamilySummary {
            id: self.id,
            name: self.name,
            face_count: count(self.faces.len()),
            file_count: count(self.files.len()),
            styles: self.styles.into_iter().collect(),
            weights: self.weights.into_iter().collect(),
            formats: self.formats.into_iter().collect(),
            sources: self.sources.into_iter().collect(),
            monospaced: self.monospaced,
            has_conflict,
            faces: self.faces,
        }
    }
}

pub fn scan_installed_fonts() -> FontCatalogue {
    let started = Instant::now();
    let mut database = Database::new();
    database.load_system_fonts();

    let face_count = count(database.len());
    let mut families = BTreeMap::<String, FontFamily>::new();

    for face in database.faces() {
        let Some((name, _language)) = face.families.first() else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }

        families
            .entry(name.to_lowercase())
            .or_insert_with(|| FontFamily::new(name.to_owned()))
            .add_face(face);
    }

    let families: Vec<_> = families.into_values().map(FontFamily::finish).collect();
    let conflict_count = count(families.iter().filter(|family| family.has_conflict).count());

    FontCatalogue {
        family_count: count(families.len()),
        face_count,
        conflict_count,
        families,
        scan_duration_ms: u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX),
    }
}

fn face_file_details(face: &FaceInfo) -> (String, String, String, String) {
    let Source::File(path) = &face.source else {
        return (
            "Embedded".to_owned(),
            "In-memory font".to_owned(),
            format!("embedded:{}", face.post_script_name),
            "Unknown".to_owned(),
        );
    };

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown file")
        .to_owned();
    let file_key = path.to_string_lossy().into_owned();

    (
        source_label(path).to_owned(),
        file_name,
        file_key,
        format_label(path).to_owned(),
    )
}

fn source_label(path: &Path) -> &'static str {
    let normalized = path.to_string_lossy().replace('\\', "/").to_lowercase();

    if normalized.contains("/appdata/local/microsoft/windows/fonts/")
        || normalized.contains("/appdata/roaming/microsoft/windows/fonts/")
        || normalized.contains("/.local/share/fonts/")
        || normalized.contains("/library/fonts/") && !normalized.starts_with("/system/")
    {
        "User"
    } else if normalized.contains("/windows/fonts/")
        || normalized.starts_with("/system/library/fonts/")
        || normalized.starts_with("/usr/share/fonts/")
    {
        "System"
    } else {
        "Other"
    }
}

fn format_label(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("ttf") => "TrueType",
        Some("otf") => "OpenType",
        Some("ttc") => "TrueType collection",
        Some("otc") => "OpenType collection",
        _ => "Unknown",
    }
}

fn style_value(style: Style) -> &'static str {
    match style {
        Style::Normal => "normal",
        Style::Italic => "italic",
        Style::Oblique => "oblique",
    }
}

fn style_name(weight: u16, style: Style) -> String {
    let weight_name = match weight {
        0..=150 => "Thin",
        151..=250 => "Extra Light",
        251..=350 => "Light",
        351..=450 => "Regular",
        451..=550 => "Medium",
        551..=650 => "Semi Bold",
        651..=750 => "Bold",
        751..=850 => "Extra Bold",
        _ => "Black",
    };

    match (weight_name, style) {
        ("Regular", Style::Normal) => "Regular".to_owned(),
        ("Regular", Style::Italic) => "Italic".to_owned(),
        ("Regular", Style::Oblique) => "Oblique".to_owned(),
        (_, Style::Normal) => weight_name.to_owned(),
        (_, Style::Italic) => format!("{weight_name} Italic"),
        (_, Style::Oblique) => format!("{weight_name} Oblique"),
    }
}

fn family_id(name: &str) -> String {
    name.trim().to_lowercase()
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use fontdb::Style;

    use super::{family_id, format_label, source_label, style_name};

    #[test]
    fn family_ids_are_normalized_for_selection() {
        assert_eq!(family_id("  Source Serif 4 "), "source serif 4");
    }

    #[test]
    fn style_names_combine_weight_and_posture() {
        assert_eq!(style_name(400, Style::Italic), "Italic");
        assert_eq!(style_name(700, Style::Italic), "Bold Italic");
    }

    #[test]
    fn windows_font_sources_keep_system_and_user_boundaries_explicit() {
        assert_eq!(
            source_label(Path::new("C:\\Windows\\Fonts\\arial.ttf")),
            "System"
        );
        assert_eq!(
            source_label(Path::new(
                "C:\\Users\\Akari\\AppData\\Local\\Microsoft\\Windows\\Fonts\\Inter.ttf"
            )),
            "User"
        );
        assert_eq!(
            format_label(Path::new("C:\\Windows\\Fonts\\arial.ttf")),
            "TrueType"
        );
    }
}
