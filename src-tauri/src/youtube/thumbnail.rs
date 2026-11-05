use std::path::Path;

use super::{UPLOAD_BASE, api_error};
use crate::error::{AppError, AppResult};

pub const MAX_BYTES: u64 = 2 * 1024 * 1024;

pub fn validate(path: &Path) -> AppResult<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(ext.as_deref(), Some("jpg" | "jpeg" | "png")) {
        return Err(AppError::msg("Thumbnail must be a JPG or PNG."));
    }
    if std::fs::metadata(path)?.len() > MAX_BYTES {
        return Err(AppError::msg("Thumbnail must be 2 MB or smaller."));
    }
    Ok(())
}

/// Thumbnails are at most 2 MB, so a single media upload is fine (no resumable session).
pub async fn set(http: &reqwest::Client, token: &str, video_id: &str, path: &Path) -> AppResult<()> {
    validate(path)?;
    let bytes = tokio::fs::read(path).await?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let resp = http
        .post(format!("{UPLOAD_BASE}/thumbnails/set"))
        .query(&[("videoId", video_id), ("uploadType", "media")])
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, mime.as_ref())
        .body(bytes)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(api_error(resp).await);
    }
    Ok(())
}
