use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct Greeting {
    pub app_name: &'static str,
    pub message: String,
    pub version: &'static str,
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
}
