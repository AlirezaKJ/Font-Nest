//! Everything `FontNest` remembers between launches, owned by the application rather than by the
//! web view.
//!
//! These settings used to live in the web view's `localStorage`. That store belongs to the
//! browser engine, not to `FontNest`: it carries no version, it has no answer for a document that
//! comes back unreadable, and clearing the web view's data takes it with it. The catalogue ledger
//! next door has all three, and settings are worth the same care, so they are a document this
//! application writes, versions and repairs.
//!
//! What the interface stores inside `session` is deliberately not modelled here. A session is a
//! record of where somebody was in the interface, it changes with the interface, and giving it a
//! second definition in Rust would only mean keeping two copies of the same shape in step. It is
//! carried through as the object it is, and the frontend checks its own fields on the way in.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

/// Sits beside the ledger and the window rectangle in the application data directory.
const PREFERENCES_FILE_NAME: &str = "preferences.json";

/// Bumped when the stored shape changes in a way an older build could not read.
const SCHEMA_VERSION: u32 = 1;

/// A specimen long enough to be a paste accident is not worth carrying between launches.
const MAX_PREVIEW_TEXT: usize = 2_000;

/// More saved previews than anyone pinned by hand, and a bound on what a hand-edited file can
/// make the interface render.
const MAX_PINNED_FAMILIES: usize = 500;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum ThemePreference {
    /// Follow the operating system.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum DensityPreference {
    #[default]
    Comfortable,
    Compact,
}

/// The settings themselves. Every field carries a default, so a document written by an older
/// build, or one somebody trimmed by hand, still loads: the missing parts simply are not set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Preferences {
    pub theme: ThemePreference,
    pub density: DensityPreference,
    pub focus_outlines: bool,
    pub preview_text: String,
    pub sidebar_collapsed: bool,
    pub pinned_family_ids: Vec<String>,
    /// Where the interface was: the view, the selection, the filters, the scroll position. Stored
    /// as it arrives and handed back the same way; see the note at the top of this file.
    #[ts(type = "Record<string, unknown> | null")]
    pub session: Option<Map<String, Value>>,
}

impl Preferences {
    /// Trims anything a hand-edited or corrupted document could otherwise make the interface
    /// carry: an enormous specimen, or more saved previews than a person could have pinned.
    fn bounded(mut self) -> Self {
        if self.preview_text.chars().count() > MAX_PREVIEW_TEXT {
            self.preview_text = self.preview_text.chars().take(MAX_PREVIEW_TEXT).collect();
        }
        self.pinned_family_ids.truncate(MAX_PINNED_FAMILIES);
        self
    }
}

/// Why the settings on disk could not be used, when that happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum PreferencesRecovery {
    /// The document could not be parsed. It is kept, under another name, and defaults are used.
    Unreadable,
    /// The document was written by a newer build. It is left exactly as found.
    NewerVersion,
}

/// What a load produced: the settings to use, and whether anything had to be recovered from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct LoadedPreferences {
    pub preferences: Preferences,
    /// `None` on an ordinary load, including the first launch, when there is simply no document.
    pub recovery: Option<PreferencesRecovery>,
}

/// The stored document. The version rides alongside the settings so a shape from a later build
/// can be recognized rather than misread.
#[derive(Serialize, Deserialize)]
struct StoredPreferences {
    version: u32,
    #[serde(flatten)]
    preferences: Preferences,
}

#[must_use]
pub fn preferences_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(PREFERENCES_FILE_NAME)
}

/// Reads the settings, repairing rather than refusing.
///
/// A missing document is the first launch and is not a failure. A document that cannot be parsed
/// is moved aside instead of deleted, so somebody can still look at what was in it, and defaults
/// take over. A document from a newer build is left exactly where it is: overwriting it would
/// destroy settings that build still expects to find.
#[must_use]
pub fn load(app_data_dir: &Path) -> LoadedPreferences {
    let path = preferences_path(app_data_dir);
    let Ok(contents) = fs::read_to_string(&path) else {
        return LoadedPreferences {
            preferences: Preferences::default(),
            recovery: None,
        };
    };

    match serde_json::from_str::<StoredPreferences>(&contents) {
        Ok(stored) if stored.version == SCHEMA_VERSION => LoadedPreferences {
            preferences: stored.preferences.bounded(),
            recovery: None,
        },
        Ok(_) => LoadedPreferences {
            preferences: Preferences::default(),
            recovery: Some(PreferencesRecovery::NewerVersion),
        },
        Err(error) => {
            log::warn!("FontNest could not read its settings ({error}); starting from defaults.");
            let kept = path.with_extension("json.unreadable");
            if let Err(error) = fs::rename(&path, &kept) {
                log::warn!("FontNest could not set the unreadable settings aside: {error}");
            }
            LoadedPreferences {
                preferences: Preferences::default(),
                recovery: Some(PreferencesRecovery::Unreadable),
            }
        }
    }
}

/// Writes the settings through a temporary file, so an interrupted write cannot leave a document
/// that the next launch has to recover from.
///
/// # Errors
///
/// Returns the underlying error when the directory cannot be created or the document cannot be
/// written or renamed into place.
pub fn save(app_data_dir: &Path, preferences: &Preferences) -> io::Result<()> {
    fs::create_dir_all(app_data_dir)?;
    let document = serde_json::to_string_pretty(&StoredPreferences {
        version: SCHEMA_VERSION,
        preferences: preferences.clone().bounded(),
    })
    .map_err(io::Error::other)?;

    let path = preferences_path(app_data_dir);
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, document)?;
    fs::rename(&temporary, &path)
}

