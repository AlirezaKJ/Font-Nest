use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Greeting {
    pub app_name: &'static str,
    pub message: String,
    pub version: &'static str,
}

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

impl Greeting {
    pub fn new(name: &str) -> Self {
        Self {
            app_name: "FontNest",
            message: format!("Welcome to FontNest, {name}."),
            version: env!("CARGO_PKG_VERSION"),
        }
    }
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
    pub source: String,
    pub file_name: String,
    pub face_index: u32,
    pub monospaced: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FontParserJsonExport {
    pub face_id: String,
    pub parser_name: &'static str,
    pub parser_version: &'static str,
    pub json_byte_length: u32,
    pub raw_json: String,
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
    pub sources: Vec<String>,
    pub monospaced: bool,
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
    pub data_url: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub message: &'static str,
}

impl CommandError {
    pub const fn invalid_name() -> Self {
        Self {
            code: "invalid_name",
            message: "Name must contain between 1 and 64 characters.",
        }
    }

    pub const fn catalogue_unavailable() -> Self {
        Self {
            code: "catalogue_unavailable",
            message: "FontNest could not read the installed font catalogue. Try scanning again.",
        }
    }

    pub const fn update_check_failed() -> Self {
        Self {
            code: "update_check_failed",
            message: "FontNest could not check for updates. Check your connection and try again.",
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

    pub const fn font_glyph_unavailable() -> Self {
        Self {
            code: "font_glyph_unavailable",
            message: "The selected font does not expose that character as a glyph.",
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

    pub const fn managed_storage_unavailable() -> Self {
        Self {
            code: "managed_storage_unavailable",
            message: "FontNest could not open its managed-installation ledger.",
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
