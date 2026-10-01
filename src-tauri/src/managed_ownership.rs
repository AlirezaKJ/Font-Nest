//! Decides whether `FontNest` may take a font back off this computer.
//!
//! Installing a font writes a row into the ledger, and the ledger is an ordinary `SQLite` file in
//! the user's own application data directory. Anybody who can run `FontNest` can also open that
//! file and change what it says, so a row is a claim rather than a proof. An uninstall that
//! trusted the `installed_path` column would unregister and take away whatever file that column
//! happened to name, which turns a font manager into a way to delete anything the user can write
//! to.
//!
//! So nothing here acts on a path the ledger supplied. The path is derived instead, from two
//! things a tampered row cannot reach: the provider's release manifest, which is compiled into the
//! binary, and the per-user font directory the platform reports. What the ledger stored is then
//! compared against what was derived, and a disagreement ends the operation instead of being
//! reconciled.
//!
//! A proof is only issued when every one of these agrees, and each is checked against the computer
//! as it is now rather than against a record of how it once was:
//!
//! - **Provider identity.** The provider must be one this build knows, and the artifact must exist
//!   in its bundled manifest. That manifest supplies the file name and the content hash, so every
//!   later check starts from the provider's own record.
//! - **Managed file name.** The name is derived from the manifest exactly as installation derived
//!   it, hash segment included, so a file only qualifies under the single name its artifact may be
//!   installed as.
//! - **Canonical containment.** The derived path is canonicalized and must sit directly in the
//!   canonicalized per-user font directory, which settles case differences, short names, and any
//!   attempt to climb out of the directory with `..`.
//! - **Reparse-point status.** A symlink, junction, or any other reparse point is refused outright,
//!   because following one would move the operation to a file somewhere else on the computer. A
//!   file carrying a second hard link is refused for the same reason.
//! - **Content hash.** The bytes are read and hashed, and the digest must equal the manifest's. A
//!   font swapped in place is a different font, and it stops being ours the moment it changes.
//! - **Registry mapping.** The registration name is recomputed from those same bytes, and the
//!   value must still point at this exact file. A value that now names something else belongs to
//!   another font, and removing it would unregister that font instead.
//! - **Protection state.** The file must resolve inside the per-user font directory rather than
//!   anywhere the operating system keeps its own fonts, judged from where the path actually landed
//!   rather than from the directory the environment named.
//!
//! The proof carries the file identity taken from the handle the bytes were read through, so the
//! operation that follows can confirm it is acting on the file that was verified rather than on
//! whatever the path resolves to by then.

use std::fs::{File, Metadata};
use std::io::Read;
use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

use crate::dto::FontOrigin;
use crate::font_identity::{FileIdentity, file_record};
use crate::font_origin;
use crate::font_platform::{
    self, FontPlatformError, InstallableFormat, MANAGED_FILE_PREFIX, managed_file_name,
    validate_font,
};
use crate::local_import::LOCAL_PROVIDER;

/// What the ledger says about one installed font. Every field is untrusted input.
#[derive(Debug, Clone, Copy)]
pub struct UninstallClaim<'a> {
    pub provider: &'a str,
    pub artifact_id: &'a str,
    pub installed_path: &'a str,
    pub registry_value_name: &'a str,
    /// The digest the ledger recorded for these bytes. Untrusted, like every other field, and
    /// used only as a second statement that has to agree with the file itself.
    pub source_hash: &'a str,
}

/// What a provider's own bundled record says one artifact is. It comes from the binary, never from
/// the ledger, which is what makes it usable as evidence against the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderArtifact {
    pub file_name: String,
    /// The digest the provider publishes for these bytes.
    pub content_hash: String,
    pub size_bytes: u64,
}

/// The parts of the computer a managed operation has to ask about or act on. They are injected so
/// the decisions can be tested against a font directory, a registry, and a manifest a test
/// controls, rather than against the machine running the tests.
pub trait ManagedEnvironment {
    /// Where per-user fonts live for the signed-in account.
    fn user_font_directory(&self) -> Option<PathBuf>;
    /// The provider's record of an artifact, or `None` when either is unknown to this build.
    fn provider_artifact(&self, provider: &str, artifact_id: &str) -> Option<ProviderArtifact>;
    /// The file the operating system currently maps a font registration to.
    fn registered_font_path(&self, value_name: &str) -> Option<String>;

    /// Stops the operating system serving a font, leaving the file where it is.
    ///
    /// # Errors
    ///
    /// Returns the platform error when the registration names another file or cannot be removed.
    fn unregister_font(
        &self,
        registry_value_name: &str,
        path: &Path,
    ) -> Result<(), FontPlatformError>;

