use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use futures_util::StreamExt;
use reqwest::Url;
use serde::Deserialize;
use sha1::{Digest, Sha1};

use crate::dto::{
    GoogleFontArtifactSummary, GoogleFontFamilyDetails, GoogleFontFamilySummary,
    GoogleFontInstallResult, GoogleFontPage as GoogleFontPageDto,
    GoogleFontPageRequest as GoogleFontPageRequestDto, GoogleFontPreview, InstallGoogleFontRequest,
};
use crate::font_platform::{
    FontPlatformError, PlatformInstallation, ValidatedFontMetadata, install_user_font,
    rollback_user_font, validate_font,
};
use crate::managed_installations::{ManagedInstallationRecord, ManagedInstallationRepository};

const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MAX_CATALOGUE_PAGE_SIZE: usize = 100;
const MAX_INSTALL_ARTIFACTS: usize = 32;
const MAX_FONT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_INSTALL_BYTES: u64 = 128 * 1024 * 1024;
const MAX_LICENSE_BYTES: u64 = 256 * 1024;
const PROVIDER_ID: &str = "google-fonts";
const BUNDLED_MANIFEST: &str = include_str!("../resources/google-fonts.json");

static MANIFEST: OnceLock<Result<GoogleFontsManifest, String>> = OnceLock::new();

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum GoogleFontsError {
    #[error("the bundled catalogue is invalid")]
    Manifest,
    #[error("the requested family or artifact is invalid")]
    InvalidRequest,
    #[error("the trusted source could not be downloaded")]
    Download,
    #[error("downloaded bytes did not match the bundled manifest")]
    Integrity,
    #[error("the downloaded font could not be parsed")]
    FontValidation,
    #[error("the managed installation ledger is unavailable")]
    Database,
    #[error("the operating system rejected the font installation")]
    Platform,
    #[cfg(not(windows))]
    #[error("font installation is unsupported on this platform")]
    UnsupportedPlatform,
}

#[derive(Debug)]
struct DownloadedFont {
    artifact: GoogleFontArtifact,
    bytes: Vec<u8>,
    metadata: ValidatedFontMetadata,
}

