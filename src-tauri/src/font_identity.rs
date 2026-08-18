//! Decides what makes two scanned fonts the same font, and turns that answer into the opaque
//! IDs the rest of `FontNest` selects, saves, and links against.
//!
//! An ID derived from a path breaks the moment a file is renamed, and an ID derived from a
//! display name collides the moment two families differ only in case. Both were true of the
//! previous scan-local IDs, so a pinned family stopped matching after an ordinary rename.
//! Identity here is built from things a rename cannot touch:
//!
//! - A file is identified by the record the filesystem keeps for it: volume serial number plus
//!   file index on Windows, device plus inode elsewhere. That record survives a rename and a move
//!   within the same volume, and it stays distinct for two copies of identical bytes, which keeps
//!   a duplicate installed in two places from collapsing into one row.
//! - A face is that file identity plus its index in the file and its `PostScript` name, so a font
//!   replaced in place by a different font is correctly a different face.
//! - A family is its name, normalized, because a family has no identity beyond its name. The ID
//!   is a digest rather than the name itself, so case and spacing variants can no longer produce
//!   two spellings of one ID, and the ledger can later point the key at an existing ID when
//!   explicit user grouping lands.
//!
//! IDs are derived rather than minted, so a session that cannot open the ledger still produces
//! the IDs it would have produced with it. The ledger records the mapping, which is what lets a
//! later rename or merge keep an ID that no longer matches its derivation, and what makes a real
//! digest collision visible instead of silent.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Folded into every digest. Bumping it changes every derived ID, so it changes only when the
/// identity rules change in a way that should deliberately invalidate what users have saved.
const IDENTITY_VERSION: &[u8] = b"fontnest-identity-v1";

pub const FAMILY_ID_PREFIX: &str = "family:";
pub const FACE_ID_PREFIX: &str = "face:";

/// Hex characters after the prefix: the first 128 bits of the digest. Long enough that a
/// collision across a library of any realistic size is not a practical concern, short enough to
/// stay readable in a log line.
pub const ID_HEX_LENGTH: usize = 32;

/// What the filesystem calls one file, independent of where it currently sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileIdentity {
    /// The record the filesystem keeps for the file: volume plus file index on Windows, device
    /// plus inode elsewhere. Survives a rename and a move within the volume.
    Platform { volume: u64, index: u64 },
    /// No usable file record, so the path is all that is left. This is the old, rename-fragile
    /// behavior, kept only so an unreadable or exotic file still gets a working ID.
    Path(PathBuf),
    /// A face `fontdb` holds in memory, which has no file at all.
    Embedded,
}

impl FileIdentity {
    fn write_into(&self, digest: &mut Sha256) {
        match self {
            Self::Platform { volume, index } => {
                digest.update(b"platform\0");
                digest.update(volume.to_be_bytes());
                digest.update(index.to_be_bytes());
            }
            Self::Path(path) => {
                digest.update(b"path\0");
                digest.update(path.to_string_lossy().as_bytes());
            }
            Self::Embedded => digest.update(b"embedded\0"),
        }
    }
}

/// Resolves file identities once per path for the length of one scan.
///
/// Every face of a collection asks the same question about the same file, so without this a
/// twelve-face `.ttc` would be opened twelve times.
#[derive(Debug, Default)]
pub struct FileIdentityCache {
    entries: HashMap<PathBuf, FileIdentity>,
}

impl FileIdentityCache {
    pub fn identify(&mut self, path: &Path) -> FileIdentity {
        if let Some(identity) = self.entries.get(path) {
            return identity.clone();
        }
        let identity = file_identity(path);
        self.entries.insert(path.to_owned(), identity.clone());
        identity
    }
}

/// Reads the filesystem's own record for a file, falling back to its path when there is none.
#[must_use]
pub fn file_identity(path: &Path) -> FileIdentity {
    platform_file_identity(path).unwrap_or_else(|| FileIdentity::Path(path.to_owned()))
}

