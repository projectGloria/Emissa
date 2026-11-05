use serde::{Deserialize, Serialize};

use super::{API_BASE, api_error};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    pub title: String,
    pub avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct ListResp {
    #[serde(default)]
    items: Vec<Item>,
}
#[derive(Deserialize)]
struct Item {
    id: String,
    snippet: Snippet,
}
#[derive(Deserialize)]
struct Snippet {
    title: String,
    thumbnails: Option<Thumbs>,
}
#[derive(Deserialize)]
struct Thumbs {
    default: Option<Thumb>,
    medium: Option<Thumb>,
}
#[derive(Deserialize)]
struct Thumb {
    url: String,
}

pub async fn mine(http: &reqwest::Client, token: &str) -> AppResult<Channel> {
    let resp = http
        .get(format!("{API_BASE}/channels"))
        .query(&[("part", "snippet"), ("mine", "true")])
        .bearer_auth(token)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(api_error(resp).await);
    }
    let list: ListResp = resp.json().await?;
    let item = list.items.into_iter().next().ok_or_else(|| {
        AppError::msg("This Google account has no YouTube channel. Create one at youtube.com first.")
    })?;
    let avatar_url = item
        .snippet
        .thumbnails
        .and_then(|t| t.medium.or(t.default))
        .map(|t| t.url);
    Ok(Channel {
        id: item.id,
        title: item.snippet.title,
        avatar_url,
    })
}