#[derive(Debug, Clone)]
struct CompletedInstallation {
    artifact: GoogleFontArtifact,
    platform: PlatformInstallation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleFontsManifest {
    schema_version: u32,
    generated_at: String,
    source_commit: String,
    snapshot: String,
    families: Vec<GoogleFontFamily>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleFontFamily {
    id: String,
    family: String,
    category: String,
    subsets: Vec<String>,
    license: String,
    license_url: String,
    license_git_blob_sha: String,
    license_size_bytes: u64,
    last_modified: String,
    version: String,
    preview_artifact_id: String,
    artifacts: Vec<GoogleFontArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleFontArtifact {
    id: String,
    file_name: String,
    style: String,
    format: String,
    download_url: String,
    git_blob_sha: String,
    size_bytes: u64,
}

#[derive(Debug, Clone)]
struct GoogleFontPageRequest {
    query: String,
    category: String,
    offset: usize,
    limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GoogleFontPage {
    family_ids: Vec<String>,
    total: usize,
    offset: usize,
    limit: usize,
}

pub fn initialize_storage(app_data_dir: &Path) -> Result<(), GoogleFontsError> {
    let repository = installation_repository(app_data_dir);
    repository
        .initialize()
        .map_err(|_| GoogleFontsError::Database)
}

pub fn list_fonts(
    request: &GoogleFontPageRequestDto,
    app_data_dir: &Path,
) -> Result<GoogleFontPageDto, GoogleFontsError> {
    let manifest = bundled_manifest()?;
    let installed_family_ids = installation_repository(app_data_dir)
        .installed_family_ids(PROVIDER_ID)
        .map_err(|_| GoogleFontsError::Database)?;
    let internal_request = GoogleFontPageRequest {
        query: request.query.clone(),
        category: request.category.clone(),
        offset: usize::try_from(request.offset).unwrap_or(usize::MAX),
        limit: usize::try_from(request.limit).unwrap_or(usize::MAX),
    };
    let page = catalogue_page(manifest, &internal_request);
    let families = page
        .family_ids
        .iter()
        .filter_map(|family_id| {
            manifest
                .families
                .iter()
                .find(|family| family.id == *family_id)
        })
        .map(|family| family_summary(family, installed_family_ids.contains(&family.id)))
        .collect();

    Ok(GoogleFontPageDto {
        families,
        total: u32::try_from(page.total).unwrap_or(u32::MAX),
        offset: u32::try_from(page.offset).unwrap_or(u32::MAX),
        limit: u32::try_from(page.limit).unwrap_or(u32::MAX),
        snapshot: manifest.snapshot.clone(),
    })
}

pub fn font_details(
    family_id: &str,
    app_data_dir: &Path,
) -> Result<GoogleFontFamilyDetails, GoogleFontsError> {
    if !is_safe_id(family_id) {
        return Err(GoogleFontsError::InvalidRequest);
    }
    let manifest = bundled_manifest()?;
    let family = manifest
        .families
        .iter()
        .find(|family| family.id == family_id)
        .ok_or(GoogleFontsError::InvalidRequest)?;
    let installed = installation_repository(app_data_dir)
        .installed_artifact_ids(PROVIDER_ID, family_id)
        .map_err(|_| GoogleFontsError::Database)?
        .into_iter()
        .collect::<HashSet<_>>();

    Ok(GoogleFontFamilyDetails {
        id: family.id.clone(),
        family: family.family.clone(),
        category: family.category.clone(),
        subsets: family.subsets.clone(),
        license: family.license.clone(),
        last_modified: family.last_modified.clone(),
        version: family.version.clone(),
        preview_artifact_id: family.preview_artifact_id.clone(),
        artifacts: family
            .artifacts
            .iter()
            .map(|artifact| artifact_summary(artifact, installed.contains(&artifact.id)))
            .collect(),
    })
}

pub async fn prepare_preview(
    artifact_id: &str,
    cache_dir: &Path,
) -> Result<GoogleFontPreview, GoogleFontsError> {
    if !is_safe_id(artifact_id) {
        return Err(GoogleFontsError::InvalidRequest);
    }
    let manifest = bundled_manifest()?;
    let artifact = manifest
        .families
        .iter()
        .flat_map(|family| family.artifacts.iter())
        .find(|artifact| artifact.id == artifact_id)
        .cloned()
        .ok_or(GoogleFontsError::InvalidRequest)?;
    let downloaded = cached_font(&artifact, cache_dir).await?;
    let font_family = format!("FontNestRemote{}", &artifact.git_blob_sha[..12]);

    Ok(GoogleFontPreview {
        artifact_id: artifact.id,
        font_family,
        data_url: format!(
            "data:font/ttf;base64,{}",
            BASE64_STANDARD.encode(downloaded.bytes)
        ),
    })
}

pub async fn install_fonts(
    request: &InstallGoogleFontRequest,
    cache_dir: &Path,
    app_data_dir: &Path,
) -> Result<GoogleFontInstallResult, GoogleFontsError> {
    let manifest = bundled_manifest()?;
    let family = manifest
        .families
        .iter()
        .find(|family| family.id == request.family_id)
        .cloned()
        .ok_or(GoogleFontsError::InvalidRequest)?;
    let selected = selected_artifacts(manifest, &request.family_id, &request.artifact_ids)
        .map_err(|_| GoogleFontsError::InvalidRequest)?;
    let selected_bytes = selected.iter().try_fold(0_u64, |total, artifact| {
        total.checked_add(artifact.size_bytes)
    });
    if selected_bytes.is_none_or(|total| total > MAX_INSTALL_BYTES) {
        return Err(GoogleFontsError::InvalidRequest);
    }
    let repository = installation_repository(app_data_dir);
    let repository_for_read = repository.clone();
    let family_id = family.id.clone();
    let already_installed = tauri::async_runtime::spawn_blocking(move || {
        repository_for_read
            .installed_artifact_ids(PROVIDER_ID, &family_id)
            .map_err(|_| GoogleFontsError::Database)
    })
    .await
    .map_err(|_| GoogleFontsError::Database)??;
    let already_installed_set = already_installed.iter().collect::<HashSet<_>>();
    let pending = selected
        .into_iter()
        .filter(|artifact| !already_installed_set.contains(&artifact.id))
        .collect::<Vec<_>>();

    if pending.is_empty() {
        return Ok(GoogleFontInstallResult {
            family_id: family.id,
            family_name: family.family,
            installed_artifact_ids: Vec::new(),
            already_installed_artifact_ids: already_installed,
        });
    }

    let license_path = preserve_license(&family, app_data_dir).await?;
    let mut downloads = Vec::with_capacity(pending.len());
    for artifact in pending {
        downloads.push(cached_font(&artifact, cache_dir).await?);
    }

    let completed = tauri::async_runtime::spawn_blocking(move || install_batch(downloads))
        .await
        .map_err(|_| GoogleFontsError::Platform)??;
    let records = completed
        .iter()
        .map(|installation| ManagedInstallationRecord {
            id: format!("{PROVIDER_ID}:{}", installation.artifact.id),
            provider: PROVIDER_ID.to_owned(),
            family_id: family.id.clone(),
            artifact_id: installation.artifact.id.clone(),
            family_name: family.family.clone(),
            display_name: installation.platform.display_name.clone(),
            source_commit: manifest.source_commit.clone(),
            source_hash: installation.artifact.git_blob_sha.clone(),
            installed_path: installation
                .platform
                .installed_path
                .to_string_lossy()
                .into_owned(),
            registry_value_name: installation.platform.registry_value_name.clone(),
            license: family.license.clone(),
            license_path: license_path.to_string_lossy().into_owned(),
        })
        .collect::<Vec<_>>();
    let completed_for_rollback = completed.clone();
    let repository_for_write = repository.clone();
    if tauri::async_runtime::spawn_blocking(move || repository_for_write.record_batch(&records))
        .await
        .map_err(|_| GoogleFontsError::Database)?
        .is_err()
    {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            for installation in completed_for_rollback.iter().rev() {
                let _ = rollback_user_font(&installation.platform);
            }
        })
        .await;
        return Err(GoogleFontsError::Database);
    }

    Ok(GoogleFontInstallResult {
        family_id: family.id,
        family_name: family.family,
        installed_artifact_ids: completed
            .iter()
            .map(|installation| installation.artifact.id.clone())
            .collect(),
        already_installed_artifact_ids: already_installed,
    })
}

fn bundled_manifest() -> Result<&'static GoogleFontsManifest, GoogleFontsError> {
    MANIFEST
        .get_or_init(|| parse_manifest(BUNDLED_MANIFEST))
        .as_ref()
        .map_err(|_| GoogleFontsError::Manifest)
}

fn installation_repository(app_data_dir: &Path) -> ManagedInstallationRepository {
    ManagedInstallationRepository::new(app_data_dir.join("fontnest.sqlite3"))
}

fn family_summary(family: &GoogleFontFamily, installed: bool) -> GoogleFontFamilySummary {
    GoogleFontFamilySummary {
        id: family.id.clone(),
        family: family.family.clone(),
        category: family.category.clone(),
        subsets: family.subsets.clone(),
        license: family.license.clone(),
        artifact_count: u32::try_from(family.artifacts.len()).unwrap_or(u32::MAX),
        preview_artifact_id: family.preview_artifact_id.clone(),
        installed,
    }
}

fn artifact_summary(artifact: &GoogleFontArtifact, installed: bool) -> GoogleFontArtifactSummary {
    GoogleFontArtifactSummary {
        id: artifact.id.clone(),
        file_name: artifact.file_name.clone(),
        style: artifact.style.clone(),
        format: artifact.format.clone(),
        size_bytes: u32::try_from(artifact.size_bytes)
            .expect("validated Google Fonts artifacts are smaller than u32::MAX"),
        installed,
    }
}

async fn cached_font(
    artifact: &GoogleFontArtifact,
    cache_dir: &Path,
) -> Result<DownloadedFont, GoogleFontsError> {
    let cache_path = cache_dir
        .join("google-fonts")
        .join(format!("{}.ttf", artifact.git_blob_sha));
    if let Some(bytes) = read_verified_cache(
        &cache_path,
        artifact.size_bytes,
        &artifact.git_blob_sha,
        MAX_FONT_BYTES,
    )
    .await?
    {
        let metadata = validate_font(&bytes).map_err(|_| GoogleFontsError::FontValidation)?;
        return Ok(DownloadedFont {
            artifact: artifact.clone(),
            bytes,
            metadata,
        });
    }

    let bytes = download_verified(
        &artifact.download_url,
        artifact.size_bytes,
        &artifact.git_blob_sha,
        MAX_FONT_BYTES,
    )
    .await?;
    let metadata = validate_font(&bytes).map_err(|_| GoogleFontsError::FontValidation)?;
    write_cache_file(cache_path, bytes.clone()).await?;

    Ok(DownloadedFont {
        artifact: artifact.clone(),
        bytes,
        metadata,
    })
}

async fn preserve_license(
    family: &GoogleFontFamily,
    app_data_dir: &Path,
) -> Result<PathBuf, GoogleFontsError> {
    let manifest = bundled_manifest()?;
    let safe_license = family
        .license
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let path = app_data_dir
        .join("licenses")
        .join("google-fonts")
        .join(format!(
            "{}-{}-{safe_license}.txt",
            family.id.trim_start_matches("gf:").replace(':', "-"),
            &manifest.source_commit[..12]
        ));
    if read_verified_cache(
        &path,
        family.license_size_bytes,
        &family.license_git_blob_sha,
        MAX_LICENSE_BYTES,
    )
    .await?
    .is_some()
    {
        return Ok(path);
    }
    let bytes = download_verified(
        &family.license_url,
        family.license_size_bytes,
        &family.license_git_blob_sha,
        MAX_LICENSE_BYTES,
    )
    .await?;
    std::str::from_utf8(&bytes).map_err(|_| GoogleFontsError::Integrity)?;
    write_cache_file(path.clone(), bytes).await?;
    Ok(path)
}

async fn read_verified_cache(
    path: &Path,
    expected_size: u64,
    expected_sha: &str,
    maximum_size: u64,
) -> Result<Option<Vec<u8>>, GoogleFontsError> {
    let path = path.to_path_buf();
    let read_path = path.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || std::fs::read(read_path))
        .await
        .map_err(|_| GoogleFontsError::Download)?;
    match bytes {
        Ok(bytes)
            if u64::try_from(bytes.len()).unwrap_or(u64::MAX) == expected_size
                && expected_size <= maximum_size
                && git_blob_sha(&bytes).eq_ignore_ascii_case(expected_sha) =>
        {
            Ok(Some(bytes))
        }
        Ok(_) => {
            let _ = tauri::async_runtime::spawn_blocking(move || std::fs::remove_file(path)).await;
            Ok(None)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(GoogleFontsError::Download),
    }
}

async fn download_verified(
    url: &str,
    expected_size: u64,
    expected_sha: &str,
    maximum_size: u64,
) -> Result<Vec<u8>, GoogleFontsError> {
    if expected_size == 0 || expected_size > maximum_size {
        return Err(GoogleFontsError::Integrity);
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| GoogleFontsError::Download)?;
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "FontNest/0.1")
        .send()
        .await
        .map_err(|_| GoogleFontsError::Download)?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|length| length > maximum_size)
    {
        return Err(GoogleFontsError::Download);
    }

    let mut stream = response.bytes_stream();
    let mut bytes = Vec::with_capacity(usize::try_from(expected_size).unwrap_or_default());
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| GoogleFontsError::Download)?;
        if u64::try_from(bytes.len() + chunk.len()).unwrap_or(u64::MAX) > maximum_size {
            return Err(GoogleFontsError::Integrity);
        }
        bytes.extend_from_slice(&chunk);
    }
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != expected_size
        || !git_blob_sha(&bytes).eq_ignore_ascii_case(expected_sha)
    {
        return Err(GoogleFontsError::Integrity);
    }
    Ok(bytes)
}