/// Windows keeps a volume serial number and a 64-bit file index per open handle. Both survive a
/// rename and a move within the volume, which is exactly the property a face ID needs.
///
/// A file that cannot be opened, and a filesystem that reports no identifier at all (some network
/// redirectors answer with zeros), fall through to the path so the face still gets an ID.
#[cfg(windows)]
#[allow(unsafe_code)] // Reading a file's identifier requires the Win32 call; nothing else offers it.
fn platform_file_identity(path: &Path) -> Option<FileIdentity> {
    use std::fs::File;
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let file = File::open(path).ok()?;
    let mut information = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();

    // SAFETY: the handle is owned by `file` and stays open across the call, and the out pointer
    // is a live, correctly sized, uniquely borrowed allocation.
    let filled = unsafe {
        GetFileInformationByHandle(file.as_raw_handle().cast(), information.as_mut_ptr())
    };
    if filled == 0 {
        return None;
    }
    // SAFETY: a non-zero return is the call's contract for having written the whole structure.
    let information = unsafe { information.assume_init() };

    let volume = u64::from(information.dwVolumeSerialNumber);
    let index =
        (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
    if volume == 0 && index == 0 {
        return None;
    }
    Some(FileIdentity::Platform { volume, index })
}

/// The device and inode pair, which is the same guarantee Windows gives through its file index.
#[cfg(unix)]
fn platform_file_identity(path: &Path) -> Option<FileIdentity> {
    use std::os::unix::fs::MetadataExt;

    let metadata = std::fs::metadata(path).ok()?;
    Some(FileIdentity::Platform {
        volume: metadata.dev(),
        index: metadata.ino(),
    })
}

#[cfg(not(any(windows, unix)))]
fn platform_file_identity(_path: &Path) -> Option<FileIdentity> {
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKind {
    Family,
    Face,
}

impl IdentityKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Family => "family",
            Self::Face => "face",
        }
    }

    const fn prefix(self) -> &'static str {
        match self {
            Self::Family => FAMILY_ID_PREFIX,
            Self::Face => FACE_ID_PREFIX,
        }
    }
}

/// A stable name for one thing in the catalogue, and the ID that name derives.
///
/// The key is what the ledger stores and looks up; the derived ID is what a session uses when the
/// ledger has nothing to say, which is every scan before a rename or a merge has moved an ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityKey {
    kind: IdentityKind,
    key: String,
    derived_id: String,
}

impl IdentityKey {
    fn finish(kind: IdentityKind, digest: Sha256) -> Self {
        let bytes = digest.finalize();
        let key = hex(&bytes[..ID_HEX_LENGTH / 2]);
        let derived_id = format!("{}{key}", kind.prefix());
        Self {
            kind,
            key,
            derived_id,
        }
    }

    /// Disambiguates a key a scan has already seen.
    ///
    /// Two faces must never share an ID, or the catalogue silently loses one of them. Platform
    /// file identities make that impossible in practice, but the path fallback can repeat, so the
    /// second and later sightings fold an ordinal into the key. Scanning in a fixed order makes
    /// the result the same on every run.
    #[must_use]
    pub fn disambiguated(&self, ordinal: u32) -> Self {
        let mut digest = digest_for(self.kind);
        digest.update(b"duplicate\0");
        digest.update(self.key.as_bytes());
        digest.update(b"\0");
        digest.update(ordinal.to_be_bytes());
        Self::finish(self.kind, digest)
    }

    #[must_use]
    pub const fn kind(&self) -> IdentityKind {
        self.kind
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    #[must_use]
    pub fn derived_id(&self) -> &str {
        &self.derived_id
    }

    #[must_use]
    pub fn into_derived_id(self) -> String {
        self.derived_id
    }
}

/// The identity key for one face: which file, which index inside it, and which font that index
/// currently holds.
#[must_use]
pub fn face_identity_key(
    file: &FileIdentity,
    face_index: u32,
    post_script_name: &str,
) -> IdentityKey {
    let mut digest = digest_for(IdentityKind::Face);
    file.write_into(&mut digest);
    digest.update(b"\0");
    digest.update(face_index.to_be_bytes());
    digest.update(b"\0");
    digest.update(post_script_name.as_bytes());
    IdentityKey::finish(IdentityKind::Face, digest)
}

/// The identity key for one family. A family is named and nothing else, so the key is the name
/// with the differences that never meant a different family removed: surrounding and repeated
/// whitespace, and case.
#[must_use]
pub fn family_identity_key(name: &str) -> IdentityKey {
    let mut digest = digest_for(IdentityKind::Family);
    digest.update(normalize_family_name(name).as_bytes());
    IdentityKey::finish(IdentityKind::Family, digest)
}

/// The grouping key two spellings of one family name must share.
#[must_use]
pub fn normalize_family_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// True when a string has the shape this module produces for the given kind.
#[must_use]
pub fn is_well_formed(kind: IdentityKind, id: &str) -> bool {
    let Some(digest) = id.strip_prefix(kind.prefix()) else {
        return false;
    };
    digest.len() == ID_HEX_LENGTH && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn digest_for(kind: IdentityKind) -> Sha256 {
    let mut digest = Sha256::new();
    digest.update(IDENTITY_VERSION);
    digest.update(b"\0");
    digest.update(kind.as_str().as_bytes());
    digest.update(b"\0");
    digest
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(value, "{byte:02x}");
    }
    value
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        FileIdentity, IdentityKind, face_identity_key, family_identity_key, is_well_formed,
        normalize_family_name,
    };

