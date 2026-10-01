use std::path::{Path, PathBuf};

use ttf_parser::{Face, name_id};

/// Every file `FontNest` writes into the user font directory starts with this. It is the first
/// thing checked before anything is deleted, so a font the user installed themselves is never
/// mistaken for one of ours.
/// The prefix every file `FontNest` installs carries, followed by the first characters of the
/// digest of the bytes it was installed from. The ownership proof reads that digest back out of
/// the name, so this is part of the evidence rather than decoration.
pub const MANAGED_FILE_PREFIX: &str = "FontNest-";

/// Where Windows records the fonts registered for the signed-in account only.
#[cfg(windows)]
const USER_FONTS_REGISTRY_KEY: &str = r"Software\Microsoft\Windows NT\CurrentVersion\Fonts";

#[derive(Debug, Clone)]
pub struct ValidatedFontMetadata {
    pub family_name: String,
    pub full_name: String,
}

#[derive(Debug, Clone)]
pub struct PlatformInstallation {
    pub installed_path: PathBuf,
    pub registry_value_name: String,
    pub display_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FontPlatformError {
    #[error("the font file is not a supported desktop font")]
    InvalidFont,
    #[error("the font contains invalid naming metadata")]
    InvalidMetadata,
    #[error("the current-user font directory is unavailable")]
    UserFontDirectoryUnavailable,
    #[error("the target font file already exists outside the FontNest ledger")]
    TargetConflict,
    #[error("the Windows font registry already contains a different file for this font")]
    RegistryConflict,
    #[error("the operating system rejected the font registration")]
    RegistrationFailed,
    #[cfg(not(windows))]
    #[error("font installation is currently supported on Windows only")]
    UnsupportedPlatform,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn validate_font(bytes: &[u8]) -> Result<ValidatedFontMetadata, FontPlatformError> {
    let face = Face::parse(bytes, 0).map_err(|_| FontPlatformError::InvalidFont)?;
    if face.number_of_glyphs() == 0 {
        return Err(FontPlatformError::InvalidFont);
    }

    let family_name = unicode_name(&face, name_id::TYPOGRAPHIC_FAMILY)
        .or_else(|| unicode_name(&face, name_id::FAMILY))
        .ok_or(FontPlatformError::InvalidMetadata)?;
    let full_name = unicode_name(&face, name_id::FULL_NAME).unwrap_or_else(|| family_name.clone());
    let post_script_name =
        unicode_name(&face, name_id::POST_SCRIPT_NAME).ok_or(FontPlatformError::InvalidMetadata)?;

    if [
        family_name.as_str(),
        full_name.as_str(),
        post_script_name.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty() || value.chars().count() > 255)
    {
        return Err(FontPlatformError::InvalidMetadata);
    }

    Ok(ValidatedFontMetadata {
        family_name,
        full_name,
    })
}

/// How much of the request the file manager could honour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevealOutcome {
    /// The file manager opened with the font file itself highlighted.
    Selected,
    /// Only the containing folder could be opened. See [`reveal_in_file_manager`].
    FolderOnly,
}

/// Opens the platform file manager with `path` selected.
///
/// The path always originates from the catalogue scan, never from the web view, so no
/// caller-supplied string reaches a shell. Every argument is passed as a separate process
/// argument rather than through a shell command line.
///
/// Selecting the file is not always possible. Windows presents `C:\Windows\Fonts` as the
/// Fonts control panel rather than a directory, and the files inside it are not addressable
/// as shell items at all, so system fonts fall back to opening the folder.
pub fn reveal_in_file_manager(path: &Path) -> Result<RevealOutcome, FontPlatformError> {
    if !path.is_file() {
        return Err(FontPlatformError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "the font file is no longer on disk",
        )));
    }

    #[cfg(windows)]
    {
        reveal_on_windows(path)
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map(|_| RevealOutcome::Selected)
            .map_err(FontPlatformError::Io)
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        open_containing_folder(path).map(|()| RevealOutcome::FolderOnly)
    }
}