    /// Puts a font back into service after a removal was undone.
    ///
    /// # Errors
    ///
    /// Returns the platform error when the font cannot be registered again.
    fn register_font(
        &self,
        registry_value_name: &str,
        path: &Path,
    ) -> Result<(), FontPlatformError>;
}

/// Where the expectation about a file comes from, which is the one thing that differs between a
/// font from a provider and a font from this computer.
///
/// A provider publishes a manifest, and `FontNest` carries it in the binary, so the file name,
/// digest and size are all evidence from outside the ledger. A font somebody imported has no such
/// upstream by definition: it came off their own disk. What stands in for the manifest is the name
/// `FontNest` gave the file when it installed it, which carries the first twelve characters of the
/// digest of the bytes it was installed from. A file in the per-user font directory whose own name
/// states the digest its contents hash to is making a claim that only `FontNest` writes and that
/// nothing else in that directory can accidentally satisfy.
///
/// This is weaker than a manifest and deliberately so, because nothing stronger exists for a file
/// that came from the person's own computer. What it still guarantees is what matters: the only
/// files that can ever be proven are ones sitting in the per-user font directory, under a name
/// `FontNest` derived, holding bytes that hash to what that name says, registered under a value
/// recomputed from those same bytes. A ledger somebody edited can at most point the operation at a
/// different font of exactly that description, which is a font the application would remove on
/// request anyway.
enum Expectation {
    /// The provider's own record, compiled into this build.
    Published(ProviderArtifact),
    /// The file's own managed name, and the digest prefix it carries.
    SelfCertified {
        file_name: String,
        digest_prefix: String,
    },
}

impl Expectation {
    fn file_name(&self) -> &str {
        match self {
            Self::Published(artifact) => &artifact.file_name,
            Self::SelfCertified { file_name, .. } => file_name,
        }
    }

    /// How many bytes may be read before the digest is checked. A manifest states the size; a
    /// self-certified file is bounded by what this platform will install at all.
    fn read_limit(&self) -> u64 {
        match self {
            Self::Published(artifact) => artifact.size_bytes,
            Self::SelfCertified { .. } => MAX_SELF_CERTIFIED_BYTES,
        }
    }

    /// Whether the bytes that were read are the bytes this file is supposed to hold.
    fn accepts(&self, digest: &str, read_bytes: u64) -> bool {
        match self {
            Self::Published(artifact) => {
                read_bytes == artifact.size_bytes
                    && digest.eq_ignore_ascii_case(&artifact.content_hash)
            }
            Self::SelfCertified { digest_prefix, .. } => digest
                .get(..digest_prefix.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(digest_prefix)),
        }
    }
}

/// Proof that one specific file on this computer is a font `FontNest` installed and may remove.
///
/// Every field was derived or measured during authorization. Nothing in it came from the ledger,
/// so an operation built on a proof is an operation the ledger cannot redirect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenOwnership {
    /// The canonical path of the verified file.
    pub path: PathBuf,
    /// The registration name recomputed from the file's own bytes.
    pub registry_value_name: String,
    /// The font's full name, so the interface can say plainly what is being removed.
    pub display_name: String,
    /// The digest of the bytes that were read.
    pub content_hash: String,
    /// The filesystem's record for the handle those bytes were read through.
    pub identity: FileIdentity,
}

/// Why `FontNest` will not treat a font as its own to remove.
///
/// Each variant names the check that stopped, because the difference matters to whoever reads it:
/// a font somebody replaced is a different situation from a ledger somebody edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OwnershipRefusal {
    #[error("the font was not installed from a source this version of FontNest knows")]
    UnknownProvider,
    #[error("the per-user font directory is unavailable")]
    FontDirectoryUnavailable,
    #[error("the ledger names a file the provider record does not")]
    LedgerPathMismatch,
    #[error("the font is no longer where FontNest installed it")]
    Missing,
    #[error("the path is a link rather than the installed file")]
    Redirected,
    #[error("the file is outside the per-user font directory")]
    OutsideManagedRoot,
    #[error("the file no longer holds the bytes FontNest installed")]
    ContentMismatch,
    #[error("the font registration no longer points at this file")]
    RegistryMismatch,
    #[error("the font is protected by the operating system")]
    Protected,
    #[error("the font file could not be read")]
    Unreadable,
}

/// Most bytes a self-certified font may hold. A file from this computer states no size anywhere
/// outside itself, so the bound is what this platform is willing to install at all.
const MAX_SELF_CERTIFIED_BYTES: u64 = 64 * 1024 * 1024;

/// How many characters of the digest a managed file name carries.
const DIGEST_PREFIX_LENGTH: usize = 12;