async fn write_cache_file(path: PathBuf, bytes: Vec<u8>) -> Result<(), GoogleFontsError> {
    tauri::async_runtime::spawn_blocking(move || {
        let parent = path.parent().ok_or(GoogleFontsError::Download)?;
        std::fs::create_dir_all(parent).map_err(|_| GoogleFontsError::Download)?;
        if path.exists() {
            return Ok(());
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp = parent.join(format!(".fontnest-{}-{nonce}.tmp", std::process::id()));
        std::fs::write(&temp, bytes).map_err(|_| GoogleFontsError::Download)?;
        match std::fs::rename(&temp, &path) {
            Ok(()) => Ok(()),
            Err(_) if path.exists() => {
                let _ = std::fs::remove_file(temp);
                Ok(())
            }
            Err(_) => {
                let _ = std::fs::remove_file(temp);
                Err(GoogleFontsError::Download)
            }
        }
    })
    .await
    .map_err(|_| GoogleFontsError::Download)?
}

fn install_batch(
    downloads: Vec<DownloadedFont>,
) -> Result<Vec<CompletedInstallation>, GoogleFontsError> {
    let mut completed = Vec::with_capacity(downloads.len());
    for download in downloads {
        match install_user_font(
            &download.bytes,
            &download.artifact.file_name,
            &download.artifact.git_blob_sha,
            &download.metadata,
        ) {
            Ok(platform) => completed.push(CompletedInstallation {
                artifact: download.artifact,
                platform,
            }),
            Err(error) => {
                for installation in completed.iter().rev() {
                    let _ = rollback_user_font(&installation.platform);
                }
                return Err(map_platform_error(&error));
            }
        }
    }
    Ok(completed)
}

fn map_platform_error(_error: &FontPlatformError) -> GoogleFontsError {
    #[cfg(not(windows))]
    if matches!(_error, FontPlatformError::UnsupportedPlatform) {
        return GoogleFontsError::UnsupportedPlatform;
    }
    GoogleFontsError::Platform
}

fn parse_manifest(json: &str) -> Result<GoogleFontsManifest, String> {
    let manifest: GoogleFontsManifest =
        serde_json::from_str(json).map_err(|_| "invalid_manifest".to_owned())?;

    if manifest.schema_version != MANIFEST_SCHEMA_VERSION
        || manifest.generated_at.trim().is_empty()
        || manifest.snapshot.trim().is_empty()
        || !is_hex_sha(&manifest.source_commit)
        || manifest.families.is_empty()
    {
        return Err("invalid_manifest".to_owned());
    }

    let mut family_ids = HashSet::new();
    let mut artifact_ids = HashSet::new();

    for family in &manifest.families {
        if !family.id.starts_with("gf:")
            || !is_safe_id(&family.id)
            || family.family.trim().is_empty()
            || family.category.trim().is_empty()
            || family.subsets.iter().any(|subset| subset.trim().is_empty())
            || family.license.trim().is_empty()
            || family.last_modified.trim().is_empty()
            || family.version.trim().is_empty()
            || family.artifacts.is_empty()
            || !family_ids.insert(family.id.as_str())
            || !is_hex_sha(&family.license_git_blob_sha)
            || family.license_size_bytes == 0
            || family.license_size_bytes > 256 * 1024
            || !is_trusted_repository_url(&family.license_url, &manifest.source_commit)
        {
            return Err("invalid_manifest".to_owned());
        }

        let mut preview_found = false;
        for artifact in &family.artifacts {
            if artifact.id == family.preview_artifact_id {
                preview_found = true;
            }
            if !artifact.id.starts_with(&format!("{}:", family.id))
                || !is_safe_id(&artifact.id)
                || artifact.file_name.trim().is_empty()
                || artifact.style.trim().is_empty()
                || artifact.format != "TrueType"
                || !artifact.file_name.to_ascii_lowercase().ends_with(".ttf")
                || !is_hex_sha(&artifact.git_blob_sha)
                || artifact.size_bytes == 0
                || artifact.size_bytes > MAX_FONT_BYTES
                || !artifact_ids.insert(artifact.id.as_str())
                || !is_trusted_repository_url(&artifact.download_url, &manifest.source_commit)
            {
                return Err("invalid_manifest".to_owned());
            }
        }

        if !preview_found {
            return Err("invalid_manifest".to_owned());
        }
    }

    Ok(manifest)
}

fn catalogue_page(
    manifest: &GoogleFontsManifest,
    request: &GoogleFontPageRequest,
) -> GoogleFontPage {
    let query_terms = request
        .query
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    let category = request.category.trim().to_lowercase();
    let filtered = manifest
        .families
        .iter()
        .filter(|family| {
            let searchable = format!(
                "{} {} {} {}",
                family.family,
                family.category,
                family.subsets.join(" "),
                family.license
            )
            .to_lowercase();
            let category_matches = category.is_empty()
                || category == "all"
                || family.category.eq_ignore_ascii_case(&category);
            category_matches && query_terms.iter().all(|term| searchable.contains(term))
        })
        .collect::<Vec<_>>();
    let total = filtered.len();
    let limit = if request.limit == 0 {
        60
    } else {
        request.limit.min(MAX_CATALOGUE_PAGE_SIZE)
    };
    let offset = request.offset.min(total);
    let family_ids = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|family| family.id.clone())
        .collect();

    GoogleFontPage {
        family_ids,
        total,
        offset,
        limit,
    }
}