/// Reveals `path` through the shell rather than through `explorer.exe /select,`.
///
/// `explorer.exe` parses its own command line and mangles `/select,` arguments containing
/// spaces or commas; worse, when the argument names something the shell cannot resolve it
/// silently opens the user's default folder instead of reporting an error. Going through
/// `SHOpenFolderAndSelectItems` avoids both problems and lets an unresolvable item be
/// detected up front, which is what the system font directory produces.
#[cfg(windows)]
#[allow(unsafe_code)]
fn reveal_on_windows(path: &Path) -> Result<RevealOutcome, FontPlatformError> {
    use windows_sys::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize,
    };
    use windows_sys::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems};

    let wide = wide_path(path);

    // windows-sys types the COINIT constants as i32 but the parameter as u32. The constant
    // is a small positive flag, so widening it loses nothing.
    #[allow(clippy::cast_sign_loss)]
    let apartment_threaded = COINIT_APARTMENTTHREADED as u32;

    // The shell APIs below need an initialized apartment on the calling thread, and without
    // one ILCreateFromPathW skips the namespace lookup entirely and returns a PIDL for paths
    // that have no shell item. This runs on a dedicated blocking worker, so the apartment is
    // ours to manage.
    // SAFETY: CoUninitialize below is paired with a successful CoInitializeEx only.
    let initialized = unsafe { CoInitializeEx(std::ptr::null(), apartment_threaded) } >= 0;

    // SAFETY: `wide` is a live, null-terminated UTF-16 buffer. ILCreateFromPathW returns
    // either null or a PIDL owned by the caller, which is freed on every path below before
    // the buffer goes out of scope.
    let item = unsafe { ILCreateFromPathW(wide.as_ptr()) };

    let outcome = if item.is_null() {
        // The path is inside a shell namespace folder that does not expose its files, so
        // no item PIDL exists to select. The folder itself still resolves.
        open_containing_folder(path).map(|()| RevealOutcome::FolderOnly)
    } else {
        // SAFETY: `item` is a valid PIDL. Passing it as the folder with a count of zero is
        // the documented way to ask the shell to open its parent and select it.
        let result = unsafe { SHOpenFolderAndSelectItems(item, 0, std::ptr::null(), 0) };
        // SAFETY: `item` came from ILCreateFromPathW and is freed exactly once.
        unsafe { ILFree(item) };

        if result >= 0 {
            Ok(RevealOutcome::Selected)
        } else {
            open_containing_folder(path).map(|()| RevealOutcome::FolderOnly)
        }
    };

    if initialized {
        // SAFETY: Balances the CoInitializeEx above on the same thread.
        unsafe { CoUninitialize() };
    }

    outcome
}

/// Opens the directory holding `path`, without selecting anything inside it.
fn open_containing_folder(path: &Path) -> Result<(), FontPlatformError> {
    let directory = path.parent().unwrap_or(path);

    #[cfg(windows)]
    let mut command = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");

    // explorer.exe exits with a non-zero status even when the window opens, so the status
    // is deliberately not checked; only a failure to spawn is an error.
    command
        .arg(directory)
        .spawn()
        .map(|_| ())
        .map_err(FontPlatformError::Io)
}

fn unicode_name(face: &Face<'_>, name_id: u16) -> Option<String> {
    face.names()
        .into_iter()
        .filter(|name| name.name_id == name_id && name.is_unicode())
        .find_map(|name| name.to_string())
        .map(|name| name.trim().to_owned())
}

/// The font formats `FontNest` installs, and the word the operating system uses for each one in
/// the registry.
///
/// Both belong together: a registration whose value name does not carry the word Windows expects
/// for that format leaves a font registered but not usable. Deriving the name anywhere else would
/// be a second copy of this table, and a copy that drifted would make an installed font
/// unprovable, so everything goes through [`InstallableFormat`].
const INSTALLABLE_FORMATS: &[(&str, &str)] = &[("ttf", "TrueType"), ("otf", "OpenType")];

/// A font format this platform can install, recognized from a file name.
///
/// Holding it as a value rather than a string is what makes the registry name underivable for a
/// format nothing recognized: there is no way to ask for one without having been given a format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallableFormat {
    extension: &'static str,
    registry_label: &'static str,
}