/// Works out what this file is supposed to be, from the strongest evidence available for it.
fn expectation_for(
    claim: &UninstallClaim<'_>,
    environment: &impl ManagedEnvironment,
) -> Result<Expectation, OwnershipRefusal> {
    if let Some(artifact) = environment.provider_artifact(claim.provider, claim.artifact_id) {
        // Derived from the provider's record, exactly as installation derived it. A ledger row
        // cannot widen this to a second name, so there is only ever one file an artifact may be
        // removed as.
        let file_name = managed_file_name(&artifact.file_name, &artifact.content_hash)
            .map_err(|_| OwnershipRefusal::UnknownProvider)?;
        return Ok(Expectation::Published(ProviderArtifact {
            file_name,
            ..artifact
        }));
    }

    if claim.provider != LOCAL_PROVIDER {
        return Err(OwnershipRefusal::UnknownProvider);
    }

    // Only the file name is taken from the claim, and only after it proves to be one FontNest
    // writes. The directory it is looked for in comes from the platform, never from the ledger.
    let file_name = Path::new(claim.installed_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(OwnershipRefusal::LedgerPathMismatch)?;
    if !font_platform::is_managed_file_name(file_name) {
        return Err(OwnershipRefusal::LedgerPathMismatch);
    }
    let digest_prefix = file_name
        .strip_prefix(MANAGED_FILE_PREFIX)
        .and_then(|rest| rest.get(..DIGEST_PREFIX_LENGTH))
        .filter(|prefix| prefix.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or(OwnershipRefusal::LedgerPathMismatch)?
        .to_owned();

    Ok(Expectation::SelfCertified {
        file_name: file_name.to_owned(),
        digest_prefix,
    })
}

/// Decides whether one claimed installation may be uninstalled.
///
/// # Errors
///
/// Returns the [`OwnershipRefusal`] naming the first check that failed. A refusal means nothing on
/// the computer was touched: authorization only reads.
pub fn authorize_uninstall(
    claim: &UninstallClaim<'_>,
    environment: &impl ManagedEnvironment,
) -> Result<ProvenOwnership, OwnershipRefusal> {
    let expectation = expectation_for(claim, environment)?;
    let file_name = expectation.file_name().to_owned();
    let directory = environment
        .user_font_directory()
        .ok_or(OwnershipRefusal::FontDirectoryUnavailable)?;
    let expected_path = directory.join(&file_name);

    // The ledger gets to be checked, not obeyed. A row naming anything else is evidence of
    // tampering, or of a database this build must not act on, and either way the answer is no.
    if !same_path(Path::new(claim.installed_path), &expected_path) {
        return Err(OwnershipRefusal::LedgerPathMismatch);
    }

    let metadata =
        std::fs::symlink_metadata(&expected_path).map_err(|_| OwnershipRefusal::Missing)?;
    if is_reparse_point(&metadata) {
        return Err(OwnershipRefusal::Redirected);
    }
    if !metadata.is_file() {
        return Err(OwnershipRefusal::Missing);
    }
    if metadata.len() > expectation.read_limit() {
        return Err(OwnershipRefusal::ContentMismatch);
    }

    // Canonicalizing both sides is what makes containment mean containment: it settles case, short
    // names, and anything that survived the join.
    let path = std::fs::canonicalize(&expected_path).map_err(|_| OwnershipRefusal::Missing)?;
    let canonical_directory = std::fs::canonicalize(&directory)
        .map_err(|_| OwnershipRefusal::FontDirectoryUnavailable)?;
    if path.parent() != Some(canonical_directory.as_path())
        || path.file_name().and_then(|name| name.to_str()) != Some(file_name.as_str())
    {
        return Err(OwnershipRefusal::OutsideManagedRoot);
    }

    let mut file = File::open(&path).map_err(|_| OwnershipRefusal::Unreadable)?;
    let record = file_record(&file).ok_or(OwnershipRefusal::Unreadable)?;
    // A managed font is written once and never linked. A second name for it is somebody else's
    // doing, and taking this one away would leave that one holding the file.
    if record.links.is_some_and(|links| links > 1) {
        return Err(OwnershipRefusal::Redirected);
    }

    // Bounded before anything is read, so a file that grew between the check and the read cannot
    // pull an unbounded amount into memory.
    let limit = expectation.read_limit();
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or_default());
    file.by_ref()
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| OwnershipRefusal::Unreadable)?;
    let digest = content_hash(&bytes);
    if !expectation.accepts(&digest, u64::try_from(bytes.len()).unwrap_or(u64::MAX)) {
        return Err(OwnershipRefusal::ContentMismatch);
    }
    // The ledger is not evidence, but it is a second statement, and a font FontNest installed has
    // no reason for its recorded digest to disagree with the bytes that are there.
    if !claim.source_hash.is_empty() && !digest.eq_ignore_ascii_case(claim.source_hash) {
        return Err(OwnershipRefusal::ContentMismatch);
    }

    // The registration name is recomputed from the bytes rather than read out of the ledger, so
    // the value about to be removed is the one this font's own metadata produces.
    let font = validate_font(&bytes).map_err(|_| OwnershipRefusal::ContentMismatch)?;
    // Derived through the same table installation used, so a format whose registration Windows
    // names differently cannot end up unprovable here.
    let format =
        InstallableFormat::of_file_name(&file_name).ok_or(OwnershipRefusal::LedgerPathMismatch)?;
    let registry_value_name = format.registry_value_name(&font.full_name);
    if claim.registry_value_name != registry_value_name {
        return Err(OwnershipRefusal::RegistryMismatch);
    }
    // A value that names another file belongs to another font, and deleting it would unregister
    // that font instead of this one. A value that is not there at all is a different situation:
    // there is nothing to unregister, and the font is still ours by everything that does not
    // depend on the registry, which is the name it was installed under, where it sits, and the
    // bytes it holds. Refusing that would strand a file `FontNest` put on the computer and would
    // never take back, which is the opposite of what this module is for.
    if let Some(registered) = environment.registered_font_path(&registry_value_name)
        && !registers_the_same_file(&registered, &path)
    {
        return Err(OwnershipRefusal::RegistryMismatch);
    }

    // Judged from where the file actually resolved. If the environment named a font directory
    // somewhere the operating system keeps its own fonts, this is the check that notices.
    if font_origin::classify(&path, &font.family_name) != FontOrigin::UserInstalled {
        return Err(OwnershipRefusal::Protected);
    }

    Ok(ProvenOwnership {
        path,
        registry_value_name,
        display_name: font.full_name,
        content_hash: digest,
        identity: record.identity,
    })
}

