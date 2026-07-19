//! Trust boundary for user-selected local font files.
//!
//! A font the user picks from disk must never reach the web view as a raw path or
//! as unvalidated bytes. Every file is parsed and validated here first (all faces,
//! not just face zero, under strict resource limits). Only after it passes are the
//! bytes stashed in an in-memory registry behind an opaque handle. The frontend
//! receives that handle plus a synthetic preview family name and loads the font
//! through the `fontnest-preview` internal protocol, which serves bytes by handle
//! and nothing else.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use sha1::{Digest, Sha1};
use ttf_parser::{Face, name_id};

use crate::dto::{LocalFontFaceSummary, ValidatedLocalFont};

/// Largest local font file read into memory (mirrors the provider cap).
pub const MAX_LOCAL_FONT_BYTES: u64 = 64 * 1024 * 1024;
/// Largest number of faces validated inside a single collection.
const MAX_FACES: u32 = 256;
/// Longest name string accepted from a font's naming table.
const MAX_NAME_CHARS: usize = 255;
/// Length of an opaque preview handle in hex characters.
const HANDLE_HEX_LENGTH: usize = 40;

/// Most previews kept resident at once before the oldest is evicted.
const MAX_PREVIEW_ENTRIES: usize = 24;
/// Soft cap on total resident preview bytes; the oldest previews are evicted first.
const MAX_PREVIEW_TOTAL_BYTES: usize = 192 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum LocalFontError {
    #[error("the font file is larger than FontNest will load")]
    TooLarge,
    #[error("the file is not a supported desktop font")]
    InvalidFont,
    #[error("the font collection contains too many faces")]
    TooManyFaces,
    #[error("the font contains invalid naming metadata")]
    InvalidMetadata,
}

/// Bytes for one validated font, keyed by an opaque handle. Never stores a path.
struct PreviewEntry {
    handle: String,
    bytes: Arc<Vec<u8>>,
}

#[derive(Default)]
struct PreviewStoreInner {
    entries: VecDeque<PreviewEntry>,
    total_bytes: usize,
}

/// In-memory registry of validated preview bytes. Bounded by entry count and total
/// bytes; oldest previews are evicted first so memory stays flat across a session.
/// Cloning shares the same registry (the inner state is reference counted), so an
/// owned handle can be moved onto a blocking worker without copying any bytes.
#[derive(Default, Clone)]
pub struct PreviewStore {
    inner: Arc<Mutex<PreviewStoreInner>>,
}

impl PreviewStore {
    fn insert(&self, bytes: Vec<u8>) -> String {
        let handle = generate_handle();
        let size = bytes.len();
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.entries.push_back(PreviewEntry {
            handle: handle.clone(),
            bytes: Arc::new(bytes),
        });
        inner.total_bytes = inner.total_bytes.saturating_add(size);
        while inner.entries.len() > MAX_PREVIEW_ENTRIES
            || (inner.total_bytes > MAX_PREVIEW_TOTAL_BYTES && inner.entries.len() > 1)
        {
            let Some(evicted) = inner.entries.pop_front() else {
                break;
            };
            inner.total_bytes = inner.total_bytes.saturating_sub(evicted.bytes.len());
        }
        handle
    }

    /// Returns the validated bytes for a handle, or `None` for an unknown or
    /// malformed handle. The internal protocol serves bytes only through this.
    pub fn get(&self, handle: &str) -> Option<Arc<Vec<u8>>> {
        if !is_valid_handle(handle) {
            return None;
        }
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner
            .entries
            .iter()
            .find(|entry| entry.handle == handle)
            .map(|entry| Arc::clone(&entry.bytes))
    }
}