impl InstallableFormat {
    /// The format `file_name` names, or `None` when it is not one `FontNest` installs.
    ///
    /// Font collections are deliberately absent. A `.ttc` holds several faces that Windows
    /// registers together under names taken from inside the file, which is a different operation
    /// from installing one face, and `FontNest` previews them rather than pretending otherwise.
    #[must_use]
    pub fn of_file_name(file_name: &str) -> Option<Self> {
        let lowered = file_name.to_ascii_lowercase();
        INSTALLABLE_FORMATS
            .iter()
            .find(|(extension, _)| lowered.ends_with(&format!(".{extension}")))
            .map(|(extension, registry_label)| Self {
                extension,
                registry_label,
            })
    }

    #[must_use]
    pub const fn extension(self) -> &'static str {
        self.extension
    }

    /// What the operating system calls this font once it is registered.
    #[must_use]
    pub fn registry_value_name(self, full_name: &str) -> String {
        format!("{full_name} ({})", self.registry_label)
    }
}

/// The one file name a provider artifact may be installed under.
///
/// Install writes it and uninstall derives it again from the bundled manifest, so the name on the
/// disk is checked against the provider's own record rather than against anything the ledger
/// stored. The first twelve characters of the artifact hash are part of the name, which is what
/// ties a file to the bytes it is supposed to hold.
///
/// # Errors
///
/// Returns [`FontPlatformError::InvalidFont`] when the file is not a format this platform
/// installs, and [`FontPlatformError::InvalidMetadata`] when the name has nothing safe left in it
/// or the hash is not a full hexadecimal digest.
pub fn managed_file_name(
    original_file_name: &str,
    source_hash: &str,
) -> Result<String, FontPlatformError> {
    let file_name = Path::new(original_file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(FontPlatformError::InvalidMetadata)?;
    let format =
        InstallableFormat::of_file_name(file_name).ok_or(FontPlatformError::InvalidFont)?;
    let stem = file_name
        .get(..file_name.len() - (format.extension().len() + 1))
        .ok_or(FontPlatformError::InvalidFont)?;
    let safe_stem = stem
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if safe_stem.is_empty()
        || source_hash.len() != 40
        || !source_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(FontPlatformError::InvalidMetadata);
    }
    Ok(format!(
        "{MANAGED_FILE_PREFIX}{}-{safe_stem}.{}",
        &source_hash[..12],
        format.extension()
    ))
}

/// Where per-user fonts live for the signed-in account.
///
/// # Errors
///
/// Returns [`FontPlatformError::UserFontDirectoryUnavailable`] when the environment does not name
/// a local application data directory.
#[cfg(windows)]
pub fn user_font_directory() -> Result<PathBuf, FontPlatformError> {
    let local_app_data =
        std::env::var_os("LOCALAPPDATA").ok_or(FontPlatformError::UserFontDirectoryUnavailable)?;
    Ok(PathBuf::from(local_app_data)
        .join("Microsoft")
        .join("Windows")
        .join("Fonts"))
}

/// Nothing installs fonts on a platform `FontNest` cannot manage, so it reports no managed
/// directory either rather than naming one nothing may write to.
///
/// # Errors
///
/// Always returns [`FontPlatformError::UnsupportedPlatform`].
#[cfg(not(windows))]
pub fn user_font_directory() -> Result<PathBuf, FontPlatformError> {
    Err(FontPlatformError::UnsupportedPlatform)
}

/// The file the operating system currently maps a per-user font registration to, if any.
///
/// Uninstall asks this before it removes anything: a value that has come to name a different file
/// belongs to another font now, and taking it away would unregister that font instead.
#[cfg(windows)]
#[must_use]
pub fn registered_user_font_path(value_name: &str) -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE};

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(USER_FONTS_REGISTRY_KEY, KEY_QUERY_VALUE)
        .ok()?
        .get_value::<String, _>(value_name)
        .ok()
}

/// No per-user font registry exists off Windows, so nothing is ever registered.
#[cfg(not(windows))]
#[must_use]
pub fn registered_user_font_path(_value_name: &str) -> Option<String> {
    None
}