/// Whether an open file is still the one a proof was issued for.
///
/// Authorization and removal are separate moments, and a path can be pointed at something else in
/// between. Comparing the record of the file a handle now holds against the record taken from the
/// handle the bytes were read through closes that window: a swapped file has a different record
/// even when it has the same name.
#[must_use]
pub fn still_the_proven_file(proof: &ProvenOwnership, file: &File) -> bool {
    file_record(file).is_some_and(|record| record.identity == proof.identity)
}

/// The digest scheme the bundled providers publish their artifacts under: the Git blob hash, which
/// a repository snapshot can be verified against without a second index.
#[must_use]
pub fn content_hash(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()).as_bytes());
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut value = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(value, "{byte:02x}");
    }
    value
}

/// Compares two paths the way the platform compares them, before either has to exist.
///
/// Windows paths differ in case without differing at all, so a row written by an earlier version,
/// or by somebody editing the file, is judged the way the filesystem would judge it.
fn same_path(left: &Path, right: &Path) -> bool {
    names_the_same_path(
        &left.as_os_str().to_string_lossy(),
        &right.as_os_str().to_string_lossy(),
    )
}

/// Whether two recorded paths name the same file, judged before either has to exist.
///
/// Shared with the ledger, which compares the paths it stored against the paths a journal holds.
/// Both have to agree about what counts as the same path, or one of them decides a font is not the
/// font it is.
#[must_use]
pub fn names_the_same_path(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        without_verbatim_prefix(left).eq_ignore_ascii_case(without_verbatim_prefix(right))
    } else {
        left == right
    }
}