#[cfg(test)]
mod tests {
    use super::{
        DensityPreference, MAX_PINNED_FAMILIES, MAX_PREVIEW_TEXT, Preferences, PreferencesRecovery,
        SCHEMA_VERSION, ThemePreference, load, preferences_path, save,
    };
    use serde_json::{Map, Value, json};
    use std::fs;
    use tempfile::tempdir;

    fn a_session() -> Map<String, Value> {
        json!({ "view": "discover", "scrollTop": 240 })
            .as_object()
            .expect("an object")
            .clone()
    }

    fn settings() -> Preferences {
        Preferences {
            theme: ThemePreference::Dark,
            density: DensityPreference::Compact,
            focus_outlines: true,
            preview_text: "Hamburgefonstiv".to_owned(),
            sidebar_collapsed: true,
            pinned_family_ids: vec!["family:0123".to_owned()],
            session: Some(a_session()),
        }
    }

    #[test]
    fn settings_read_back_as_they_were_written() {
        let directory = tempdir().expect("a temporary directory");

        save(directory.path(), &settings()).expect("the settings write");
        let loaded = load(directory.path());

        assert_eq!(loaded.preferences, settings());
        assert_eq!(loaded.recovery, None);
    }

    #[test]
    fn a_first_launch_starts_from_defaults_without_calling_it_a_failure() {
        let directory = tempdir().expect("a temporary directory");

        let loaded = load(directory.path());

        assert_eq!(loaded.preferences, Preferences::default());
        assert_eq!(loaded.recovery, None);
    }

    // The interface owns what a session means, so the store carries it through untouched.
    #[test]
    fn a_session_survives_without_the_store_understanding_it() {
        let directory = tempdir().expect("a temporary directory");
        let exotic = json!({ "view": "discover", "somethingNewer": { "nested": [1, 2, 3] } })
            .as_object()
            .expect("an object")
            .clone();

        save(
            directory.path(),
            &Preferences {
                session: Some(exotic.clone()),
                ..Preferences::default()
            },
        )
        .expect("the settings write");

        assert_eq!(load(directory.path()).preferences.session, Some(exotic));
    }

    #[test]
    fn a_document_missing_fields_fills_them_with_defaults() {
        let directory = tempdir().expect("a temporary directory");
        fs::write(
            preferences_path(directory.path()),
            r#"{"version":1,"theme":"dark"}"#,
        )
        .expect("the fixture writes");

        let loaded = load(directory.path());

        assert_eq!(loaded.preferences.theme, ThemePreference::Dark);
        assert_eq!(loaded.preferences.density, DensityPreference::Comfortable);
        assert_eq!(loaded.preferences.pinned_family_ids, Vec::<String>::new());
        assert_eq!(loaded.recovery, None);
    }

    #[test]
    fn an_unreadable_document_is_set_aside_and_defaults_take_over() {
        let directory = tempdir().expect("a temporary directory");
        let path = preferences_path(directory.path());
        fs::write(&path, r#"{"version":1,"theme":"da"#).expect("the fixture writes");

        let loaded = load(directory.path());

        assert_eq!(loaded.preferences, Preferences::default());
        assert_eq!(loaded.recovery, Some(PreferencesRecovery::Unreadable));
        assert!(!path.exists(), "the unreadable document is moved aside");
        assert!(
            path.with_extension("json.unreadable").exists(),
            "and kept rather than deleted"
        );
    }

    #[test]
    fn a_document_from_a_newer_build_is_left_exactly_as_found() {
        let directory = tempdir().expect("a temporary directory");
        let path = preferences_path(directory.path());
        let document = format!(r#"{{"version":{},"theme":"dark"}}"#, SCHEMA_VERSION + 1);
        fs::write(&path, &document).expect("the fixture writes");

        let loaded = load(directory.path());

        assert_eq!(loaded.preferences, Preferences::default());
        assert_eq!(loaded.recovery, Some(PreferencesRecovery::NewerVersion));
        assert_eq!(
            fs::read_to_string(&path).expect("the document is still there"),
            document
        );
    }

    #[test]
    fn an_enormous_specimen_is_cut_to_something_the_interface_can_render() {
        let directory = tempdir().expect("a temporary directory");
        save(
            directory.path(),
            &Preferences {
                preview_text: "n".repeat(MAX_PREVIEW_TEXT + 500),
                pinned_family_ids: vec!["family:0123".to_owned(); MAX_PINNED_FAMILIES + 50],
                ..Preferences::default()
            },
        )
        .expect("the settings write");

        let loaded = load(directory.path());

        assert_eq!(
            loaded.preferences.preview_text.chars().count(),
            MAX_PREVIEW_TEXT
        );
        assert_eq!(
            loaded.preferences.pinned_family_ids.len(),
            MAX_PINNED_FAMILIES
        );
    }

    #[test]
    fn saving_twice_leaves_one_document_and_no_temporary_file() {
        let directory = tempdir().expect("a temporary directory");

        save(directory.path(), &Preferences::default()).expect("the first write");
        save(directory.path(), &settings()).expect("the second write");

        let left = fs::read_dir(directory.path())
            .expect("the directory lists")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(left, vec!["preferences.json".to_owned()]);
        assert_eq!(load(directory.path()).preferences, settings());
    }

    // Recovering has to leave somewhere to write to, or the next save fails too.
    #[test]
    fn settings_can_be_written_again_after_a_recovery() {
        let directory = tempdir().expect("a temporary directory");
        fs::write(preferences_path(directory.path()), "not json at all").expect("the fixture");

        assert_eq!(
            load(directory.path()).recovery,
            Some(PreferencesRecovery::Unreadable)
        );
        save(directory.path(), &settings()).expect("the settings write");

        assert_eq!(load(directory.path()).preferences, settings());
    }
}