/// Works out exactly where a font would be written and what it would be called in the registry,
/// without touching either.
///
/// Installation is journalled before it begins, and the journal can only be written from values
/// that are already known. Resolving them here, ahead of any change to the computer, is what lets
/// an interrupted install be undone from the record instead of from a guess.
///
/// # Errors
///
/// Returns [`FontPlatformError::InvalidMetadata`] or [`FontPlatformError::InvalidFont`] when the
/// artifact cannot produce a safe managed file name, and
/// [`FontPlatformError::UserFontDirectoryUnavailable`] when the user font directory is unknown.
#[cfg(windows)]
pub fn plan_user_font_installation(
    original_file_name: &str,
    source_hash: &str,
    metadata: &ValidatedFontMetadata,
) -> Result<PlatformInstallation, FontPlatformError> {
    let file_name = managed_file_name(original_file_name, source_hash)?;
    let format =
        InstallableFormat::of_file_name(&file_name).ok_or(FontPlatformError::InvalidFont)?;
    Ok(PlatformInstallation {
        installed_path: user_font_directory()?.join(&file_name),
        registry_value_name: format.registry_value_name(&metadata.full_name),
        display_name: metadata.full_name.clone(),
    })
}

/// Whether this path is one `FontNest` may delete while cleaning up after itself.
///
/// Recovery works from a journal that a crashed process wrote, so the path is checked again rather
/// than trusted: it must be named like a managed font, sit directly in the real user font
/// directory, and not be a symlink or junction that could redirect the delete somewhere else.
#[cfg(windows)]
pub fn is_managed_installation_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !is_managed_file_name(name) {
        return false;
    }
    // Never follow a reparse point to a delete: the target could be anywhere on the computer.
    if std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return false;
    }
    let (Ok(font_dir), Some(parent)) = (user_font_directory(), path.parent()) else {
        return false;
    };
    match (
        std::fs::canonicalize(parent),
        std::fs::canonicalize(&font_dir),
    ) {
        (Ok(parent), Ok(font_dir)) => parent == font_dir,
        // Without both real directories there is nothing safe to compare, and nothing to delete.
        _ => false,
    }
}

/// Nothing is managed on a platform that cannot install fonts, so nothing may be deleted either.
#[cfg(not(windows))]
pub fn is_managed_installation_path(_path: &Path) -> bool {
    false
}

/// Whether a file name is one `FontNest` writes. It says nothing about where the file is, so it is
/// only ever half of a decision.
#[must_use]
pub fn is_managed_file_name(name: &str) -> bool {
    name.starts_with(MANAGED_FILE_PREFIX) && InstallableFormat::of_file_name(name).is_some()
}

#[cfg(not(windows))]
pub fn plan_user_font_installation(
    _original_file_name: &str,
    _source_hash: &str,
    _metadata: &ValidatedFontMetadata,
) -> Result<PlatformInstallation, FontPlatformError> {
    Err(FontPlatformError::UnsupportedPlatform)
}

/// Carries out a plan that has already been written to the operation journal.
///
/// # Errors
///
/// Returns the platform error when the file cannot be written, the registry value conflicts with
/// an existing entry, or the operating system refuses to register the font. Every failure path
/// removes what it created before returning.
#[cfg(windows)]
pub fn install_planned_user_font(
    bytes: &[u8],
    plan: &PlatformInstallation,
) -> Result<(), FontPlatformError> {
    use std::fs::OpenOptions;
    use std::io::Write;

    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let target = plan.installed_path.clone();
    let font_dir = target
        .parent()
        .ok_or(FontPlatformError::UserFontDirectoryUnavailable)?;
    std::fs::create_dir_all(font_dir)?;

    if target.exists() {
        return Err(FontPlatformError::TargetConflict);
    }
    let temp = font_dir.join(format!(
        ".{}.{}.tmp",
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("fontnest-font"),
        std::process::id()
    ));
    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    if let Err(error) = temp_file
        .write_all(bytes)
        .and_then(|()| temp_file.sync_all())
    {
        let _ = std::fs::remove_file(&temp);
        return Err(FontPlatformError::Io(error));
    }
    drop(temp_file);
    if let Err(error) = std::fs::rename(&temp, &target) {
        let _ = std::fs::remove_file(&temp);
        return Err(FontPlatformError::Io(error));
    }

    let registry_value_name = plan.registry_value_name.clone();
    let target_value = target.to_string_lossy().into_owned();
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (fonts_key, _) = match current_user.create_subkey(USER_FONTS_REGISTRY_KEY) {
        Ok(result) => result,
        Err(error) => {
            let _ = std::fs::remove_file(&target);
            return Err(FontPlatformError::Io(error));
        }
    };
    match fonts_key.get_value::<String, _>(&registry_value_name) {
        Ok(existing) if !existing.eq_ignore_ascii_case(&target_value) => {
            let _ = std::fs::remove_file(&target);
            return Err(FontPlatformError::RegistryConflict);
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            let _ = std::fs::remove_file(&target);
            return Err(FontPlatformError::Io(error));
        }
        _ => {}
    }
    if let Err(error) = fonts_key.set_value(&registry_value_name, &target_value) {
        let _ = std::fs::remove_file(&target);
        return Err(FontPlatformError::Io(error));
    }

    if !register_font_resource(&target) {
        let _ = fonts_key.delete_value(&registry_value_name);
        let _ = std::fs::remove_file(&target);
        return Err(FontPlatformError::RegistrationFailed);
    }
    broadcast_font_change();

    Ok(())
}

