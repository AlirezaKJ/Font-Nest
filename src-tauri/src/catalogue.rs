use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use fontdb::{Database, FaceInfo, ID, Source, Style};
use sha1::{Digest, Sha1};

use crate::dto::{
    FontCatalogue, FontFaceInspection, FontFaceSummary, FontFamilySummary, FontGlyphOutline,
    FontGlyphVariationValue, FontOrigin, FontParserJsonExport,
};
use crate::font_inspection::{self, FontInspectionError};
use crate::font_origin;
use crate::font_variations;

pub struct ScannedFontCatalogue {
    pub catalogue: FontCatalogue,
    pub store: FontCatalogueStore,
}

pub struct FontCatalogueStore {
    database: Database,
    faces: BTreeMap<String, ID>,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogueInspectionError {
    #[error("the requested face ID is not in the current catalogue")]
    UnknownFace,
    #[error("the font data could not be loaded")]
    DataUnavailable,
    #[error("the requested face is not backed by a file on disk")]
    NotFileBacked,
    #[error(transparent)]
    Parser(#[from] FontInspectionError),
}

impl FontCatalogueStore {
    pub fn inspect_face(
        &self,
        face_id: &str,
    ) -> Result<FontFaceInspection, CatalogueInspectionError> {
        self.with_face_data(face_id, |data, index| {
            font_inspection::inspect_face(face_id, data, index)
        })
    }

    pub fn export_face_json(
        &self,
        face_id: &str,
    ) -> Result<FontParserJsonExport, CatalogueInspectionError> {
        self.with_face_data(face_id, |data, index| {
            font_inspection::export_face_json(face_id, data, index)
        })
    }

    pub fn inspect_glyph_outline(
        &self,
        face_id: &str,
        codepoint: u32,
        variations: &[FontGlyphVariationValue],
    ) -> Result<FontGlyphOutline, CatalogueInspectionError> {
        self.with_face_data(face_id, |data, index| {
            font_inspection::inspect_glyph_outline(face_id, data, index, codepoint, variations)
        })
    }

    /// Resolves an opaque face ID back to the file it was scanned from.
    ///
    /// Face IDs are one-way digests, so this lookup is the only way back to a path and it
    /// stays behind the command layer: callers decide whether the path is revealed to the
    /// user or handed to the platform file manager.
    pub fn face_file_path(&self, face_id: &str) -> Result<PathBuf, CatalogueInspectionError> {
        let database_id = self
            .faces
            .get(face_id)
            .copied()
            .ok_or(CatalogueInspectionError::UnknownFace)?;
        let face = self
            .database
            .face(database_id)
            .ok_or(CatalogueInspectionError::UnknownFace)?;
        match &face.source {
            Source::File(path) => Ok(path.clone()),
            _ => Err(CatalogueInspectionError::NotFileBacked),
        }
    }

    fn with_face_data<T>(
        &self,
        face_id: &str,
        parse: impl FnOnce(&[u8], u32) -> Result<T, FontInspectionError>,
    ) -> Result<T, CatalogueInspectionError> {
        let database_id = self
            .faces
            .get(face_id)
            .copied()
            .ok_or(CatalogueInspectionError::UnknownFace)?;
        self.database
            .with_face_data(database_id, parse)
            .ok_or(CatalogueInspectionError::DataUnavailable)?
            .map_err(CatalogueInspectionError::from)
    }
}

#[derive(Debug)]
struct FontFamily {
    id: String,
    name: String,
    faces: Vec<FontFaceSummary>,
    files: BTreeSet<String>,
    styles: BTreeSet<String>,
    weights: BTreeSet<u16>,
    formats: BTreeSet<String>,
    origins: BTreeSet<FontOrigin>,
    signatures: BTreeMap<String, BTreeSet<String>>,
    monospaced: bool,
    variable: bool,
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
            origins: BTreeSet::new(),
            signatures: BTreeMap::new(),
            monospaced: true,
            variable: false,
        }
    }

