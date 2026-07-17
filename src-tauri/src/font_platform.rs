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
