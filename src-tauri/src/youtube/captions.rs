//! captions.insert: costs 400 quota units (regular pool) and needs the youtube.force-ssl scope.

use std::path::Path;

use serde_json::json;

use super::{UPLOAD_BASE, api_error};
use crate::error::{AppError, AppResult};
use crate::queue::job::Caption;

pub const MAX_BYTES: u64 = 100 * 1024 * 1024;
/// Formats YouTube accepts for caption uploads.
pub const EXTS: &[&str] = &[
    "srt", "vtt", "sbv", "sub", "ttml", "dfxp", "scc", "smi", "sami", "lrc", "stl", "cap",
];

pub fn validate(path: &Path) -> AppResult<()> {
    let ok = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTS.contains(&e.to_ascii_lowercase().as_str()));
    if !ok {
        return Err(AppError::msg("Unsupported subtitle format. Use .srt, .vtt, .sbv, .ttml, .scc, …"));
    }
    if std::fs::metadata(path)?.len() > MAX_BYTES {
        return Err(AppError::msg("Subtitle file must be 100 MB or smaller."));
    }
    Ok(())
}

/// Google wants `multipart/related` (JSON metadata part + media part), which reqwest's
/// multipart support doesn't do (it only builds `multipart/form-data`). So the body is
/// assembled by hand: the format is just boundaries and headers.
pub fn multipart_body(boundary: &str, metadata: &serde_json::Value, file: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(file.len() + 512);
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{metadata}\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

pub async fn insert(
    http: &reqwest::Client,
    base: &str,
    token: &str,
    video_id: &str,
    c: &Caption,
) -> AppResult<()> {
    let path = Path::new(&c.path);
    validate(path)?;
    let file = tokio::fs::read(path).await?;
    let meta = json!({
        "snippet": { "videoId": video_id, "language": c.language, "name": c.name, "isDraft": false }
    });
    // A UUID can't plausibly appear inside a subtitle file, so it's a safe boundary.
    let boundary = format!("emissa-{}", uuid::Uuid::new_v4().simple());
    let resp = http
        .post(format!("{base}/captions"))
        .query(&[("part", "snippet"), ("uploadType", "multipart")])
        .bearer_auth(token)
        .header(
            reqwest::header::CONTENT_TYPE,
            format!("multipart/related; boundary={boundary}"),
        )
        .body(multipart_body(&boundary, &meta, &file))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(api_error(resp).await);
    }
    Ok(())
}

pub async fn insert_default(http: &reqwest::Client, token: &str, video_id: &str, c: &Caption) -> AppResult<()> {
    insert(http, UPLOAD_BASE, token, video_id, c).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[test]
    fn body_shape() {
        let b = multipart_body("B", &json!({"a":1}), b"1\n00:00:00,000 --> 00:00:01,000\nhi\n");
        let s = String::from_utf8(b).unwrap();
        assert!(s.starts_with("--B\r\nContent-Type: application/json"));
        assert!(s.contains("{\"a\":1}\r\n--B\r\nContent-Type: application/octet-stream\r\n\r\n1\n"));
        assert!(s.ends_with("\r\n--B--\r\n"));
    }

    #[test]
    fn formats() {
        let dir = std::env::temp_dir().join(format!("emissa-cap-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let srt = dir.join("a.SRT");
        std::fs::write(&srt, "x").unwrap();
        assert!(validate(&srt).is_ok());
        let txt = dir.join("a.txt");
        std::fs::write(&txt, "x").unwrap();
        assert!(validate(&txt).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn posts_multipart_related() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/captions")
                    .query_param("uploadType", "multipart")
                    .header_exists("content-type")
                    .body_includes("\"videoId\":\"vid1\"")
                    .body_includes("00:00:01,000");
                then.status(200).json_body(json!({"id": "cap1"}));
            })
            .await;
        let dir = std::env::temp_dir().join(format!("emissa-cap-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("en.srt");
        std::fs::write(&f, "1\n00:00:00,000 --> 00:00:01,000\nhi\n").unwrap();
        let c = Caption { path: f.to_string_lossy().into(), language: "en".into(), name: "English".into() };
        let http = crate::youtube::http_client().unwrap();
        insert(&http, &server.base_url(), "tok", "vid1", &c).await.unwrap();
        m.assert_async().await;
        let _ = std::fs::remove_dir_all(dir);
    }
}
