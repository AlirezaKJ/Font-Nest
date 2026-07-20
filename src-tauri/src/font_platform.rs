use std::path::{Path, PathBuf};

use ttf_parser::{Face, name_id};

#[derive(Debug, Clone)]
pub struct ValidatedFontMetadata {
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

    Ok(ValidatedFontMetadata { full_name })
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

fn managed_file_name(
    original_file_name: &str,
    source_hash: &str,
) -> Result<String, FontPlatformError> {
    let file_name = Path::new(original_file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(FontPlatformError::InvalidMetadata)?;
    let stem = file_name
        .strip_suffix(".ttf")
        .or_else(|| file_name.strip_suffix(".TTF"))
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
    Ok(format!("FontNest-{}-{safe_stem}.ttf", &source_hash[..12]))
}

#[cfg(windows)]
pub fn install_user_font(
    bytes: &[u8],
    original_file_name: &str,
    source_hash: &str,
    metadata: &ValidatedFontMetadata,
) -> Result<PlatformInstallation, FontPlatformError> {
    use std::fs::OpenOptions;
    use std::io::Write;

    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let local_app_data =
        std::env::var_os("LOCALAPPDATA").ok_or(FontPlatformError::UserFontDirectoryUnavailable)?;
    let font_dir = PathBuf::from(local_app_data)
        .join("Microsoft")
        .join("Windows")
        .join("Fonts");
    std::fs::create_dir_all(&font_dir)?;

    let target = font_dir.join(managed_file_name(original_file_name, source_hash)?);
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

    let registry_value_name = format!("{} (TrueType)", metadata.full_name);
    let target_value = target.to_string_lossy().into_owned();
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (fonts_key, _) = match current_user
        .create_subkey("Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts")
    {
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

    Ok(PlatformInstallation {
        installed_path: target,
        registry_value_name,
        display_name: metadata.full_name.clone(),
    })
}

#[cfg(not(windows))]
pub fn install_user_font(
    _bytes: &[u8],
    _original_file_name: &str,
    _source_hash: &str,
    _metadata: &ValidatedFontMetadata,
) -> Result<PlatformInstallation, FontPlatformError> {
    Err(FontPlatformError::UnsupportedPlatform)
}

#[cfg(windows)]
pub fn rollback_user_font(installation: &PlatformInstallation) -> Result<(), FontPlatformError> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};

    unregister_font_resource(&installation.installed_path);
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(fonts_key) = current_user.open_subkey_with_flags(
        "Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts",
        KEY_SET_VALUE,
    ) {
        let _ = fonts_key.delete_value(&installation.registry_value_name);
    }
    if installation.installed_path.exists() {
        std::fs::remove_file(&installation.installed_path)?;
    }
    broadcast_font_change();
    Ok(())
}

#[cfg(not(windows))]
pub fn rollback_user_font(_installation: &PlatformInstallation) -> Result<(), FontPlatformError> {
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
    use super::{FontPlatformError, managed_file_name};

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
}