    fn add_face(&mut self, face: &FaceInfo) -> String {
        let (origin, file_name, file_key, format) = face_file_details(face, &self.name);
        let variable = face_is_variable(face);
        let style = style_value(face.style);
        let style_name = style_name(face.weight.0, face.style);
        let signature = format!("{}:{style}", face.weight.0);
        let face_id = opaque_face_id(&file_key, face.index, &face.post_script_name);

        self.files.insert(file_key.clone());
        self.styles.insert(style_name.clone());
        self.weights.insert(face.weight.0);
        self.formats.insert(format.clone());
        self.origins.insert(origin);
        self.signatures
            .entry(signature)
            .or_default()
            .insert(file_key.clone());
        self.monospaced &= face.monospaced;
        self.variable |= variable;

        self.faces.push(FontFaceSummary {
            id: face_id.clone(),
            post_script_name: face.post_script_name.clone(),
            style_name,
            style: style.to_owned(),
            weight: face.weight.0,
            format,
            origin,
            file_name,
            face_index: face.index,
            monospaced: face.monospaced,
            variable,
        });
        face_id
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
            origins: self.origins.into_iter().collect(),
            monospaced: self.monospaced,
            variable: self.variable,
            has_conflict,
            faces: self.faces,
        }
    }
}

pub fn scan_installed_fonts() -> ScannedFontCatalogue {
    let started = Instant::now();
    let mut database = Database::new();
    database.load_system_fonts();

    let face_count = count(database.len());
    let mut families = BTreeMap::<String, FontFamily>::new();
    let mut face_ids = BTreeMap::new();

    for face in database.faces() {
        let Some((name, _language)) = face.families.first() else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }

        let face_id = families
            .entry(name.to_lowercase())
            .or_insert_with(|| FontFamily::new(name.to_owned()))
            .add_face(face);
        face_ids.insert(face_id, face.id);
    }

    let families: Vec<_> = families.into_values().map(FontFamily::finish).collect();
    let conflict_count = count(families.iter().filter(|family| family.has_conflict).count());

    ScannedFontCatalogue {
        catalogue: FontCatalogue {
            family_count: count(families.len()),
            face_count,
            conflict_count,
            families,
            scan_duration_ms: u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX),
        },
        store: FontCatalogueStore {
            database,
            faces: face_ids,
        },
    }
}

fn opaque_face_id(file_key: &str, face_index: u32, post_script_name: &str) -> String {
    use std::fmt::Write;

    let mut digest = Sha1::new();
    digest.update(file_key.as_bytes());
    digest.update(face_index.to_be_bytes());
    digest.update(post_script_name.as_bytes());
    let mut face_id = String::with_capacity("face:".len() + 40);
    face_id.push_str("face:");
    for byte in digest.finalize() {
        let _ = write!(face_id, "{byte:02x}");
    }
    face_id
}

/// Only file-backed faces can be checked; fontdb's in-memory sources have no path to read.
fn face_is_variable(face: &FaceInfo) -> bool {
    match &face.source {
        Source::File(path) => font_variations::face_is_variable(path, face.index),
        _ => false,
    }
}

fn face_file_details(face: &FaceInfo, family_name: &str) -> (FontOrigin, String, String, String) {
    let Source::File(path) = &face.source else {
        return (
            FontOrigin::Unknown,
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
        font_origin::classify(path, family_name),
        file_name,
        file_key,
        format_label(path).to_owned(),
    )
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

    use super::{family_id, format_label, opaque_face_id, style_name};

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
    fn font_formats_are_named_from_the_file_extension() {
        assert_eq!(
            format_label(Path::new("C:\\Windows\\Fonts\\arial.ttf")),
            "TrueType"
        );
        assert_eq!(
            format_label(Path::new("/Library/Fonts/Inter.otf")),
            "OpenType"
        );
    }

    #[test]
    fn opaque_face_ids_are_stable_and_do_not_expose_source_paths() {
        let path = "C:\\Windows\\Fonts\\arial.ttf";
        let id = opaque_face_id(path, 0, "ArialMT");

        assert_eq!(id, opaque_face_id(path, 0, "ArialMT"));
        assert_ne!(id, opaque_face_id(path, 1, "ArialMT"));
        assert!(id.starts_with("face:"));
        assert!(!id.contains("Windows"));
        assert!(!id.contains("Arial"));
    }
}
