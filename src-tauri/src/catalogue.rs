use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use fontdb::{Database, FaceInfo, ID, Source, Style};

use crate::dto::{
    FontCatalogue, FontFaceInspection, FontFaceSummary, FontFamilySummary, FontGlyphOutline,
    FontGlyphVariationValue, FontOrigin,
};
use crate::font_identity::{
    FileIdentity, FileIdentityCache, IdentityKey, face_identity_key, family_identity_key,
    normalize_family_name,
};
use crate::font_inspection::{self, FontInspectionError};
use crate::font_origin;
use crate::font_variations;
use crate::managed_installations::ManagedInstallationRepository;

/// Largest font file read back out of the catalogue for a long-running inspection.
const MAX_FACE_BYTES: u64 = 64 * 1024 * 1024;

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

/// The bytes of one scanned face, held without borrowing the catalogue.
///
/// A memory-mapped or in-memory source is shared by reference count; a plain file is read
/// back from disk under a size cap. Either way the caller owns the bytes, so long parses
/// run with no catalogue lock held.
pub enum FaceBytes {
    Owned(Vec<u8>),
    Shared(Arc<dyn AsRef<[u8]> + Send + Sync>),
}

impl FaceBytes {
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes,
            Self::Shared(shared) => (**shared).as_ref(),
        }
    }
}