#[cfg(not(windows))]
pub fn install_planned_user_font(
    _bytes: &[u8],
    _plan: &PlatformInstallation,
) -> Result<(), FontPlatformError> {
    Err(FontPlatformError::UnsupportedPlatform)
}

/// Takes back an installation, whether it failed moments ago or was left behind by a run that
/// never finished.
///
/// Both the registry value and the file are removed only when they are still the ones this
/// installation created: an entry that now points somewhere else belongs to another font, and a
/// path outside the managed naming and location is not `FontNest`'s to delete.
///
/// # Errors
///
/// Returns [`FontPlatformError::TargetConflict`] when the file is no longer recognizably a managed
/// font, and the I/O error when it exists but cannot be removed.
#[cfg(windows)]
pub fn rollback_user_font(installation: &PlatformInstallation) -> Result<(), FontPlatformError> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};

    unregister_font_resource(&installation.installed_path);
    let target_value = installation.installed_path.to_string_lossy().into_owned();
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(fonts_key) = current_user
        .open_subkey_with_flags(USER_FONTS_REGISTRY_KEY, KEY_QUERY_VALUE | KEY_SET_VALUE)
        && fonts_key
            .get_value::<String, _>(&installation.registry_value_name)
            .is_ok_and(|existing| existing.eq_ignore_ascii_case(&target_value))
    {
        let _ = fonts_key.delete_value(&installation.registry_value_name);
    }
    if installation.installed_path.exists() {
        if !is_managed_installation_path(&installation.installed_path) {
            return Err(FontPlatformError::TargetConflict);
        }
        std::fs::remove_file(&installation.installed_path)?;
    }
    broadcast_font_change();
    Ok(())
}

#[cfg(not(windows))]
pub fn rollback_user_font(_installation: &PlatformInstallation) -> Result<(), FontPlatformError> {
    Err(FontPlatformError::UnsupportedPlatform)
}

/// Stops the operating system serving a font, leaving the file itself alone.
///
/// This runs before the file moves, so an uninstall that stops here has taken a font out of use
/// without having taken it away. A registry value that has come to name a different file is left
/// exactly as it is and reported as a conflict: it belongs to another font now, and removing it
/// would unregister that one instead.
///
/// # Errors
///
/// Returns [`FontPlatformError::RegistryConflict`] when the value names another file, and the I/O
/// error when the registry cannot be read or written.
#[cfg(windows)]
pub fn unregister_user_font(
    registry_value_name: &str,
    path: &Path,
) -> Result<(), FontPlatformError> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};

    unregister_font_resource(path);
    let fonts_key = match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(USER_FONTS_REGISTRY_KEY, KEY_QUERY_VALUE | KEY_SET_VALUE)
    {
        Ok(key) => key,
        // No per-user font key at all means nothing is registered to remove.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(FontPlatformError::Io(error)),
    };
    match fonts_key.get_value::<String, _>(registry_value_name) {
        Ok(existing) if crate::managed_ownership::registers_the_same_file(&existing, path) => {
            fonts_key
                .delete_value(registry_value_name)
                .map_err(FontPlatformError::Io)?;
        }
        Ok(_) => return Err(FontPlatformError::RegistryConflict),
        // Already gone, which is the state this function is trying to reach.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(FontPlatformError::Io(error)),
    }
    broadcast_font_change();
    Ok(())
}