fn selected_artifacts(
    manifest: &GoogleFontsManifest,
    family_id: &str,
    artifact_ids: &[String],
) -> Result<Vec<GoogleFontArtifact>, String> {
    if artifact_ids.is_empty() || artifact_ids.len() > MAX_INSTALL_ARTIFACTS {
        return Err("invalid_artifact_selection".to_owned());
    }

    let family = manifest
        .families
        .iter()
        .find(|family| family.id == family_id)
        .ok_or_else(|| "invalid_artifact_selection".to_owned())?;
    let unique_ids = artifact_ids.iter().collect::<HashSet<_>>();
    if unique_ids.len() != artifact_ids.len() {
        return Err("invalid_artifact_selection".to_owned());
    }

    artifact_ids
        .iter()
        .map(|artifact_id| {
            family
                .artifacts
                .iter()
                .find(|artifact| artifact.id == *artifact_id)
                .cloned()
                .ok_or_else(|| "invalid_artifact_selection".to_owned())
        })
        .collect()
}

fn git_blob_sha(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()).as_bytes());
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(40), |mut output, byte| {
            write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        })
}

fn is_hex_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b':' | b'-')
        })
}

fn is_trusted_repository_url(value: &str, source_commit: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && url.host_str() == Some("raw.githubusercontent.com")
        && url.query().is_none()
        && url.fragment().is_none()
        && url
            .path()
            .starts_with(&format!("/google/fonts/{source_commit}/"))
}