/// Loads the bytes for a face source that was cloned out of the catalogue.
///
/// This deliberately takes a `Source` rather than a store reference: resolving the source
/// is a short locked lookup, and reading and parsing then happen with the lock released.
pub fn read_face_bytes(source: &Source) -> Result<FaceBytes, CatalogueInspectionError> {
    match source {
        Source::Binary(shared) | Source::SharedFile(_, shared) => {
            Ok(FaceBytes::Shared(Arc::clone(shared)))
        }
        Source::File(path) => {
            let metadata =
                std::fs::metadata(path).map_err(|_| CatalogueInspectionError::DataUnavailable)?;
            if !metadata.is_file() || metadata.len() > MAX_FACE_BYTES {
                return Err(CatalogueInspectionError::DataUnavailable);
            }
            std::fs::read(path)
                .map(FaceBytes::Owned)
                .map_err(|_| CatalogueInspectionError::DataUnavailable)
        }
    }
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

    /// Clones the source and face index behind an opaque face ID.
    ///
    /// Cloning a `Source` is a path clone or a reference-count bump, so the catalogue lock
    /// is held only for the lookup. Reading the bytes and parsing them happen afterwards,
    /// which is what keeps a large export from blocking every other font command.
    pub fn face_source(&self, face_id: &str) -> Result<(Source, u32), CatalogueInspectionError> {
        let database_id = self
            .faces
            .get(face_id)
            .copied()
            .ok_or(CatalogueInspectionError::UnknownFace)?;
        let face = self
            .database
            .face(database_id)
            .ok_or(CatalogueInspectionError::UnknownFace)?;
        Ok((face.source.clone(), face.index))
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
    fn new(id: String, name: String) -> Self {
        Self {
            id,
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

    fn add_face(&mut self, scanned: &ScannedFace<'_>, face_id: String) {
        let face = scanned.face;
        let variable = face_is_variable(face);
        let style = style_value(face.style);
        let style_name = style_name(face.weight.0, face.style);
        let signature = format!("{}:{style}", face.weight.0);

        self.files.insert(scanned.file_key.clone());
        self.styles.insert(style_name.clone());
        self.weights.insert(face.weight.0);
        self.formats.insert(scanned.format.clone());
        self.origins.insert(scanned.origin);
        self.signatures
            .entry(signature)
            .or_default()
            .insert(scanned.file_key.clone());
        self.monospaced &= face.monospaced;
        self.variable |= variable;

        self.faces.push(FontFaceSummary {
            id: face_id,
            post_script_name: face.post_script_name.clone(),
            style_name,
            style: style.to_owned(),
            weight: face.weight.0,
            format: scanned.format.clone(),
            origin: scanned.origin,
            file_name: scanned.file_name.clone(),
            face_index: face.index,
            monospaced: face.monospaced,
            variable,
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
            origins: self.origins.into_iter().collect(),
            monospaced: self.monospaced,
            variable: self.variable,
            has_conflict,
            faces: self.faces,
        }
    }
}

/// One face as the scan found it, with everything its identity and its summary need.
struct ScannedFace<'a> {
    face: &'a FaceInfo,
    /// The family name as the font spells it, trimmed but not otherwise altered.
    family_name: String,
    origin: FontOrigin,
    file_name: String,
    /// The face's source as a comparable string. It names a file rather than identifying one, so
    /// it is used for grouping and conflict evidence and never for an ID.
    file_key: String,
    format: String,
}

/// Scans every font installed on this computer.
///
/// `ledger` is the store that remembers which ID belongs to which font. It is optional because
/// the catalogue must still work when the session is in read-only recovery: IDs are derived from
/// each font's own identity, so a session without the ledger produces the same IDs, it just
/// cannot learn about a rename the ledger had already recorded.
pub fn scan_installed_fonts(
    ledger: Option<&ManagedInstallationRepository>,
) -> ScannedFontCatalogue {
    let started = Instant::now();
    let mut database = Database::new();
    database.load_system_fonts();

    let face_count = count(database.len());
    let (families, face_ids) = build_families(&database, ledger);
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

/// Groups the scanned faces into families and gives both an ID.
///
/// Identity is settled before any summary is built, because an ID can depend on what the ledger
/// already knows and on which faces the scan has already seen. The faces are put in a fixed order
/// first, so both answers come out the same on every run over the same library.
fn build_families(
    database: &Database,
    ledger: Option<&ManagedInstallationRepository>,
) -> (Vec<FontFamilySummary>, BTreeMap<String, ID>) {
    let mut scanned: Vec<ScannedFace<'_>> = database.faces().filter_map(ScannedFace::new).collect();
    scanned.sort_by(|left, right| {
        left.file_key
            .cmp(&right.file_key)
            .then_with(|| left.face.index.cmp(&right.face.index))
            .then_with(|| left.face.post_script_name.cmp(&right.face.post_script_name))
    });

    let mut file_identities = FileIdentityCache::default();
    let mut keys = Vec::with_capacity(scanned.len() * 2);
    let mut claimed = HashSet::with_capacity(scanned.len());
    let mut family_keys = BTreeMap::<String, IdentityKey>::new();
    let mut face_keys = Vec::with_capacity(scanned.len());

    for entry in &scanned {
        if let btree_map::Entry::Vacant(slot) =
            family_keys.entry(normalize_family_name(&entry.family_name))
        {
            let key = family_identity_key(&entry.family_name);
            keys.push(key.clone());
            slot.insert(key);
        }

        let file = entry.file_identity(&mut file_identities);
        let mut key = face_identity_key(&file, entry.face.index, &entry.face.post_script_name);
        // Two faces answering to one identity would silently become one row. Platform file
        // identities make that impossible; the path fallback cannot promise it, so a repeat moves
        // onto a key of its own rather than overwrite the face already there.
        for ordinal in 1.. {
            if claimed.insert(key.key().to_owned()) {
                break;
            }
            key = key.disambiguated(ordinal);
        }
        keys.push(key.clone());
        face_keys.push(key);
    }

    let resolved = resolve_identities(ledger, &keys);
    let mut families = BTreeMap::<String, FontFamily>::new();
    let mut face_ids = BTreeMap::new();

    for (entry, key) in scanned.iter().zip(face_keys) {
        let grouping = normalize_family_name(&entry.family_name);
        let family_id = family_keys
            .get(&grouping)
            .map(|family_key| identity(&resolved, family_key))
            .expect("every scanned family was keyed in the first pass");
        let face_id = identity(&resolved, &key);

        families
            .entry(grouping)
            .or_insert_with(|| FontFamily::new(family_id, entry.family_name.clone()))
            .add_face(entry, face_id.clone());
        face_ids.insert(face_id, entry.face.id);
    }

    (
        families.into_values().map(FontFamily::finish).collect(),
        face_ids,
    )
}

/// Asks the ledger which IDs these identities already carry. A ledger that cannot be read is not
/// fatal: every identity falls back to the ID it derives, which is the ID the ledger would have
/// stored for a font it is meeting for the first time.
fn resolve_identities(
    ledger: Option<&ManagedInstallationRepository>,
    keys: &[IdentityKey],
) -> HashMap<String, String> {
    let Some(ledger) = ledger else {
        return HashMap::new();
    };
    ledger.resolve_font_identities(keys).unwrap_or_else(|error| {
        log::error!(
            "Font identities could not be read from the ledger, so this scan is using derived IDs: {error}"
        );
        HashMap::new()
    })
}

fn identity(resolved: &HashMap<String, String>, key: &IdentityKey) -> String {
    resolved
        .get(key.key())
        .cloned()
        .unwrap_or_else(|| key.derived_id().to_owned())
}

impl<'a> ScannedFace<'a> {
    fn new(face: &'a FaceInfo) -> Option<Self> {
        let (name, _language) = face.families.first()?;
        let family_name = name.trim();
        if family_name.is_empty() {
            return None;
        }

        let (origin, file_name, file_key, format) = face_file_details(face, family_name);
        Some(Self {
            face,
            family_name: family_name.to_owned(),
            origin,
            file_name,
            file_key,
            format,
        })
    }

    /// What the filesystem calls this face's file. A face `fontdb` holds in memory has no file, so
    /// its identity is the font itself.
    fn file_identity(&self, cache: &mut FileIdentityCache) -> FileIdentity {
        match &self.face.source {
            Source::File(path) => cache.identify(path),
            _ => FileIdentity::Embedded,
        }
    }
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

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use fontdb::Style;

    use super::{format_label, style_name};

    /// The preview pipeline the `preview_font_face` command runs, minus Tauri: resolve the
    /// scanned face, read it, lift it out of its collection, and check that what comes back
    /// is the face that was asked for. A collection is where this used to go wrong, since a
    /// `FontFace` in the web view always loads face zero of whatever bytes it is given.
    #[cfg(target_os = "windows")]
    #[test]
    fn collection_backed_faces_resolve_to_themselves_for_preview() {
        use crate::local_fonts;

        let scanned = super::scan_installed_fonts(None);
        let collection_faces: Vec<_> = scanned
            .catalogue
            .families
            .iter()
            .flat_map(|family| family.faces.iter())
            .filter(|face| face.file_name.to_lowercase().ends_with(".ttc"))
            .take(12)
            .collect();
        if collection_faces.is_empty() {
            return; // No collections installed on this machine.
        }

        for face in collection_faces {
            let (source, index) = scanned
                .store
                .face_source(&face.id)
                .expect("a scanned face resolves to its source");
            let bytes = super::read_face_bytes(&source).expect("the face reads back");
            let extracted =
                local_fonts::extract_face_sfnt(bytes.as_slice(), index).expect("the face extracts");
            let parsed = ttf_parser::Face::parse(&extracted, 0).expect("the rebuild parses");
            let post_script_name = parsed
                .names()
                .into_iter()
                .filter(|name| {
                    name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME && name.is_unicode()
                })
                .find_map(|name| name.to_string());

            assert_eq!(
                post_script_name.as_deref(),
                Some(face.post_script_name.as_str()),
                "face zero of the preview bytes must be {}",
                face.post_script_name
            );
        }
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

    /// The catalogue's own guarantee, on this machine's real fonts: every face and every family
    /// carries a well-formed opaque ID, and no two of them share one. A repeated ID is not a
    /// cosmetic problem, it silently drops a face from the catalogue and from the interface.
    #[cfg(target_os = "windows")]
    #[test]
    fn every_scanned_id_is_opaque_and_unique() {
        use std::collections::HashSet;

        use crate::font_identity::{IdentityKind, is_well_formed};

        let scanned = super::scan_installed_fonts(None);
        let mut ids = HashSet::new();

        for family in &scanned.catalogue.families {
            assert!(
                is_well_formed(IdentityKind::Family, &family.id),
                "{} has a malformed family ID: {}",
                family.name,
                family.id
            );
            assert!(ids.insert(family.id.clone()), "duplicate ID {}", family.id);

            for face in &family.faces {
                assert!(
                    is_well_formed(IdentityKind::Face, &face.id),
                    "{} has a malformed face ID: {}",
                    face.post_script_name,
                    face.id
                );
                assert!(ids.insert(face.id.clone()), "duplicate ID {}", face.id);
            }
        }
    }

    /// Going through the ledger must not change what anything is called, and it must leave a row
    /// behind for every family and face, because that record is what a later rename will read.
    #[cfg(target_os = "windows")]
    #[test]
    fn a_ledger_backed_scan_agrees_with_a_ledger_free_one_and_records_what_it_saw() {
        use crate::managed_installations::ManagedInstallationRepository;

        let temp = tempfile::tempdir().expect("a temporary directory");
        let ledger = ManagedInstallationRepository::in_app_data_dir(temp.path());
        ledger.initialize().expect("the migrations");

        let without = super::scan_installed_fonts(None).catalogue;
        let with = super::scan_installed_fonts(Some(&ledger)).catalogue;

        let ids = |catalogue: &crate::dto::FontCatalogue| {
            catalogue
                .families
                .iter()
                .flat_map(|family| {
                    std::iter::once(family.id.clone())
                        .chain(family.faces.iter().map(|face| face.id.clone()))
                })
                .collect::<Vec<String>>()
        };
        let expected = ids(&without);
        assert_eq!(expected, ids(&with));

        let connection = rusqlite::Connection::open(temp.path().join("fontnest.sqlite3"))
            .expect("the ledger opens");
        let recorded: i64 = connection
            .query_row("SELECT COUNT(*) FROM font_identities", [], |row| row.get(0))
            .expect("the identities are counted");
        assert_eq!(
            usize::try_from(recorded).expect("a small count"),
            expected.len()
        );
    }

    /// Scanning the same library twice must name everything the same way, or a pinned family and
    /// a saved preview would stop matching after a restart.
    #[cfg(target_os = "windows")]
    #[test]
    fn two_scans_of_one_library_agree_on_every_id() {
        let first = super::scan_installed_fonts(None).catalogue;
        let second = super::scan_installed_fonts(None).catalogue;

        let names = |catalogue: &crate::dto::FontCatalogue| {
            catalogue
                .families
                .iter()
                .map(|family| {
                    (
                        family.id.clone(),
                        family.faces.iter().map(|face| face.id.clone()).collect(),
                    )
                })
                .collect::<Vec<(String, Vec<String>)>>()
        };

        assert_eq!(names(&first), names(&second));
    }
}