/// Validates `bytes` as a desktop font, registers them behind an opaque handle, and
/// returns a bounded summary the frontend can render and preview. The original path
/// is never returned; only a sanitized display file name is echoed back.
pub fn validate_and_register(
    store: &PreviewStore,
    bytes: Vec<u8>,
    file_name: &str,
) -> Result<ValidatedLocalFont, LocalFontError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_LOCAL_FONT_BYTES {
        return Err(LocalFontError::TooLarge);
    }
    if bytes.len() < 4 {
        return Err(LocalFontError::InvalidFont);
    }

    let collection_faces = ttf_parser::fonts_in_collection(&bytes);
    let is_collection = collection_faces.is_some();
    let face_count = collection_faces.unwrap_or(1);
    if face_count == 0 {
        return Err(LocalFontError::InvalidFont);
    }
    if face_count > MAX_FACES {
        return Err(LocalFontError::TooManyFaces);
    }

    // Validate every face. A collection that only partially validates is rejected
    // whole rather than registered.
    let mut faces = Vec::with_capacity(face_count as usize);
    for index in 0..face_count {
        faces.push(validate_face(&bytes, index)?);
    }

    let format = sfnt_format(&bytes, is_collection).to_owned();
    let handle = store.insert(bytes);
    let preview_family = format!("FontNestPreview-{}", &handle[..16]);
    let preview_url = preview_url(&handle);

    Ok(ValidatedLocalFont {
        handle,
        preview_family,
        preview_url,
        file_name: safe_file_name(file_name),
        format,
        face_count,
        faces,
    })
}

fn validate_face(bytes: &[u8], index: u32) -> Result<LocalFontFaceSummary, LocalFontError> {
    let face = Face::parse(bytes, index).map_err(|_| LocalFontError::InvalidFont)?;
    let glyph_count = face.number_of_glyphs();
    if glyph_count == 0 {
        return Err(LocalFontError::InvalidFont);
    }

    let family_name = unicode_name(&face, name_id::TYPOGRAPHIC_FAMILY)
        .or_else(|| unicode_name(&face, name_id::FAMILY))
        .ok_or(LocalFontError::InvalidMetadata)?;
    let subfamily_name = unicode_name(&face, name_id::TYPOGRAPHIC_SUBFAMILY)
        .or_else(|| unicode_name(&face, name_id::SUBFAMILY))
        .unwrap_or_else(|| "Regular".to_owned());
    let full_name = unicode_name(&face, name_id::FULL_NAME).unwrap_or_else(|| family_name.clone());
    let post_script_name =
        unicode_name(&face, name_id::POST_SCRIPT_NAME).ok_or(LocalFontError::InvalidMetadata)?;

    for value in [
        family_name.as_str(),
        subfamily_name.as_str(),
        full_name.as_str(),
        post_script_name.as_str(),
    ] {
        if value.trim().is_empty() || value.chars().count() > MAX_NAME_CHARS {
            return Err(LocalFontError::InvalidMetadata);
        }
    }

    Ok(LocalFontFaceSummary {
        face_index: index,
        family_name,
        subfamily_name,
        full_name,
        post_script_name,
        is_variable: face.is_variable(),
        glyph_count: u32::from(glyph_count),
    })
}

fn unicode_name(face: &Face<'_>, name_id: u16) -> Option<String> {
    face.names()
        .into_iter()
        .filter(|name| name.name_id == name_id && name.is_unicode())
        .find_map(|name| name.to_string())
        .map(|name| name.trim().to_owned())
}

/// Classifies the container from its SFNT header tag rather than the file extension.
fn sfnt_format(bytes: &[u8], is_collection: bool) -> &'static str {
    if is_collection {
        return "Font Collection";
    }
    let tag = bytes.get(0..4).unwrap_or(&[]);
    if tag == b"OTTO" {
        "OpenType (CFF)"
    } else if tag == b"true" || tag == b"\x00\x01\x00\x00" {
        "TrueType"
    } else if tag == b"typ1" {
        "PostScript (Type 1 SFNT)"
    } else {
        "SFNT"
    }
}