#[cfg(test)]
mod tests {
    use super::{
        GoogleFontPageRequest, catalogue_page, git_blob_sha, parse_manifest, selected_artifacts,
    };

    const MANIFEST: &str = r#"
    {
      "schemaVersion": 1,
      "generatedAt": "2026-07-18T00:00:00.000Z",
      "sourceCommit": "0123456789abcdef0123456789abcdef01234567",
      "snapshot": "fixture",
      "families": [
        {
          "id": "gf:inter",
          "family": "Inter",
          "category": "sans-serif",
          "subsets": ["latin"],
          "license": "OFL-1.1",
          "licenseUrl": "https://raw.githubusercontent.com/google/fonts/0123456789abcdef0123456789abcdef01234567/ofl/inter/OFL.txt",
          "licenseGitBlobSha": "21f6aff961064c2e429f570995e446bcdd555422",
          "licenseSizeBytes": 4377,
          "lastModified": "2026-07-18",
          "version": "fixture",
          "previewArtifactId": "gf:inter:regular",
          "artifacts": [
            {
              "id": "gf:inter:regular",
              "fileName": "Inter.ttf",
              "style": "Variable",
              "format": "TrueType",
              "downloadUrl": "https://raw.githubusercontent.com/google/fonts/0123456789abcdef0123456789abcdef01234567/ofl/inter/Inter.ttf",
              "gitBlobSha": "4c20c9e8d7c89c5f0f6f7b4ea989b7e7f4e6c8b2",
              "sizeBytes": 4
            }
          ]
        },
        {
          "id": "gf:source-serif-4",
          "family": "Source Serif 4",
          "category": "serif",
          "subsets": ["latin"],
          "license": "OFL-1.1",
          "licenseUrl": "https://raw.githubusercontent.com/google/fonts/0123456789abcdef0123456789abcdef01234567/ofl/sourceserif4/OFL.txt",
          "licenseGitBlobSha": "318074ec9d43c936a56ee1dae92999f4df073ce8",
          "licenseSizeBytes": 4400,
          "lastModified": "2026-07-18",
          "version": "fixture",
          "previewArtifactId": "gf:source-serif-4:regular",
          "artifacts": [
            {
              "id": "gf:source-serif-4:regular",
              "fileName": "SourceSerif4.ttf",
              "style": "Variable",
              "format": "TrueType",
              "downloadUrl": "https://raw.githubusercontent.com/google/fonts/0123456789abcdef0123456789abcdef01234567/ofl/sourceserif4/SourceSerif4.ttf",
              "gitBlobSha": "6c20c9e8d7c89c5f0f6f7b4ea989b7e7f4e6c8b2",
              "sizeBytes": 4
            }
          ]
        }
      ]
    }
    "#;

