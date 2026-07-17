use crate::catalogue;
use crate::dto::{CommandError, FontCatalogue, Greeting};

const MAX_NAME_LENGTH: usize = 64;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri deserializes command arguments into owned values.
pub fn greet(name: String) -> Result<Greeting, CommandError> {
    let name = name.trim();

    if name.is_empty() || name.chars().count() > MAX_NAME_LENGTH {
        return Err(CommandError::invalid_name());
    }

    Ok(Greeting::new(name))
}

#[tauri::command]
pub async fn scan_installed_fonts() -> Result<FontCatalogue, CommandError> {
    tauri::async_runtime::spawn_blocking(catalogue::scan_installed_fonts)
        .await
        .map_err(|_| CommandError::catalogue_unavailable())
}

#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greeting_uses_a_trimmed_name() {
        let greeting = greet("  Akari  ".to_owned()).expect("a valid greeting");

        assert_eq!(greeting.message, "Welcome to FontNest, Akari.");
    }

    #[test]
    fn greeting_rejects_an_empty_name() {
        let error = greet("   ".to_owned()).expect_err("an empty name must be rejected");

        assert_eq!(error.code, "invalid_name");
    }
}