#[cfg(not(windows))]
pub fn unregister_user_font(
    _registry_value_name: &str,
    _path: &Path,
) -> Result<(), FontPlatformError> {
    Err(FontPlatformError::UnsupportedPlatform)
}

/// Puts a font back into service after an uninstall was undone.
///
/// # Errors
///
/// Returns [`FontPlatformError::RegistrationFailed`] when the operating system refuses the font,
/// and the I/O error when the registry cannot be written.
#[cfg(windows)]
pub fn register_user_font(registry_value_name: &str, path: &Path) -> Result<(), FontPlatformError> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let (fonts_key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(USER_FONTS_REGISTRY_KEY)
        .map_err(FontPlatformError::Io)?;
    fonts_key
        .set_value(registry_value_name, &path.to_string_lossy().into_owned())
        .map_err(FontPlatformError::Io)?;
    if !register_font_resource(path) {
        return Err(FontPlatformError::RegistrationFailed);
    }
    broadcast_font_change();
    Ok(())
}

#[cfg(not(windows))]
pub fn register_user_font(
    _registry_value_name: &str,
    _path: &Path,
) -> Result<(), FontPlatformError> {
    Err(FontPlatformError::UnsupportedPlatform)
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn register_font_resource(path: &Path) -> bool {
    use windows_sys::Win32::Graphics::Gdi::AddFontResourceExW;

    let path = wide_path(path);
    // SAFETY: `path` is a live, null-terminated UTF-16 buffer for the duration of the call;
    // flags are zero and the reserved pointer is null as required by AddFontResourceExW.
    unsafe { AddFontResourceExW(path.as_ptr(), 0, std::ptr::null()) > 0 }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn unregister_font_resource(path: &Path) {
    use windows_sys::Win32::Graphics::Gdi::RemoveFontResourceExW;

    let path = wide_path(path);
    // SAFETY: `path` is a live, null-terminated UTF-16 buffer and uses the same flags that
    // were passed to AddFontResourceExW. The reserved pointer is required to be null.
    let _ = unsafe { RemoveFontResourceExW(path.as_ptr(), 0, std::ptr::null()) };
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn broadcast_font_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_FONTCHANGE,
    };

    // SAFETY: HWND_BROADCAST and WM_FONTCHANGE are documented constants. Both message
    // parameters are zero for WM_FONTCHANGE and the optional result pointer is null.
    let _ = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_FONTCHANGE,
            0,
            0,
            SMTO_ABORTIFHUNG,
            1_000,
            std::ptr::null_mut(),
        )
    };
}

#[cfg(test)]
mod tests {
    use super::{FontPlatformError, InstallableFormat, is_managed_file_name, managed_file_name};

    /// Pins the assumption the Windows reveal path branches on: files inside the system
    /// font directory are not addressable as shell items, because Explorer renders that
    /// directory as the Fonts control panel. If a future Windows release changes this,
    /// the fallback becomes dead code and this test says so.
    ///
    /// The COM initialization matters. Without it `ILCreateFromPathW` does not consult the
    /// shell namespace and happily returns a PIDL for these files, which would make the
    /// whole check silently pass and the fallback never run.
    #[cfg(windows)]
    #[test]
    #[allow(unsafe_code)]
    fn system_font_files_are_not_shell_items_but_ordinary_files_are() {
        use std::os::windows::ffi::OsStrExt;

        use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
        use windows_sys::Win32::UI::Shell::{ILCreateFromPathW, ILFree};

        fn parses(path: &std::path::Path) -> bool {
            let wide: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            // SAFETY: `wide` is a live, null-terminated UTF-16 buffer, and any returned
            // PIDL is freed before the function returns.
            let pidl = unsafe { ILCreateFromPathW(wide.as_ptr()) };
            if pidl.is_null() {
                return false;
            }
            // SAFETY: `pidl` came from ILCreateFromPathW and is freed exactly once.
            unsafe { ILFree(pidl) };
            true
        }

        #[allow(clippy::cast_sign_loss)]
        let apartment_threaded = COINIT_APARTMENTTHREADED as u32;
        // SAFETY: Initializes COM for this test thread, matching what reveal_on_windows does
        // on its worker thread.
        unsafe { CoInitializeEx(std::ptr::null(), apartment_threaded) };

        let system_fonts = std::path::Path::new(r"C:\Windows\Fonts");
        let Some(system_font) = std::fs::read_dir(system_fonts)
            .ok()
            .and_then(|entries| entries.flatten().find(|entry| entry.path().is_file()))
        else {
            return; // No readable system fonts on this machine; nothing to assert.
        };

        let ordinary = tempfile::NamedTempFile::new().expect("a temporary file");

        assert!(
            parses(system_fonts),
            "the system font directory itself must still resolve, since the fallback opens it"
        );
        assert!(
            !parses(&system_font.path()),
            "a file inside the system font directory unexpectedly resolved as a shell item"
        );
        assert!(
            parses(ordinary.path()),
            "an ordinary file must resolve, or every reveal would fall back"
        );
    }