fn safe_file_name(file_name: &str) -> String {
    let base = Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("font");
    let trimmed: String = base.chars().take(120).collect();
    if trimmed.trim().is_empty() {
        "font".to_owned()
    } else {
        trimmed
    }
}

/// The internal-protocol URL the web view uses to fetch validated preview bytes.
fn preview_url(handle: &str) -> String {
    #[cfg(any(windows, target_os = "android"))]
    {
        format!("http://fontnest-preview.localhost/{handle}")
    }
    #[cfg(not(any(windows, target_os = "android")))]
    {
        format!("fontnest-preview://localhost/{handle}")
    }
}

/// True for a well-formed opaque handle (40 lowercase-or-upper hex characters).
pub fn is_valid_handle(handle: &str) -> bool {
    handle.len() == HANDLE_HEX_LENGTH && handle.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn generate_handle() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());

    let mut hasher = Sha1::new();
    hasher.update(counter.to_le_bytes());
    hasher.update(nanos.to_le_bytes());
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(HANDLE_HEX_LENGTH), |mut out, byte| {
            let _ = write!(&mut out, "{byte:02x}");
            out
        })
}

#[cfg(test)]
mod tests {
    use super::{
        LocalFontError, MAX_PREVIEW_ENTRIES, PreviewStore, is_valid_handle, validate_and_register,
    };

    #[test]
    fn rejects_non_font_bytes() {
        let store = PreviewStore::default();
        let error = validate_and_register(&store, vec![0_u8; 128], "junk.bin")
            .expect_err("garbage is not a font");
        assert!(matches!(error, LocalFontError::InvalidFont));
    }

    #[test]
    fn rejects_files_that_are_too_small() {
        let store = PreviewStore::default();
        let error = validate_and_register(&store, vec![0_u8; 2], "tiny.ttf")
            .expect_err("a 2-byte file cannot be a font");
        assert!(matches!(error, LocalFontError::InvalidFont));
    }

    #[test]
    fn handles_are_hex_and_unique_and_lookupable() {
        let store = PreviewStore::default();
        let first = store.insert(vec![1_u8, 2, 3, 4]);
        let second = store.insert(vec![5_u8, 6, 7, 8]);

        assert!(is_valid_handle(&first));
        assert_ne!(first, second);
        assert!(store.get(&first).is_some());
        assert!(store.get(&second).is_some());
        assert!(store.get("not-a-real-handle").is_none());
        assert!(store.get("../../secret").is_none());
    }

    #[test]
    fn registry_evicts_the_oldest_preview_over_the_entry_limit() {
        let store = PreviewStore::default();
        let mut handles = Vec::new();
        for _ in 0..(MAX_PREVIEW_ENTRIES + 3) {
            handles.push(store.insert(vec![9_u8; 16]));
        }

        assert!(store.get(&handles[0]).is_none(), "oldest must be evicted");
        assert!(
            store
                .get(handles.last().expect("at least one handle"))
                .is_some(),
            "newest must remain"
        );
        let inner = store.inner.lock().expect("registry lock");
        assert_eq!(inner.entries.len(), MAX_PREVIEW_ENTRIES);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn validates_an_installed_true_type_font_across_its_faces() {
        let bytes = std::fs::read(r"C:\Windows\Fonts\arial.ttf")
            .expect("Arial is part of the supported Windows font set");
        let store = PreviewStore::default();

        let validated = validate_and_register(&store, bytes, r"C:\Windows\Fonts\arial.ttf")
            .expect("Arial validates");

        assert_eq!(validated.faces.len(), validated.face_count as usize);
        assert!(validated.faces.iter().all(|face| face.glyph_count > 0));
        assert!(validated.preview_family.starts_with("FontNestPreview-"));
        assert_eq!(validated.file_name, "arial.ttf");
        assert!(is_valid_handle(&validated.handle));
        assert!(store.get(&validated.handle).is_some());
    }
}
