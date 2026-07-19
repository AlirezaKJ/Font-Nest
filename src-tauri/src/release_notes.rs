use std::time::Duration;

use futures_util::StreamExt;

/// Raw CHANGELOG.md on the default branch. The application webview cannot reach GitHub directly
/// under the app content-security policy, so the Rust side fetches the live release notes.
const REMOTE_URL: &str = "https://raw.githubusercontent.com/AlirezaKJ/Font-Nest/main/CHANGELOG.md";

/// A ceiling far above any realistic changelog, used to reject an unexpectedly large response.
const MAX_BYTES: usize = 512 * 1024;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum ReleaseNotesError {
    #[error("the release notes request failed")]
    Fetch,
    #[error("the release notes response exceeded the size limit")]
    TooLarge,
}

/// Fetches the current CHANGELOG.md text from GitHub over HTTPS, capping the response size.
pub async fn fetch_changelog() -> Result<String, ReleaseNotesError> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(3))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|_| ReleaseNotesError::Fetch)?;

    let response = client
        .get(REMOTE_URL)
        .header(reqwest::header::USER_AGENT, "FontNest")
        .header(reqwest::header::ACCEPT, "text/plain")
        .send()
        .await
        .map_err(|_| ReleaseNotesError::Fetch)?;

    if !response.status().is_success() {
        return Err(ReleaseNotesError::Fetch);
    }
    if response
        .content_length()
        .is_some_and(|length| usize::try_from(length).unwrap_or(usize::MAX) > MAX_BYTES)
    {
        return Err(ReleaseNotesError::TooLarge);
    }

    let mut stream = response.bytes_stream();
    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ReleaseNotesError::Fetch)?;
        if bytes.len().saturating_add(chunk.len()) > MAX_BYTES {
            return Err(ReleaseNotesError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }

    String::from_utf8(bytes).map_err(|_| ReleaseNotesError::Fetch)
}

#[cfg(test)]
mod tests {
    use super::REMOTE_URL;

    #[test]
    fn remote_url_targets_the_repository_changelog_over_https() {
        assert!(REMOTE_URL.starts_with("https://"));
        assert!(REMOTE_URL.ends_with("/CHANGELOG.md"));
        assert!(REMOTE_URL.contains("AlirezaKJ/Font-Nest"));
    }
}