    #[test]
    fn managed_file_names_cannot_escape_the_user_font_directory() {
        let name = managed_file_name(
            "..\\..\\Inter[opsz,wght].ttf",
            "047c92f6e2212473dc436020afed689527076d44",
        )
        .expect("a safe managed name");

        assert_eq!(name, "FontNest-047c92f6e221-Inter_opsz_wght_.ttf");
        assert!(!name.contains(['/', '\\']));
    }

    #[test]
    fn managed_file_names_require_a_manifest_hash() {
        let error = managed_file_name("Inter.ttf", "not-a-hash")
            .expect_err("untrusted hashes must be rejected");

        assert!(matches!(error, FontPlatformError::InvalidMetadata));
    }

    // An OpenType font installed under a .ttf name is a font Windows will not serve, and one
    // FontNest could no longer derive a matching name for.
    #[test]
    fn a_managed_name_keeps_the_format_the_file_actually_is() {
        let hash = "047c92f6e2212473dc436020afed689527076d44";

        assert_eq!(
            managed_file_name("Cardo-Regular.otf", hash).expect("an OpenType name"),
            "FontNest-047c92f6e221-Cardo-Regular.otf"
        );
        assert_eq!(
            managed_file_name("Cardo-Regular.OTF", hash).expect("an OpenType name"),
            "FontNest-047c92f6e221-Cardo-Regular.otf"
        );
    }

    #[test]
    fn formats_this_platform_does_not_install_have_no_managed_name() {
        let hash = "047c92f6e2212473dc436020afed689527076d44";

        for file_name in ["Noto.ttc", "Inter.woff2", "Inter.pfb", "Inter"] {
            assert!(
                matches!(
                    managed_file_name(file_name, hash),
                    Err(FontPlatformError::InvalidFont)
                ),
                "{file_name} must not resolve to a managed name"
            );
        }
    }

    // Installation and the ownership proof both derive the registration name from this table, so
    // the words matter: Windows will not serve an OpenType font registered as a TrueType one.
    #[test]
    fn a_registration_is_named_the_way_its_format_requires() {
        let truetype = InstallableFormat::of_file_name("FontNest-047c92f6e221-Inter.ttf")
            .expect("a TrueType format");
        let opentype = InstallableFormat::of_file_name("FontNest-047c92f6e221-Cardo.otf")
            .expect("an OpenType format");

        assert_eq!(
            truetype.registry_value_name("Inter Regular"),
            "Inter Regular (TrueType)"
        );
        assert_eq!(
            opentype.registry_value_name("Cardo Regular"),
            "Cardo Regular (OpenType)"
        );
    }

    #[test]
    fn a_managed_file_is_recognized_in_every_format_fontnest_installs() {
        assert!(is_managed_file_name("FontNest-047c92f6e221-Inter.ttf"));
        assert!(is_managed_file_name("FontNest-047c92f6e221-Cardo.otf"));
        assert!(!is_managed_file_name("FontNest-047c92f6e221-Noto.ttc"));
        assert!(!is_managed_file_name("Inter.ttf"));
    }
}