    fn platform(index: u64) -> FileIdentity {
        FileIdentity::Platform { volume: 7, index }
    }

    #[test]
    fn a_face_keeps_its_id_when_only_its_path_changes() {
        let renamed = face_identity_key(&platform(42), 0, "ArialMT");
        let original = face_identity_key(&platform(42), 0, "ArialMT");

        assert_eq!(renamed, original);
    }

    #[test]
    fn two_copies_of_one_font_stay_separate_faces() {
        let first = face_identity_key(&platform(1), 0, "Inter-Regular");
        let second = face_identity_key(&platform(2), 0, "Inter-Regular");

        assert_ne!(first.derived_id(), second.derived_id());
    }

    #[test]
    fn each_face_of_a_collection_gets_its_own_id() {
        let first = face_identity_key(&platform(9), 0, "MSGothic");
        let second = face_identity_key(&platform(9), 1, "MSPGothic");

        assert_ne!(first.derived_id(), second.derived_id());
    }

    #[test]
    fn a_face_replaced_in_place_by_another_font_is_another_face() {
        let before = face_identity_key(&platform(9), 0, "Inter-Regular");
        let after = face_identity_key(&platform(9), 0, "Geist-Regular");

        assert_ne!(before.derived_id(), after.derived_id());
    }

    #[test]
    fn a_face_id_reveals_neither_a_path_nor_a_name() {
        let key = face_identity_key(
            &FileIdentity::Path(PathBuf::from("C:\\Windows\\Fonts\\arial.ttf")),
            0,
            "ArialMT",
        );

        assert!(is_well_formed(IdentityKind::Face, key.derived_id()));
        assert!(!key.derived_id().contains("Windows"));
        assert!(!key.derived_id().contains("arial"));
    }

    #[test]
    fn family_names_that_differ_only_in_case_or_spacing_are_one_family() {
        assert_eq!(
            family_identity_key("  Source   Serif 4 "),
            family_identity_key("SOURCE SERIF 4")
        );
        assert_eq!(
            normalize_family_name("  Source   Serif 4 "),
            "source serif 4"
        );
    }

    #[test]
    fn different_families_do_not_share_an_id() {
        assert_ne!(
            family_identity_key("Inter").derived_id(),
            family_identity_key("Inter Tight").derived_id()
        );
    }

    #[test]
    fn a_family_id_is_opaque_and_well_formed() {
        let key = family_identity_key("Source Serif 4");

        assert!(is_well_formed(IdentityKind::Family, key.derived_id()));
        assert!(!is_well_formed(IdentityKind::Face, key.derived_id()));
        assert!(!key.derived_id().contains("source"));
    }

    #[test]
    fn a_disambiguated_key_is_stable_and_distinct() {
        let key = face_identity_key(&platform(3), 0, "Inter-Regular");

        assert_eq!(key.disambiguated(1), key.disambiguated(1));
        assert_ne!(key.disambiguated(1).key(), key.key());
        assert_ne!(key.disambiguated(1).key(), key.disambiguated(2).key());
    }

    #[test]
    fn a_family_key_and_a_face_key_never_collide() {
        assert_ne!(
            family_identity_key("Inter").key(),
            face_identity_key(&FileIdentity::Embedded, 0, "Inter").key()
        );
    }

    #[test]
    fn malformed_ids_are_rejected() {
        assert!(!is_well_formed(IdentityKind::Face, "face:"));
        assert!(!is_well_formed(IdentityKind::Face, "arial"));
        assert!(!is_well_formed(
            IdentityKind::Face,
            "face:0123456789abcdef0123456789abcdeZ"
        ));
        assert!(!is_well_formed(IdentityKind::Face, "face:0123456789abcdef"));
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn a_real_file_is_identified_by_its_filesystem_record_not_its_name() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let before = directory.path().join("before.ttf");
        std::fs::write(&before, b"not really a font").expect("the file is written");
        let identity = super::file_identity(&before);

        let after = directory.path().join("after.ttf");
        std::fs::rename(&before, &after).expect("the file is renamed");

        assert!(matches!(identity, FileIdentity::Platform { .. }));
        assert_eq!(identity, super::file_identity(&after));
    }

    #[test]
    fn an_unreadable_file_still_gets_an_identity() {
        let missing = Path::new("nowhere/this-file-does-not-exist.ttf");

        assert_eq!(
            super::file_identity(missing),
            FileIdentity::Path(missing.to_owned())
        );
    }
}
