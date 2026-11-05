//! YouTube resumable upload protocol.
//! https://developers.google.com/youtube/v3/guides/using_resumable_upload_protocol
//!
//! 1. POST metadata → `Location` header = session URI (valid ~1 week).
//! 2. PUT byte ranges to the session URI. `308` = partial, `Range` header says what's committed.
//! 3. On any interruption, PUT an empty body with `Content-Range: bytes */TOTAL` to learn the offset.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::StatusCode;
use reqwest::header::{CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, LOCATION, RANGE};
use serde::Deserialize;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::api_error;
use crate::error::{AppError, AppResult};
use crate::queue::job::Metadata;

/// Google requires every non-final chunk to be a multiple of 256 KiB.
pub const CHUNK_ALIGN: u64 = 256 * 1024;
/// 8 MiB is a good middle ground. Bigger chunks waste less on round trips but lose
/// more work when a chunk fails. Memory stays flat at one chunk regardless of file size.
pub const CHUNK_SIZE: u64 = 32 * CHUNK_ALIGN;
/// The body is fed to hyper in slices this big, so progress moves within a chunk.
const SLICE: usize = 256 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum ChunkResult {
    /// Server has committed bytes `[0, n)`.
    Incomplete(u64),
    /// Upload finished; YouTube video ID.
    Complete(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Recovery {
    /// Transient (5xx, dropped connection): back off, query offset, continue.
    Retry,
    /// 401: access token expired or revoked mid-upload; refresh once, then continue.
    RefreshToken,
    /// 404/410 on the session: it expired, so start a fresh session from byte 0.
    RestartSession,
    /// Quota, bad metadata, forbidden: retrying won't help.
    Fatal,
}

pub fn recovery_for(err: &AppError) -> Recovery {
    match err {
        AppError::Http(e) if e.is_timeout() || e.is_connect() || e.is_request() || e.is_body() => Recovery::Retry,
        AppError::Http(_) => Recovery::Retry,
        AppError::Io(_) => Recovery::Fatal, // local file vanished/changed; not a network issue
        AppError::Api { status, reason, .. } => match (*status, reason.as_str()) {
            (500 | 502 | 503 | 504, _) => Recovery::Retry,
            (429, _) => Recovery::Retry,
            (401, _) => Recovery::RefreshToken,
            (404 | 410, _) => Recovery::RestartSession,
            // Some 403s are rate limits in disguise; the quota ones are not.
            (403, "rateLimitExceeded" | "userRateLimitExceeded") => Recovery::Retry,
            _ => Recovery::Fatal,
        },
        _ => Recovery::Fatal,
    }
}

/// `Range: bytes=0-1048575` → next offset `1048576`. No header → nothing committed yet.
pub fn parse_range(h: Option<&str>) -> u64 {
    h.and_then(|v| v.trim().strip_prefix("bytes="))
        .and_then(|r| r.split('-').nth(1))
        .and_then(|end| end.trim().parse::<u64>().ok())
        .map(|end| end + 1)
        .unwrap_or(0)
}

/// Exclusive end of the chunk starting at `offset`. Only the final chunk may be unaligned.
pub fn chunk_end(offset: u64, total: u64) -> u64 {
    // The server may commit an unaligned prefix; snap back to an aligned boundary
    // so every chunk after this one is aligned again.
    let next = (offset / CHUNK_ALIGN) * CHUNK_ALIGN + CHUNK_SIZE;
    next.min(total)
}

pub fn video_body(meta: &Metadata) -> serde_json::Value {
    json!({
        "snippet": {
            "title": meta.title.trim(),
            "description": meta.description,
            "tags": meta.tags,
            "categoryId": meta.category_id,
        },
        "status": {
            "privacyStatus": meta.privacy,
            "selfDeclaredMadeForKids": meta.made_for_kids,
        }
    })
}

/// Step 1: create the session. `base` is injectable so tests can point at a mock server.
pub async fn start_session(
    http: &reqwest::Client,
    base: &str,
    token: &str,
    meta: &Metadata,
    total: u64,
    mime: &str,
) -> AppResult<String> {
    let resp = http
        .post(format!("{base}/videos"))
        .query(&[("uploadType", "resumable"), ("part", "snippet,status")])
        .bearer_auth(token)
        .header("X-Upload-Content-Length", total)
        .header("X-Upload-Content-Type", mime)
        .json(&video_body(meta))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(api_error(resp).await);
    }
    resp.headers()
        .get(LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .ok_or_else(|| AppError::msg("Upload session created without a Location header"))
}

#[derive(Deserialize)]
struct VideoResource {
    id: String,
}

async fn interpret(resp: reqwest::Response) -> AppResult<ChunkResult> {
    match resp.status() {
        StatusCode::PERMANENT_REDIRECT => Ok(ChunkResult::Incomplete(parse_range(
            resp.headers().get(RANGE).and_then(|v| v.to_str().ok()),
        ))),
        s if s.is_success() => Ok(ChunkResult::Complete(resp.json::<VideoResource>().await?.id)),
        _ => Err(api_error(resp).await),
    }
}

/// Step 3: ask the server how many bytes it has.
pub async fn query_offset(http: &reqwest::Client, session: &str, token: &str, total: u64) -> AppResult<ChunkResult> {
    let resp = http
        .put(session)
        .bearer_auth(token)
        .header(CONTENT_LENGTH, 0)
        .header(CONTENT_RANGE, format!("bytes */{total}"))
        .send()
        .await?;
    interpret(resp).await
}

/// Step 2: send `[offset, chunk_end)` of the file. `sent` is bumped as hyper pulls each
/// slice, so a progress ticker can read it lock-free without touching this future.
pub async fn put_chunk(
    http: &reqwest::Client,
    session: &str,
    token: &str,
    path: &Path,
    offset: u64,
    total: u64,
    sent: Arc<AtomicU64>,
) -> AppResult<ChunkResult> {
    let end = chunk_end(offset, total);
    let len = end - offset;

    let mut f = tokio::fs::File::open(path).await?;
    f.seek(std::io::SeekFrom::Start(offset)).await?;
    let mut buf = vec![0u8; len as usize];
    f.read_exact(&mut buf).await?;
    let data = Bytes::from(buf);

    sent.store(offset, Ordering::Relaxed);
    // Bytes::slice is a refcount bump, not a copy: one 8 MiB allocation per chunk, total.
    let slices: Vec<Bytes> = (0..data.len())
        .step_by(SLICE)
        .map(|i| data.slice(i..(i + SLICE).min(data.len())))
        .collect();
    let counter = sent.clone();
    let stream = futures_util::stream::iter(slices).map(move |b| {
        counter.fetch_add(b.len() as u64, Ordering::Relaxed);
        Ok::<_, std::io::Error>(b)
    });

    let resp = http
        .put(session)
        .bearer_auth(token)
        // Explicit Content-Length stops hyper from switching a streamed body to chunked
        // transfer encoding, which Google's upload endpoint rejects.
        .header(CONTENT_LENGTH, len)
        .header(CONTENT_RANGE, format!("bytes {offset}-{}/{total}", end - 1))
        .header(CONTENT_TYPE, "application/octet-stream")
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await?;
    interpret(resp).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::job::{Metadata, Privacy};
    use httpmock::prelude::*;

    #[test]
    fn range_parsing() {
        assert_eq!(parse_range(None), 0);
        assert_eq!(parse_range(Some("bytes=0-0")), 1);
        assert_eq!(parse_range(Some("bytes=0-8388607")), 8_388_608);
        assert_eq!(parse_range(Some("garbage")), 0);
    }

    #[test]
    fn chunk_boundaries() {
        assert_eq!(chunk_end(0, 100), 100); // tiny file = one unaligned final chunk
        assert_eq!(chunk_end(0, CHUNK_SIZE * 3), CHUNK_SIZE);
        assert_eq!(chunk_end(CHUNK_SIZE, CHUNK_SIZE * 3), CHUNK_SIZE * 2);
        assert_eq!(chunk_end(CHUNK_SIZE * 2, CHUNK_SIZE * 2 + 5), CHUNK_SIZE * 2 + 5);
        // Server committed an unaligned prefix: the next chunk ends back on an aligned boundary.
        let end = chunk_end(CHUNK_ALIGN + 10, CHUNK_SIZE * 4);
        assert_eq!(end % CHUNK_ALIGN, 0);
        assert!(end > CHUNK_ALIGN + 10);
    }

    #[test]
    fn recovery_table() {
        let api = |status, reason: &str| AppError::Api { status, reason: reason.into(), message: String::new() };
        assert_eq!(recovery_for(&api(503, "backendError")), Recovery::Retry);
        assert_eq!(recovery_for(&api(401, "authError")), Recovery::RefreshToken);
        assert_eq!(recovery_for(&api(404, "notFound")), Recovery::RestartSession);
        assert_eq!(recovery_for(&api(403, "quotaExceeded")), Recovery::Fatal);
        assert_eq!(recovery_for(&api(403, "uploadLimitExceeded")), Recovery::Fatal);
        assert_eq!(recovery_for(&api(403, "userRateLimitExceeded")), Recovery::Retry);
        assert_eq!(recovery_for(&api(400, "invalidTitle")), Recovery::Fatal);
    }

    fn meta() -> Metadata {
        Metadata {
            title: "Test".into(),
            description: String::new(),
            tags: vec![],
            category_id: "22".into(),
            privacy: Privacy::Private,
            made_for_kids: false,
        }
    }

    /// Full protocol against a fake server: start → 308 → query → 201.
    #[tokio::test]
    async fn resumable_round_trip() {
        let server = MockServer::start_async().await;
        let session_url = server.url("/session/abc");
        let total = CHUNK_SIZE + 1000; // two chunks: one aligned, one tail

        let start = server
            .mock_async(|when, then| {
                when.method(POST).path("/videos").query_param("uploadType", "resumable")
                    .header("x-upload-content-length", total.to_string());
                then.status(200).header("Location", session_url.as_str());
            })
            .await;
        let first = server
            .mock_async(|when, then| {
                when.method(PUT).path("/session/abc")
                    .header("content-range", format!("bytes 0-{}/{total}", CHUNK_SIZE - 1));
                then.status(308).header("Range", format!("bytes=0-{}", CHUNK_SIZE - 1));
            })
            .await;
        let query = server
            .mock_async(|when, then| {
                when.method(PUT).path("/session/abc").header("content-range", format!("bytes */{total}"));
                then.status(308).header("Range", format!("bytes=0-{}", CHUNK_SIZE - 1));
            })
            .await;
        let last = server
            .mock_async(|when, then| {
                when.method(PUT).path("/session/abc")
                    .header("content-range", format!("bytes {CHUNK_SIZE}-{}/{total}", total - 1));
                then.status(201).json_body(serde_json::json!({ "id": "vid123" }));
            })
            .await;

        let dir = std::env::temp_dir().join(format!("yutub-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("v.mp4");
        std::fs::write(&file, vec![7u8; total as usize]).unwrap();

        let http = crate::youtube::http_client().unwrap();
        let sent = Arc::new(AtomicU64::new(0));
        let session = start_session(&http, &server.base_url(), "tok", &meta(), total, "video/mp4").await.unwrap();
        assert_eq!(session, session_url);

        let r1 = put_chunk(&http, &session, "tok", &file, 0, total, sent.clone()).await.unwrap();
        assert_eq!(r1, ChunkResult::Incomplete(CHUNK_SIZE));
        assert_eq!(sent.load(Ordering::Relaxed), CHUNK_SIZE);

        // Simulate a crash/resume: ask the server where we are.
        let q = query_offset(&http, &session, "tok", total).await.unwrap();
        assert_eq!(q, ChunkResult::Incomplete(CHUNK_SIZE));

        let r2 = put_chunk(&http, &session, "tok", &file, CHUNK_SIZE, total, sent.clone()).await.unwrap();
        assert_eq!(r2, ChunkResult::Complete("vid123".into()));

        start.assert_async().await;
        first.assert_async().await;
        query.assert_async().await;
        last.assert_async().await;
        let _ = std::fs::remove_dir_all(dir);
    }
}
