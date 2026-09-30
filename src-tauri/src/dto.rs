use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct AppUpdateInfo {
    pub current_version: String,
    pub version: String,
    pub notes: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "event", content = "data", rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum AppUpdateEvent {
    DownloadStarted { total: Option<u32> },
    DownloadProgress { downloaded: u32, total: Option<u32> },
    Installing,
}

/// Where a font came from, ordered from the fonts the operating system owns to the ones
/// somebody added. Ordering matters: it decides how a family lists mixed origins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum FontOrigin {
    /// Shipped with the operating system.
    SystemDefault,
    /// Installed for everyone on this computer.
    MachineInstalled,
    /// Installed for the current user only.
    UserInstalled,
    /// Loaded from somewhere `FontNest` cannot attribute.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontFaceSummary {
    pub id: String,
    pub post_script_name: String,
    pub style_name: String,
    pub style: String,
    pub weight: u16,
    pub format: String,
    pub origin: FontOrigin,
    pub file_name: String,
    pub face_index: u32,
    pub monospaced: bool,
    /// Carries variation axes, so one file covers a range of weights or widths.
    pub variable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontFaceMetrics {
    pub units_per_em: u16,
    pub ascender: i16,
    pub capital_height: Option<i16>,
    pub capital_height_source: &'static str,
    pub x_height: Option<i16>,
    pub x_height_source: &'static str,
    pub baseline: i16,
    pub descender: i16,
    pub line_gap: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontFaceNames {
    pub full_name: Option<String>,
    pub version: Option<String>,
    pub manufacturer: Option<String>,
    pub designer: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontVariationAxis {
    pub tag: String,
    pub name_id: u16,
    pub minimum: f32,
    pub default: f32,
    pub maximum: f32,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontEmbeddingProperties {
    pub permissions: Option<String>,
    pub subsetting_allowed: bool,
    pub outline_embedding_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontFaceProperties {
    pub glyph_count: u16,
    pub unicode_codepoint_count: u32,
    pub table_count: u16,
    pub weight: u16,
    pub width: u16,
    pub italic_angle: f32,
    pub traits: Vec<String>,
    pub embedding: FontEmbeddingProperties,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontFaceInspection {
    pub face_id: String,
    pub parser_name: &'static str,
    pub parser_version: &'static str,
    pub metrics: FontFaceMetrics,
    pub names: FontFaceNames,
    pub properties: FontFaceProperties,
    pub variation_axes: Vec<FontVariationAxis>,
    pub unicode_codepoints: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontGlyphVariationValue {
    pub tag: String,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontGlyphOutlineRequest {
    pub face_id: String,
    pub codepoint: u32,
    pub variations: Vec<FontGlyphVariationValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontGlyphBounds {
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontGlyphOutlinePoint {
    pub x: f32,
    pub y: f32,
    pub kind: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontGlyphOutlineHandle {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontGlyphOutline {
    pub face_id: String,
    pub codepoint: u32,
    pub glyph_id: u16,
    pub glyph_name: Option<String>,
    pub units_per_em: u16,
    pub advance_width: Option<u16>,
    pub left_side_bearing: Option<i16>,
    pub bounds: Option<FontGlyphBounds>,
    pub path_data: String,
    pub points: Vec<FontGlyphOutlinePoint>,
    pub handles: Vec<FontGlyphOutlineHandle>,
    pub contour_count: u16,
    pub outline_available: bool,
}

/// How much of one parser-snapshot section made it into the document.
///
/// The snapshot is a bounded diagnostic sample, so every capped section reports what it
/// carries against what the face actually holds; the interface says so rather than letting
/// a partial list read as the whole font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontParserJsonSection {
    pub included: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontParserJsonRequest {
    pub face_id: String,
    /// Caller-generated ID for this export, so it can be cancelled while it still runs.
    pub export_id: String,
}

/// Ordered messages for one parser-snapshot export.
///
/// The document arrives in chunks instead of as a single string: a large snapshot never
/// becomes one oversized IPC payload, and the interface can show progress and give up part
/// way through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "event",
    content = "data",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum FontParserJsonEvent {
    Started {
        face_id: String,
        parser_name: &'static str,
        parser_version: &'static str,
        total_bytes: u32,
        chunk_count: u32,
        truncated: bool,
        unicode_mappings: FontParserJsonSection,
        glyphs: FontParserJsonSection,
    },
    Chunk {
        index: u32,
        text: String,
    },
    Finished,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontFamilySummary {
    pub id: String,
    pub name: String,
    pub face_count: u32,
    pub file_count: u32,
    pub styles: Vec<String>,
    pub weights: Vec<u16>,
    pub formats: Vec<String>,
    pub origins: Vec<FontOrigin>,
    pub monospaced: bool,
    /// True when any face in the family carries variation axes.
    pub variable: bool,
    pub has_conflict: bool,
    pub faces: Vec<FontFaceSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontCatalogue {
    pub families: Vec<FontFamilySummary>,
    pub family_count: u32,
    pub face_count: u32,
    pub conflict_count: u32,
    pub scan_duration_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GoogleFontPageRequest {
    pub query: String,
    pub category: String,
    pub subset: String,
    pub technology: String,
    pub availability: String,
    pub sort: String,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GoogleFontFamilySummary {
    pub id: String,
    pub family: String,
    pub category: String,
    pub subsets: Vec<String>,
    pub license: String,
    pub artifact_count: u32,
    pub preview_artifact_id: String,
    pub variable: bool,
    pub last_modified: String,
    pub installed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GoogleFontPage {
    pub families: Vec<GoogleFontFamilySummary>,
    pub total: u32,
    pub offset: u32,
    pub limit: u32,
    pub snapshot: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GoogleFontArtifactSummary {
    pub id: String,
    pub file_name: String,
    pub style: String,
    pub format: String,
    pub size_bytes: u32,
    pub installed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GoogleFontFamilyDetails {
    pub id: String,
    pub family: String,
    pub category: String,
    pub subsets: Vec<String>,
    pub license: String,
    pub last_modified: String,
    pub version: String,
    pub preview_artifact_id: String,
    pub artifacts: Vec<GoogleFontArtifactSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GoogleFontPreview {
    pub artifact_id: String,
    pub font_family: String,
    /// `fontnest-preview` URL for the verified bytes. Never a data URL and never a path:
    /// the web view fetches through the internal protocol by opaque handle.
    pub preview_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct InstallGoogleFontRequest {
    pub family_id: String,
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GoogleFontInstallResult {
    pub family_id: String,
    pub family_name: String,
    pub installed_artifact_ids: Vec<String>,
    pub already_installed_artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct UninstallGoogleFontRequest {
    pub family_id: String,
    /// The artifacts to remove. An empty list means every artifact of the family that `FontNest`
    /// installed.
    pub artifact_ids: Vec<String>,
}

/// Why `FontNest` would not remove a font it has a record of installing.
///
/// Each value names the check that stopped, because a font somebody replaced and a ledger somebody
/// edited need different things said about them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum FontRemovalRefusal {
    /// The font came from a source this version of `FontNest` no longer knows.
    UnknownSource,
    /// The per-user font folder could not be located.
    LocationUnavailable,
    /// The record disagrees with where the font would have been installed.
    RecordMismatch,
    /// The font is no longer where it was installed.
    Missing,
    /// The path leads somewhere else now, through a link or a second name for the file.
    Redirected,
    /// The file resolved outside the per-user font folder.
    OutsideFontFolder,
    /// The file no longer holds the bytes that were installed.
    Changed,
    /// The font registration no longer points at this file.
    NotRegistered,
    /// The font is protected by the operating system.
    Protected,
    /// The font file could not be read.
    Unreadable,
}

/// One font a removal left alone, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct RefusedFontRemoval {
    pub artifact_id: String,
    pub display_name: String,
    pub reason: FontRemovalRefusal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GoogleFontUninstallResult {
    pub family_id: String,
    pub family_name: String,
    pub removed_artifact_ids: Vec<String>,
    /// Fonts `FontNest` would not remove. They are still installed and still registered.
    pub refused: Vec<RefusedFontRemoval>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct LocalFontFaceSummary {
    pub face_index: u32,
    pub family_name: String,
    pub subfamily_name: String,
    pub full_name: String,
    pub post_script_name: String,
    pub is_variable: bool,
    pub glyph_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ValidatedLocalFont {
    /// Opaque handle the internal preview protocol resolves to validated bytes.
    pub handle: String,
    /// Synthetic family name so a duplicate installed family cannot shadow the preview.
    pub preview_family: String,
    /// Internal-protocol URL the web view loads the validated bytes from.
    pub preview_url: String,
    /// Sanitized display file name. Never an authoritative filesystem path.
    pub file_name: String,
    pub format: String,
    pub face_count: u32,
    pub faces: Vec<LocalFontFaceSummary>,
}

/// Why `FontNest` cannot mutate managed font state right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum ManagedStorageRecovery {
    /// The managed-installation ledger could not be opened, created, or migrated.
    LedgerUnavailable,
    /// The ledger was written by a newer `FontNest` than this one.
    SchemaTooNew,
    /// Another `FontNest` process holds the writer lock for managed font state.
    Locked,
}

/// Whether installing, updating, uninstalling, repairing, and restoring fonts are available in
/// this session. Browsing, previewing, and inspecting stay available either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ManagedStorageStatus {
    pub writable: bool,
    pub reason: Option<ManagedStorageRecovery>,
    /// Interrupted font operations undone during this launch, so the session can say what it
    /// cleaned up instead of the user meeting fonts they never finished installing.
    pub recovered_operations: u32,
    /// Operations `FontNest` has given up undoing, including ones from earlier launches. Their
    /// files are still on the computer.
    pub quarantined_operations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub message: &'static str,
}

impl CommandError {
    pub const fn catalogue_unavailable() -> Self {
        Self {
            code: "catalogue_unavailable",
            message: "FontNest could not read the installed font catalogue. Try scanning again.",
        }
    }

    pub const fn preferences_unavailable() -> Self {
        Self {
            code: "preferences_unavailable",
            message: "FontNest could not save your settings. They will apply until you quit.",
        }
    }

    pub const fn update_check_failed() -> Self {
        Self {
            code: "update_check_failed",
            message: "FontNest could not check for updates. Check your connection and try again.",
        }
    }

    pub const fn release_notes_unavailable() -> Self {
        Self {
            code: "release_notes_unavailable",
            message: "FontNest could not fetch the latest release notes. Showing the bundled copy.",
        }
    }

    pub const fn update_unavailable() -> Self {
        Self {
            code: "update_unavailable",
            message: "That FontNest update is no longer available. Check again.",
        }
    }

    pub const fn update_changed() -> Self {
        Self {
            code: "update_changed",
            message: "A newer FontNest update became available. Check again before installing.",
        }
    }

    pub const fn update_install_failed() -> Self {
        Self {
            code: "update_install_failed",
            message: "FontNest could not install the update. Check your connection and try again.",
        }
    }

    pub const fn font_face_unavailable() -> Self {
        Self {
            code: "font_face_unavailable",
            message: "That font face is no longer available. Scan your library again.",
        }
    }

    pub const fn font_file_unavailable() -> Self {
        Self {
            code: "font_file_unavailable",
            message: "That font is not stored as a file FontNest can open.",
        }
    }

    pub const fn font_file_reveal_failed() -> Self {
        Self {
            code: "font_file_reveal_failed",
            message: "FontNest could not open that font in your file manager.",
        }
    }

    pub const fn font_parser_unavailable() -> Self {
        Self {
            code: "font_parser_unavailable",
            message: "FontNest could not parse the selected font face.",
        }
    }

    pub const fn invalid_glyph_request() -> Self {
        Self {
            code: "invalid_glyph_request",
            message: "That glyph outline request is not valid.",
        }
    }

    pub const fn invalid_parser_export_request() -> Self {
        Self {
            code: "invalid_parser_export_request",
            message: "That parser export request is not valid.",
        }
    }

    pub const fn too_many_parser_exports() -> Self {
        Self {
            code: "too_many_parser_exports",
            message: "Too many parser exports are already running. Wait for one to finish.",
        }
    }

    pub const fn font_glyph_unavailable() -> Self {
        Self {
            code: "font_glyph_unavailable",
            message: "The selected font does not expose that character as a glyph.",
        }
    }

    pub const fn local_font_unreadable() -> Self {
        Self {
            code: "local_font_unreadable",
            message: "FontNest could not read that font file. Check that it still exists.",
        }
    }

    pub const fn local_font_too_large() -> Self {
        Self {
            code: "local_font_too_large",
            message: "That font file is too large for FontNest to preview.",
        }
    }

    pub const fn local_font_invalid() -> Self {
        Self {
            code: "local_font_invalid",
            message: "That file is not a valid desktop font FontNest can preview.",
        }
    }

    pub const fn online_catalogue_unavailable() -> Self {
        Self {
            code: "online_catalogue_unavailable",
            message: "FontNest could not open the bundled Google Fonts catalogue.",
        }
    }

    pub const fn invalid_google_font_request() -> Self {
        Self {
            code: "invalid_google_font_request",
            message: "That Google Fonts selection is no longer available. Refresh and try again.",
        }
    }

    pub const fn font_download_failed() -> Self {
        Self {
            code: "font_download_failed",
            message: "FontNest could not securely download that font. Check your connection and try again.",
        }
    }

    pub const fn font_validation_failed() -> Self {
        Self {
            code: "font_validation_failed",
            message: "The downloaded file did not match the trusted Google Fonts catalogue.",
        }
    }

    pub const fn font_install_failed() -> Self {
        Self {
            code: "font_install_failed",
            message: "Windows could not install that font for the current user.",
        }
    }

    pub const fn font_uninstall_failed() -> Self {
        Self {
            code: "font_uninstall_failed",
            message: "FontNest could not finish removing that font. Nothing was deleted.",
        }
    }

    pub const fn managed_storage_unavailable() -> Self {
        Self {
            code: "managed_storage_unavailable",
            message: "FontNest could not open its managed-installation ledger.",
        }
    }

    /// The refusal a managed operation returns while `FontNest` is in read-only recovery mode.
    pub const fn managed_storage_recovery(reason: ManagedStorageRecovery) -> Self {
        match reason {
            ManagedStorageRecovery::LedgerUnavailable => Self {
                code: "managed_storage_recovery",
                message: "FontNest could not open its managed-installation ledger, so installing and removing fonts is switched off until it recovers.",
            },
            ManagedStorageRecovery::SchemaTooNew => Self {
                code: "managed_storage_schema_too_new",
                message: "This font ledger was written by a newer FontNest. Update FontNest to manage fonts again.",
            },
            ManagedStorageRecovery::Locked => Self {
                code: "managed_storage_locked",
                message: "Another FontNest is already running and managing fonts on this computer.",
            },
        }
    }

    #[cfg(not(windows))]
    pub const fn font_platform_unsupported() -> Self {
        Self {
            code: "font_platform_unsupported",
            message: "Online font installation is currently available on Windows only.",
        }
    }

    pub const fn untrusted_origin() -> Self {
        Self {
            code: "untrusted_origin",
            message: "Font installation is not allowed from this window.",
        }
    }
}