/// Windows spells the same path two ways, and both reach this comparison.
///
/// A path built by joining is `C:\...`, and one that has been through `canonicalize`
/// comes back extended-length as `\\?\C:\...`. They name the same file, and
/// ledger rows hold both: an installation writes the joined form, while a row written from a
/// verified path, or by an older build, holds the canonical one. Treating the prefix as a
/// difference made such a row name a file nothing could derive, so the font could never be
/// removed.
///
/// Only the disk form is unwrapped. A verbatim UNC path is a different shape again, and the
/// per-user font directory is never one.
fn without_verbatim_prefix(path: &str) -> &str {
    if path.starts_with(r"\\?\UNC\") {
        return path;
    }
    path.strip_prefix(r"\\?\").unwrap_or(path)
}

/// Whether a registry value names the file a proof is about.
///
/// The value holds the path as it was written at install time, so it is canonicalized before the
/// comparison. A value that cannot be resolved to a real file names nothing this may act on.
#[must_use]
pub fn registers_the_same_file(registered: &str, path: &Path) -> bool {
    std::fs::canonicalize(registered).is_ok_and(|resolved| resolved == path)
}

/// Windows sets one attribute for every kind of reparse point, which is stricter than asking
/// whether the file is a symbolic link: junctions, mount points, and the tags other software
/// invents all answer here.
#[cfg(windows)]
fn is_reparse_point(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink()
}

/// The computer this build is running on.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemEnvironment;

impl ManagedEnvironment for SystemEnvironment {
    fn user_font_directory(&self) -> Option<PathBuf> {
        font_platform::user_font_directory().ok()
    }

    fn provider_artifact(&self, provider: &str, artifact_id: &str) -> Option<ProviderArtifact> {
        crate::google_fonts::manifest_artifact(provider, artifact_id)
    }

    fn registered_font_path(&self, value_name: &str) -> Option<String> {
        font_platform::registered_user_font_path(value_name)
    }

    fn unregister_font(
        &self,
        registry_value_name: &str,
        path: &Path,
    ) -> Result<(), FontPlatformError> {
        font_platform::unregister_user_font(registry_value_name, path)
    }

    fn register_font(
        &self,
        registry_value_name: &str,
        path: &Path,
    ) -> Result<(), FontPlatformError> {
        font_platform::register_user_font(registry_value_name, path)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use super::{
        ManagedEnvironment, OwnershipRefusal, ProviderArtifact, UninstallClaim,
        authorize_uninstall, content_hash,
    };
    use crate::font_platform::{FontPlatformError, managed_file_name, validate_font};
    use crate::local_import::LOCAL_PROVIDER;

    /// Authorization runs on real bytes, so the tests do too: a font Windows ships stands in for a
    /// provider artifact, which keeps fonts the repository would have to license out of it.
    const SYSTEM_FONT: &str = r"C:\Windows\Fonts\arial.ttf";
    const ARTIFACT: &str = "gf:test:regular";
    const PROVIDER: &str = "google-fonts";

    struct FakeEnvironment {
        font_directory: PathBuf,
        artifacts: HashMap<(String, String), ProviderArtifact>,
        registry: HashMap<String, String>,
    }

    impl ManagedEnvironment for FakeEnvironment {
        fn user_font_directory(&self) -> Option<PathBuf> {
            Some(self.font_directory.clone())
        }

        fn provider_artifact(&self, provider: &str, artifact_id: &str) -> Option<ProviderArtifact> {
            self.artifacts
                .get(&(provider.to_owned(), artifact_id.to_owned()))
                .cloned()
        }

        fn registered_font_path(&self, value_name: &str) -> Option<String> {
            self.registry.get(value_name).cloned()
        }

        fn unregister_font(
            &self,
            _value_name: &str,
            _path: &Path,
        ) -> Result<(), FontPlatformError> {
            unreachable!("authorization never changes a registration")
        }

        fn register_font(&self, _value_name: &str, _path: &Path) -> Result<(), FontPlatformError> {
            unreachable!("authorization never changes a registration")
        }
    }

    struct Installed {
        environment: FakeEnvironment,
        path: PathBuf,
        registry_value_name: String,
        bytes: Vec<u8>,
        source_hash: String,
    }

    impl Installed {
        fn claim<'a>(&'a self, installed_path: &'a str) -> UninstallClaim<'a> {
            UninstallClaim {
                provider: PROVIDER,
                artifact_id: ARTIFACT,
                installed_path,
                registry_value_name: &self.registry_value_name,
                source_hash: &self.source_hash,
            }
        }

        fn recorded_path(&self) -> String {
            self.path.to_string_lossy().into_owned()
        }
    }

    /// Recreates a per-user installation inside a temporary directory: the file name installation
    /// would have derived, in a directory shaped like the real managed root, so the containment
    /// and protection checks run against a path that looks the way the real one does.
    fn install(root: &Path) -> Option<Installed> {
        let bytes = std::fs::read(SYSTEM_FONT).ok()?; // Not every machine has this font.
        let font = validate_font(&bytes).expect("a system font parses");
        let font_directory = root
            .join("AppData")
            .join("Local")
            .join("Microsoft")
            .join("Windows")
            .join("Fonts");
        std::fs::create_dir_all(&font_directory).expect("the user font directory");

        let hash = content_hash(&bytes);
        let file_name = managed_file_name("Test-Regular.ttf", &hash).expect("a managed file name");
        let path = font_directory.join(&file_name);
        std::fs::write(&path, &bytes).expect("the installed font");

        let registry_value_name = format!("{} (TrueType)", font.full_name);
        let mut artifacts = HashMap::new();
        artifacts.insert(
            (PROVIDER.to_owned(), ARTIFACT.to_owned()),
            ProviderArtifact {
                file_name: "Test-Regular.ttf".to_owned(),
                content_hash: hash,
                size_bytes: u64::try_from(bytes.len()).expect("a font that fits"),
            },
        );
        let mut registry = HashMap::new();
        registry.insert(
            registry_value_name.clone(),
            path.to_string_lossy().into_owned(),
        );

        Some(Installed {
            environment: FakeEnvironment {
                font_directory,
                artifacts,
                registry,
            },
            path,
            registry_value_name,
            source_hash: content_hash(&bytes),
            bytes,
        })
    }

    /// Sets up one installed font, or leaves the test unrun on a machine without the system font
    /// that stands in for an artifact.
    macro_rules! installed {
        ($temp:ident) => {{
            let $temp = tempfile::tempdir().expect("a temporary directory");
            let Some(installed) = install($temp.path()) else {
                return;
            };
            ($temp, installed)
        }};
    }

    #[test]
    fn a_font_fontnest_installed_is_authorized() {
        let (_temp, installed) = installed!(temp);
        let path = installed.recorded_path();

        let proof = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect("an installed font is FontNest's to remove");

        assert_eq!(
            proof.path,
            std::fs::canonicalize(&installed.path).expect("the canonical path")
        );
        assert_eq!(proof.registry_value_name, installed.registry_value_name);
        assert_eq!(proof.content_hash, content_hash(&installed.bytes));
    }

    // Windows writes the same path two ways. A row holding the extended-length form, which is what
    // `canonicalize` returns and what older builds recorded, names exactly the file the derivation
    // produces, so it has to be judged the same. Treating the prefix as a difference left those
    // fonts impossible to remove.
    #[test]
    fn a_ledger_row_written_the_extended_length_way_names_the_same_file() {
        let (_temp, installed) = installed!(temp);
        let verbatim = format!(r"\\?\{}", installed.recorded_path());

        let proof = authorize_uninstall(&installed.claim(&verbatim), &installed.environment)
            .expect("the same file spelled the other way is still the same file");

        // The proof carries the resolved path, which is the extended-length spelling on Windows,
        // so the two are compared the way the filesystem compares them.
        assert_eq!(
            proof.path,
            std::fs::canonicalize(&installed.path).expect("the installed file resolves")
        );
    }

    #[test]
    fn a_ledger_row_pointing_at_another_file_is_refused() {
        let (temp, installed) = installed!(temp);
        let elsewhere = temp.path().join("important.ttf");
        std::fs::write(&elsewhere, &installed.bytes).expect("a file that is not ours");
        let path = elsewhere.to_string_lossy().into_owned();

        let refusal = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect_err("an edited ledger must not redirect a removal");

        assert_eq!(refusal, OwnershipRefusal::LedgerPathMismatch);
        assert!(elsewhere.exists(), "authorization only ever reads");
        assert!(installed.path.exists());
    }

    #[test]
    fn a_ledger_row_naming_a_system_font_is_refused() {
        let (_temp, installed) = installed!(temp);

        let refusal = authorize_uninstall(&installed.claim(SYSTEM_FONT), &installed.environment)
            .expect_err("a ledger must not be able to name a font Windows ships");

        assert_eq!(refusal, OwnershipRefusal::LedgerPathMismatch);
        assert!(Path::new(SYSTEM_FONT).exists());
    }

    #[test]
    fn an_artifact_this_build_does_not_know_is_refused() {
        let (_temp, installed) = installed!(temp);
        let path = installed.recorded_path();
        let claim = UninstallClaim {
            artifact_id: "gf:invented:regular",
            ..installed.claim(&path)
        };

        let refusal = authorize_uninstall(&claim, &installed.environment)
            .expect_err("an artifact with no provider record proves nothing");

        assert_eq!(refusal, OwnershipRefusal::UnknownProvider);
    }

    #[test]
    fn a_provider_this_build_does_not_know_is_refused() {
        let (_temp, installed) = installed!(temp);
        let path = installed.recorded_path();
        let claim = UninstallClaim {
            provider: "some-other-provider",
            ..installed.claim(&path)
        };

        let refusal = authorize_uninstall(&claim, &installed.environment)
            .expect_err("only a provider this build bundles can vouch for an artifact");

        assert_eq!(refusal, OwnershipRefusal::UnknownProvider);
    }

    #[test]
    fn a_font_replaced_in_place_is_no_longer_ours() {
        let (_temp, installed) = installed!(temp);
        let mut replaced = installed.bytes.clone();
        let last = replaced.len() - 1;
        replaced[last] ^= 0xff;
        std::fs::write(&installed.path, &replaced).expect("the replaced font");
        let path = installed.recorded_path();

        let refusal = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect_err("bytes FontNest did not install are not FontNest's to remove");

        assert_eq!(refusal, OwnershipRefusal::ContentMismatch);
    }

    #[test]
    fn a_font_that_grew_since_it_was_installed_is_refused() {
        let (_temp, installed) = installed!(temp);
        let mut grown = installed.bytes.clone();
        grown.extend_from_slice(&[0_u8; 4096]);
        std::fs::write(&installed.path, &grown).expect("the grown font");
        let path = installed.recorded_path();

        let refusal = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect_err("a file that is not the installed size is not the installed file");

        assert_eq!(refusal, OwnershipRefusal::ContentMismatch);
    }

    #[test]
    fn a_registration_that_moved_to_another_file_is_refused() {
        let (temp, mut installed) = installed!(temp);
        let other = temp.path().join("other.ttf");
        std::fs::write(&other, &installed.bytes).expect("another font");
        installed.environment.registry.insert(
            installed.registry_value_name.clone(),
            other.to_string_lossy().into_owned(),
        );
        let path = installed.recorded_path();

        let refusal = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect_err("a registration naming another file belongs to another font");

        assert_eq!(refusal, OwnershipRefusal::RegistryMismatch);
    }

    // A font FontNest installed can lose its registration: Windows' own font settings can take one
    // out of service and leave the file behind. Nothing is then left to unregister, and everything
    // that proves the file is ours is still true, so refusing would strand a file FontNest placed
    // with no way to take it back.
    #[test]
    fn a_font_whose_registration_is_gone_can_still_be_taken_back() {
        let (_temp, mut installed) = installed!(temp);
        installed.environment.registry.clear();
        let path = installed.recorded_path();

        let proof = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect("a file FontNest placed stays FontNest's to take back");

        assert_eq!(proof.registry_value_name, installed.registry_value_name);
    }

    #[test]
    fn a_registration_name_the_font_does_not_produce_is_refused() {
        let (_temp, installed) = installed!(temp);
        let path = installed.recorded_path();
        let claim = UninstallClaim {
            registry_value_name: "Something Else (TrueType)",
            ..installed.claim(&path)
        };

        let refusal = authorize_uninstall(&claim, &installed.environment)
            .expect_err("the registration name is recomputed from the bytes, not taken on trust");

        assert_eq!(refusal, OwnershipRefusal::RegistryMismatch);
    }

    #[test]
    fn a_font_directory_outside_the_managed_root_is_refused() {
        let (temp, mut installed) = installed!(temp);
        // The environment now names a directory that is not the per-user font directory, which is
        // what a tampered LOCALAPPDATA looks like from here.
        let elsewhere = temp.path().join("Fonts");
        std::fs::create_dir_all(&elsewhere).expect("another directory");
        let moved = elsewhere.join(installed.path.file_name().expect("the managed file name"));
        std::fs::copy(&installed.path, &moved).expect("the copied font");
        installed.environment.font_directory = elsewhere;
        installed.environment.registry.insert(
            installed.registry_value_name.clone(),
            moved.to_string_lossy().into_owned(),
        );
        let path = moved.to_string_lossy().into_owned();

        let refusal = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect_err("only the per-user font directory holds fonts FontNest may remove");

        assert_eq!(refusal, OwnershipRefusal::Protected);
        assert!(moved.exists());
    }

    #[test]
    fn a_missing_font_is_refused_rather_than_treated_as_already_removed() {
        let (_temp, installed) = installed!(temp);
        let path = installed.recorded_path();
        std::fs::remove_file(&installed.path).expect("the font is gone");

        let refusal = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect_err("a font that is not there cannot be proven ours");

        assert_eq!(refusal, OwnershipRefusal::Missing);
    }

    #[test]
    fn a_second_name_for_the_installed_file_is_refused() {
        let (temp, installed) = installed!(temp);
        let link = temp.path().join("second-name.ttf");
        if std::fs::hard_link(&installed.path, &link).is_err() {
            return; // Some filesystems refuse hard links, which is the safe answer anyway.
        }
        let path = installed.recorded_path();

        let refusal = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect_err("removing one name would leave the other holding the file");

        assert_eq!(refusal, OwnershipRefusal::Redirected);
    }

    #[test]
    fn a_link_standing_in_for_the_installed_file_is_refused() {
        let (temp, installed) = installed!(temp);
        let target = temp.path().join("target.ttf");
        std::fs::write(&target, &installed.bytes).expect("the link target");
        std::fs::remove_file(&installed.path).expect("the real font makes way for a link");
        if !link_file(&target, &installed.path) {
            return; // Creating a symlink needs a privilege Windows does not grant by default.
        }
        let path = installed.recorded_path();

        let refusal = authorize_uninstall(&installed.claim(&path), &installed.environment)
            .expect_err("following a link would move the removal to another file");

        assert_eq!(refusal, OwnershipRefusal::Redirected);
        assert!(target.exists());
    }

    #[cfg(windows)]
    fn link_file(target: &Path, link: &Path) -> bool {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }

    #[cfg(unix)]
    fn link_file(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    #[cfg(not(any(windows, unix)))]
    fn link_file(_target: &Path, _link: &Path) -> bool {
        false
    }

    /// A font imported from this computer, which has no manifest behind it. The environment knows
    /// nothing about it on purpose: everything the proof needs has to come from the file itself.
    fn import(root: &Path) -> Option<Installed> {
        let mut installed = install(root)?;
        installed.environment.artifacts.clear();
        Some(installed)
    }

    fn local_claim<'a>(installed: &'a Installed, path: &'a str) -> UninstallClaim<'a> {
        UninstallClaim {
            provider: LOCAL_PROVIDER,
            artifact_id: &installed.source_hash,
            installed_path: path,
            registry_value_name: &installed.registry_value_name,
            source_hash: &installed.source_hash,
        }
    }

    // A font from this computer is proven by its own name and its own bytes, because there is no
    // manifest to prove it against.
    #[test]
    fn an_imported_font_is_proven_by_the_name_fontnest_gave_it() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let Some(installed) = import(root.path()) else {
            return;
        };
        let path = installed.recorded_path();

        let proof = authorize_uninstall(&local_claim(&installed, &path), &installed.environment)
            .expect("a font FontNest installed from a file is still one it installed");

        assert_eq!(proof.content_hash, installed.source_hash);
        assert_eq!(proof.registry_value_name, installed.registry_value_name);
    }

    #[test]
    fn an_imported_font_someone_replaced_is_refused() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let Some(installed) = import(root.path()) else {
            return;
        };
        let mut swapped = installed.bytes.clone();
        swapped.extend_from_slice(b"not the bytes this name claims");
        std::fs::write(&installed.path, &swapped).expect("the font is replaced");
        let path = installed.recorded_path();

        let refusal = authorize_uninstall(&local_claim(&installed, &path), &installed.environment)
            .expect_err("a file that no longer hashes to its own name is not ours");

        assert_eq!(refusal, OwnershipRefusal::ContentMismatch);
    }

    // The file name is the only thing taken from the ledger, and only after it proves to be one
    // FontNest writes. A row naming anything else names a file this never touches.
    #[test]
    fn a_row_naming_a_file_fontnest_never_named_is_refused() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let Some(installed) = import(root.path()) else {
            return;
        };
        let victim = installed
            .path
            .parent()
            .expect("a font directory")
            .join("SomebodyElses.ttf");
        std::fs::write(&victim, &installed.bytes).expect("a font that is not ours");
        let path = victim.to_string_lossy().into_owned();

        let refusal = authorize_uninstall(&local_claim(&installed, &path), &installed.environment)
            .expect_err("only files FontNest named can be proven");

        assert_eq!(refusal, OwnershipRefusal::LedgerPathMismatch);
        assert!(victim.exists(), "and the file is still there");
    }

    // The ledger is not evidence, but it is a second statement, and both have to agree.
    #[test]
    fn an_imported_font_whose_recorded_digest_disagrees_is_refused() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let Some(installed) = import(root.path()) else {
            return;
        };
        let path = installed.recorded_path();
        let tampered = UninstallClaim {
            source_hash: "0000000000000000000000000000000000000000",
            ..local_claim(&installed, &path)
        };

        let refusal = authorize_uninstall(&tampered, &installed.environment)
            .expect_err("a recorded digest that disagrees with the file ends the operation");

        assert_eq!(refusal, OwnershipRefusal::ContentMismatch);
    }

    #[test]
    fn a_provider_this_build_does_not_know_is_still_refused() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let Some(installed) = import(root.path()) else {
            return;
        };
        let path = installed.recorded_path();
        let unknown = UninstallClaim {
            provider: "some-other-shop",
            ..local_claim(&installed, &path)
        };

        let refusal = authorize_uninstall(&unknown, &installed.environment)
            .expect_err("only providers this build knows, and this computer, are proven");

        assert_eq!(refusal, OwnershipRefusal::UnknownProvider);
    }
}
