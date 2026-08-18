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
    /// Content key for bytes a caller can ask for again. A user-selected file has none:
    /// every pick is its own registration, because the same path may hold new bytes.
    key: Option<String>,
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
        self.register(None, bytes)
    }

    /// Registers bytes that can be requested again under the same `key`, reusing the
    /// handle already held for that key instead of registering a second copy.
    ///
    /// Provider artifacts are content addressed: the key is the digest that verified the
    /// download, so the same bytes always answer with the same handle and re-previewing a
    /// family the user already looked at costs nothing. Reuse also refreshes the entry, so
    /// a font still being looked at is not the one evicted next.
    pub fn insert_keyed(&self, key: &str, bytes: Vec<u8>) -> String {
        self.register(Some(key), bytes)
    }

    fn register(&self, key: Option<&str>, bytes: Vec<u8>) -> String {
        let size = bytes.len();
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if let Some(key) = key {
            let held = inner
                .entries
                .iter()
                .position(|entry| entry.key.as_deref() == Some(key));
            if let Some(entry) = held.and_then(|index| inner.entries.remove(index)) {
                let handle = entry.handle.clone();
                inner.entries.push_back(entry);
                return handle;
            }
        }

        let handle = generate_handle();
        inner.entries.push_back(PreviewEntry {
            handle: handle.clone(),
            key: key.map(ToOwned::to_owned),
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
pub fn preview_url(handle: &str) -> String {
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

/// Largest standalone face rebuilt out of a collection.
const MAX_EXTRACTED_FACE_BYTES: usize = 64 * 1024 * 1024;
/// Most tables accepted in one face's table directory.
const MAX_TABLE_RECORDS: usize = 512;
/// Magic the checksum of a whole SFNT file must add up to.
const CHECKSUM_MAGIC: u32 = 0xB1B0_AFBA;

/// Returns one face of a font file as a standalone SFNT.
///
/// A `FontFace` in the web view cannot name a face inside a collection: it always loads
/// face zero. Previewing face three of a `.ttc` therefore means rebuilding that face as
/// its own font, carrying only its own table directory, before the bytes leave the
/// backend. Files that already hold a single face are returned unchanged.
///
/// The rebuild copies table bytes verbatim, so every per-table checksum still holds; only
/// the file-wide `checkSumAdjustment` in `head` is recomputed. A `DSIG` signature covers
/// the original layout and cannot survive the rebuild, so it is dropped rather than
/// carried forward as a signature that no longer verifies.
pub fn extract_face_sfnt(bytes: &[u8], face_index: u32) -> Result<Vec<u8>, LocalFontError> {
    let Some(face_count) = ttf_parser::fonts_in_collection(bytes) else {
        // Not a collection: the file is the face, and any index but zero is a lie.
        if face_index != 0 {
            return Err(LocalFontError::InvalidFont);
        }
        return Ok(bytes.to_vec());
    };
    if face_index >= face_count {
        return Err(LocalFontError::InvalidFont);
    }

    let sfnt_version = collection_face_sfnt_version(bytes, face_index)?;
    let raw =
        ttf_parser::RawFace::parse(bytes, face_index).map_err(|_| LocalFontError::InvalidFont)?;

    let dsig = ttf_parser::Tag::from_bytes(b"DSIG");
    let mut tables: Vec<(ttf_parser::Tag, u32, &[u8])> = Vec::new();
    for record in raw.table_records {
        if record.tag == dsig {
            continue;
        }
        let start = usize::try_from(record.offset).map_err(|_| LocalFontError::InvalidFont)?;
        let length = usize::try_from(record.length).map_err(|_| LocalFontError::InvalidFont)?;
        let end = start
            .checked_add(length)
            .ok_or(LocalFontError::InvalidFont)?;
        let data = bytes.get(start..end).ok_or(LocalFontError::InvalidFont)?;
        if tables.iter().any(|(tag, _, _)| *tag == record.tag) {
            continue;
        }
        tables.push((record.tag, record.check_sum, data));
        if tables.len() > MAX_TABLE_RECORDS {
            return Err(LocalFontError::InvalidFont);
        }
    }
    if tables.is_empty() {
        return Err(LocalFontError::InvalidFont);
    }
    // A table directory is required to be sorted by tag, and `RawFace::table` binary
    // searches it, so the rebuilt directory has to keep that order.
    tables.sort_by_key(|(tag, _, _)| tag.0);

    let table_count = u16::try_from(tables.len()).map_err(|_| LocalFontError::InvalidFont)?;
    let directory_length = 12 + tables.len() * 16;
    let total_length = tables
        .iter()
        .try_fold(directory_length, |total, (_, _, data)| {
            total
                .checked_add(padded_length(data.len()))
                .filter(|length| *length <= MAX_EXTRACTED_FACE_BYTES)
                .ok_or(LocalFontError::TooLarge)
        })?;

    // The offset table repeats the binary-search hints every SFNT header carries.
    let entry_selector =
        u16::try_from(table_count.ilog2()).map_err(|_| LocalFontError::InvalidFont)?;
    let search_range = 16 * (1_u16 << entry_selector);
    let range_shift = table_count * 16 - search_range;

    let mut out = Vec::with_capacity(total_length);
    out.extend_from_slice(&sfnt_version.to_be_bytes());
    out.extend_from_slice(&table_count.to_be_bytes());
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&range_shift.to_be_bytes());

    let mut offset = u32::try_from(directory_length).map_err(|_| LocalFontError::InvalidFont)?;
    let mut head_offset: Option<usize> = None;
    for (tag, check_sum, data) in &tables {
        if *tag == ttf_parser::Tag::from_bytes(b"head") {
            head_offset = usize::try_from(offset).ok();
        }
        out.extend_from_slice(&tag.0.to_be_bytes());
        out.extend_from_slice(&check_sum.to_be_bytes());
        out.extend_from_slice(&offset.to_be_bytes());
        out.extend_from_slice(
            &u32::try_from(data.len())
                .map_err(|_| LocalFontError::InvalidFont)?
                .to_be_bytes(),
        );
        offset = offset
            .checked_add(
                u32::try_from(padded_length(data.len()))
                    .map_err(|_| LocalFontError::InvalidFont)?,
            )
            .ok_or(LocalFontError::TooLarge)?;
    }

    for (_, _, data) in &tables {
        out.extend_from_slice(data);
        out.resize(padded_length(out.len()), 0);
    }

    // `head.checkSumAdjustment` describes the whole file, so the copied value belongs to
    // the collection rather than to this rebuilt font. Zero it, checksum the file, and
    // write the value that makes the file sum to the magic constant.
    if let Some(head) = head_offset {
        let field = head.checked_add(8).ok_or(LocalFontError::InvalidFont)?;
        let slot = out
            .get_mut(field..field + 4)
            .ok_or(LocalFontError::InvalidFont)?;
        slot.copy_from_slice(&[0, 0, 0, 0]);
        let adjustment = CHECKSUM_MAGIC.wrapping_sub(sfnt_checksum(&out));
        out[field..field + 4].copy_from_slice(&adjustment.to_be_bytes());
    }

    Ok(out)
}

/// Reads the `sfntVersion` of one face out of a collection's header.
///
/// Deriving it from the tables present would guess; the face's own table directory states
/// it, so it is read from there.
fn collection_face_sfnt_version(bytes: &[u8], face_index: u32) -> Result<u32, LocalFontError> {
    let entry = usize::try_from(face_index)
        .ok()
        .and_then(|index| index.checked_mul(4))
        .and_then(|offset| offset.checked_add(12))
        .ok_or(LocalFontError::InvalidFont)?;
    let offset = bytes
        .get(entry..entry + 4)
        .and_then(|slice| slice.try_into().ok())
        .map(u32::from_be_bytes)
        .ok_or(LocalFontError::InvalidFont)?;
    let start = usize::try_from(offset).map_err(|_| LocalFontError::InvalidFont)?;
    bytes
        .get(start..start + 4)
        .and_then(|slice| slice.try_into().ok())
        .map(u32::from_be_bytes)
        .ok_or(LocalFontError::InvalidFont)
}

/// Rounds a length up to the four-byte boundary every SFNT table starts on.
const fn padded_length(length: usize) -> usize {
    length.div_ceil(4) * 4
}

/// Sums a whole SFNT file as big-endian `u32` words, padding a short tail with zeroes.
fn sfnt_checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0_u32, |sum, chunk| {
        let mut word = [0_u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        LocalFontError, MAX_PREVIEW_ENTRIES, PreviewStore, extract_face_sfnt, is_valid_handle,
        preview_url, unicode_name, validate_and_register,
    };

    #[test]
    fn a_preview_url_carries_only_the_handle() {
        let url = preview_url("0123456789abcdef0123456789abcdef01234567");
        assert!(url.contains("fontnest-preview"));
        assert!(url.ends_with("/0123456789abcdef0123456789abcdef01234567"));
        // Bytes are fetched, never carried. A data URL would put the font in the document
        // itself, which is what this transport exists to avoid.
        assert!(!url.starts_with("data:"));
    }

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
    fn the_same_key_answers_with_one_registration() {
        let store = PreviewStore::default();
        let first = store.insert_keyed("sha-of-inter-regular", vec![1_u8; 32]);
        let again = store.insert_keyed("sha-of-inter-regular", vec![1_u8; 32]);
        let other = store.insert_keyed("sha-of-inter-italic", vec![2_u8; 32]);

        assert_eq!(first, again, "the same artifact reuses its handle");
        assert_ne!(first, other);
        let inner = store.inner.lock().expect("registry lock");
        assert_eq!(inner.entries.len(), 2, "no second copy of the same bytes");
    }

    #[test]
    fn asking_for_a_keyed_preview_again_keeps_it_out_of_the_eviction_queue() {
        let store = PreviewStore::default();
        let kept = store.insert_keyed("still-on-screen", vec![3_u8; 16]);
        for _ in 0..(MAX_PREVIEW_ENTRIES - 1) {
            store.insert(vec![9_u8; 16]);
        }
        // Requesting it again moves it to the newest end, so the fonts registered
        // around it are evicted first.
        assert_eq!(store.insert_keyed("still-on-screen", vec![3_u8; 16]), kept);
        for _ in 0..(MAX_PREVIEW_ENTRIES - 1) {
            store.insert(vec![9_u8; 16]);
        }

        assert!(
            store.get(&kept).is_some(),
            "a preview that is still being asked for must survive"
        );
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

    #[test]
    fn passes_single_face_files_through_unchanged() {
        let bytes = vec![0_u8, 1, 0, 0, 7, 7, 7];
        let extracted = extract_face_sfnt(&bytes, 0).expect("a single-face file needs no rebuild");
        assert_eq!(extracted, bytes);
    }

    #[test]
    fn rejects_a_face_index_a_single_face_file_does_not_have() {
        let bytes = vec![0_u8, 1, 0, 0, 7, 7, 7];
        let error = extract_face_sfnt(&bytes, 1).expect_err("face one does not exist");
        assert!(matches!(error, LocalFontError::InvalidFont));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn rebuilds_each_collection_face_as_its_own_font() {
        let path = r"C:\Windows\Fonts\cambria.ttc";
        let Ok(bytes) = std::fs::read(path) else {
            return; // The collection is not installed on this machine.
        };
        let face_count =
            ttf_parser::fonts_in_collection(&bytes).expect("cambria.ttc is a collection");
        assert!(face_count > 1, "the fixture must hold more than one face");

        for index in 0..face_count {
            let extracted = extract_face_sfnt(&bytes, index).expect("every face extracts");
            assert!(
                ttf_parser::fonts_in_collection(&extracted).is_none(),
                "an extracted face is a standalone font, not a collection"
            );

            // Face zero of the rebuilt font must be the face that was asked for, which is
            // the whole point: a `FontFace` in the web view can only ever load face zero.
            let rebuilt = ttf_parser::Face::parse(&extracted, 0).expect("the rebuild parses");
            let original = ttf_parser::Face::parse(&bytes, index).expect("the original parses");
            assert_eq!(rebuilt.number_of_glyphs(), original.number_of_glyphs());
            assert_eq!(
                unicode_name(&rebuilt, ttf_parser::name_id::POST_SCRIPT_NAME),
                unicode_name(&original, ttf_parser::name_id::POST_SCRIPT_NAME)
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn an_extracted_face_keeps_the_exact_character_coverage_of_the_original() {
        let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\cambria.ttc") else {
            return;
        };
        let face_count =
            ttf_parser::fonts_in_collection(&bytes).expect("cambria.ttc is a collection");

        for index in 0..face_count {
            let extracted = extract_face_sfnt(&bytes, index).expect("every face extracts");
            let rebuilt = ttf_parser::Face::parse(&extracted, 0).expect("the rebuild parses");
            let original = ttf_parser::Face::parse(&bytes, index).expect("the original parses");

            // Coverage is the whole point of the rebuild: what the character grid draws has
            // to be the same set of characters the inspector lists for this face.
            for codepoint in (0x20_u32..0x2FFF).filter_map(char::from_u32) {
                assert_eq!(
                    rebuilt.glyph_index(codepoint).map(|glyph| glyph.0),
                    original.glyph_index(codepoint).map(|glyph| glyph.0),
                    "coverage of U+{:04X} changed in face {index}",
                    codepoint as u32
                );
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn an_extracted_collection_face_validates_and_registers() {
        let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\cambria.ttc") else {
            return;
        };
        let face_count =
            ttf_parser::fonts_in_collection(&bytes).expect("cambria.ttc is a collection");
        let last = face_count - 1;
        let extracted = extract_face_sfnt(&bytes, last).expect("the last face extracts");

        let store = PreviewStore::default();
        let validated = validate_and_register(&store, extracted, "cambria.ttc")
            .expect("an extracted face validates like any other font");

        assert_eq!(validated.face_count, 1, "the preview holds one face only");
        assert!(store.get(&validated.handle).is_some());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn rejects_a_face_index_past_the_end_of_a_collection() {
        let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\cambria.ttc") else {
            return;
        };
        let face_count =
            ttf_parser::fonts_in_collection(&bytes).expect("cambria.ttc is a collection");
        let error =
            extract_face_sfnt(&bytes, face_count).expect_err("one past the end is not a face");
        assert!(matches!(error, LocalFontError::InvalidFont));
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