    #[test]
    fn bundled_manifest_is_validated_before_use() {
        let manifest = parse_manifest(MANIFEST).expect("a trusted manifest");

        assert_eq!(manifest.source_commit.len(), 40);
        assert_eq!(manifest.snapshot, "fixture");
        assert_eq!(manifest.families.len(), 2);
    }

    #[test]
    fn catalogue_search_is_bounded_and_category_aware() {
        let manifest = parse_manifest(MANIFEST).expect("a trusted manifest");
        let page = catalogue_page(
            &manifest,
            &GoogleFontPageRequest {
                query: "source".to_owned(),
                category: "serif".to_owned(),
                offset: 0,
                limit: 500,
            },
        );

        assert_eq!(page.family_ids, vec!["gf:source-serif-4"]);
        assert_eq!(page.total, 1);
        assert_eq!(page.limit, 100);
    }

    #[test]
    fn install_selection_cannot_cross_family_boundaries() {
        let manifest = parse_manifest(MANIFEST).expect("a trusted manifest");
        let error = selected_artifacts(
            &manifest,
            "gf:inter",
            &["gf:source-serif-4:regular".to_owned()],
        )
        .expect_err("an artifact from another family must be rejected");

        assert_eq!(error, "invalid_artifact_selection");
    }

    #[test]
    fn downloaded_bytes_are_verified_as_git_blobs() {
        assert_eq!(
            git_blob_sha(b"test"),
            "30d74d258442c7c65512eafab474568dd706c430"
        );
    }
}
