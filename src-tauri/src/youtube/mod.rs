pub mod channel;
pub mod thumbnail;
pub mod upload;

use std::time::Duration;

use serde::Deserialize;

use crate::error::{AppError, AppResult};

pub const API_BASE: &str = "https://www.googleapis.com/youtube/v3";
pub const UPLOAD_BASE: &str = "https://www.googleapis.com/upload/youtube/v3";

pub fn http_client() -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        // CRITICAL: the resumable protocol answers "chunk accepted, send more" with
        // HTTP 308. reqwest treats 3xx as a redirect by default, so it must not follow them.
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("Yutub/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        // Per-read timeout, not total: a 4 GB upload may take hours, but a socket
        // that has gone silent for 2 minutes is dead.
        .read_timeout(Duration::from_secs(120))
        .build()?)
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}
#[derive(Deserialize)]
struct ErrorBody {
    message: Option<String>,
    #[serde(default)]
    errors: Vec<ErrorItem>,
}
#[derive(Deserialize)]
struct ErrorItem {
    reason: Option<String>,
}

/// Turn a non-2xx Google API response into `AppError::Api`, keeping the machine-readable
/// `reason` (quotaExceeded, uploadLimitExceeded, …) because retry decisions depend on it.
pub async fn api_error(resp: reqwest::Response) -> AppError {
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    match serde_json::from_str::<ErrorEnvelope>(&body) {
        Ok(env) => AppError::Api {
            status,
            reason: env
                .error
                .errors
                .into_iter()
                .find_map(|e| e.reason)
                .unwrap_or_else(|| "unknown".into()),
            message: env.error.message.unwrap_or_default(),
        },
        Err(_) => AppError::Api {
            status,
            reason: "unknown".into(),
            message: body.chars().take(300).collect(),
        },
    }
}
