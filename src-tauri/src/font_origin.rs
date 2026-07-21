//! Works out whether a scanned font shipped with the operating system or was installed
//! onto it afterwards.
//!
//! On macOS and Linux the install location answers this on its own: the OS keeps its own
//! fonts in directories nothing installs into. Windows has no such split. A machine-wide
//! install writes into `C:\Windows\Fonts` beside the fonts Windows itself ships, so the
//! shipped families are recognized by name instead. A machine-wide font whose name is not
//! recognized is reported as installed for all users rather than guessed at, because
//! calling a font Windows owns "installed" is the mistake that leads someone to remove it.

use std::path::Path;

use crate::dto::FontOrigin;

/// Families Windows ships. Lowercase, and kept sorted so additions stay easy to review.
const WINDOWS_SHIPPED_FAMILIES: &[&str] = &[
    "arial",
    "arial black",
    "bahnschrift",
    "batang",
    "batangche",
    "calibri",
    "cambria",
    "cambria math",
    "candara",
    "comic sans ms",
    "consolas",
    "constantia",
    "corbel",
    "courier new",
    "dengxian",
    "dotum",
    "dotumche",
    "ebrima",
    "fangsong",
    "franklin gothic medium",
    "gabriola",
    "gadugi",
    "georgia",
    "gulim",
    "gulimche",
    "gungsuh",
    "gungsuhche",
    "hololens mdl2 assets",
    "impact",
    "ink free",
    "javanese text",
    "kaiti",
    "lucida console",
    "lucida sans unicode",
    "malgun gothic",
    "marlett",
    "meiryo",
    "meiryo ui",
    "mingliu",
    "mingliu_hkscs",
    "mongolian baiti",
    "mv boli",
    "myanmar text",
    "nirmala ui",
    "nsimsun",
    "palatino linotype",
    "pmingliu",
    "simhei",
    "simsun",
    "simsun-extb",
    "sylfaen",
    "symbol",
    "tahoma",
    "times new roman",
    "trebuchet ms",
    "verdana",
    "webdings",
    "wingdings",
    "wingdings 2",
    "wingdings 3",
];

/// Name prefixes Windows uses for whole families of shipped fonts, including the ones that
/// arrive with optional language features rather than the base install.
const WINDOWS_SHIPPED_PREFIXES: &[&str] = &[
    "cascadia ",
    "leelawadee",
    "microsoft ",
    "ms gothic",
    "ms mincho",
    "ms pgothic",
    "ms pmincho",
    "ms ui gothic",
    "segoe",
    "sitka",
    "yu gothic",
    "yu mincho",
];

/// Classifies a font file by where it lives and, on Windows, what it is called.
pub fn classify(path: &Path, family_name: &str) -> FontOrigin {
    let location = path.to_string_lossy().replace('\\', "/").to_lowercase();

    if is_user_font_directory(&location) {
        return FontOrigin::UserInstalled;
    }

    if location.starts_with("/system/library/fonts")
        || location.starts_with("/system/library/assetsv2/")
        || location.starts_with("/usr/share/fonts/")
    {
        return FontOrigin::SystemDefault;
    }

    // A shared macOS or Linux font directory only ever receives deliberate installs.
    if location.contains("/library/fonts/") || location.starts_with("/usr/local/share/fonts/") {
        return FontOrigin::MachineInstalled;
    }

    if location.contains("/windows/fonts/") {
        return if ships_with_windows(family_name) {
            FontOrigin::SystemDefault
        } else {
            FontOrigin::MachineInstalled
        };
    }

    FontOrigin::Unknown
}

fn is_user_font_directory(location: &str) -> bool {
    location.contains("/appdata/local/microsoft/windows/fonts/")
        || location.contains("/appdata/roaming/microsoft/windows/fonts/")
        || location.contains("/.local/share/fonts/")
        || location.contains("/.fonts/")
        || (location.contains("/users/") && location.contains("/library/fonts/"))
}

fn ships_with_windows(family_name: &str) -> bool {
    let name = family_name.trim().to_lowercase();
    WINDOWS_SHIPPED_FAMILIES.contains(&name.as_str())
        || WINDOWS_SHIPPED_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::dto::FontOrigin;

    use super::classify;

    #[test]
    fn windows_separates_shipped_fonts_from_machine_wide_installs() {
        assert_eq!(
            classify(Path::new("C:\\Windows\\Fonts\\arial.ttf"), "Arial"),
            FontOrigin::SystemDefault
        );
        assert_eq!(
            classify(Path::new("C:\\Windows\\Fonts\\segoeui.ttf"), "Segoe UI"),
            FontOrigin::SystemDefault
        );
        assert_eq!(
            classify(Path::new("C:\\Windows\\Fonts\\Inter-Regular.ttf"), "Inter"),
            FontOrigin::MachineInstalled
        );
    }

    #[test]
    fn per_user_font_directories_are_installs_on_every_platform() {
        assert_eq!(
            classify(
                Path::new("C:\\Users\\Akari\\AppData\\Local\\Microsoft\\Windows\\Fonts\\Inter.ttf"),
                "Inter"
            ),
            FontOrigin::UserInstalled
        );
        assert_eq!(
            classify(Path::new("/Users/akari/Library/Fonts/Inter.ttf"), "Inter"),
            FontOrigin::UserInstalled
        );
        assert_eq!(
            classify(
                Path::new("/home/akari/.local/share/fonts/Inter.ttf"),
                "Inter"
            ),
            FontOrigin::UserInstalled
        );
    }

    #[test]
    fn shared_directories_stay_distinct_from_shipped_directories() {
        assert_eq!(
            classify(
                Path::new("/System/Library/Fonts/Helvetica.ttc"),
                "Helvetica"
            ),
            FontOrigin::SystemDefault
        );
        assert_eq!(
            classify(Path::new("/Library/Fonts/Inter.ttf"), "Inter"),
            FontOrigin::MachineInstalled
        );
        assert_eq!(
            classify(
                Path::new("/usr/share/fonts/dejavu/DejaVuSans.ttf"),
                "DejaVu Sans"
            ),
            FontOrigin::SystemDefault
        );
        assert_eq!(
            classify(Path::new("/usr/local/share/fonts/Inter.ttf"), "Inter"),
            FontOrigin::MachineInstalled
        );
    }

    #[test]
    fn an_unrecognized_location_is_never_reported_as_a_system_font() {
        assert_eq!(
            classify(Path::new("D:\\Projects\\type\\Inter.ttf"), "Inter"),
            FontOrigin::Unknown
        );
    }

    #[test]
    fn windows_family_matching_ignores_case_and_padding() {
        assert_eq!(
            classify(
                Path::new("C:\\WINDOWS\\FONTS\\TIMES.TTF"),
                "  Times New Roman "
            ),
            FontOrigin::SystemDefault
        );
    }
}
