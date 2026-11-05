use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};

const FILE_NAME: &str = "google_client.json";

/// The subset of Google's "Desktop app" client JSON we need.
/// Google ships it as `{ "installed": { ... } }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCreds {
    pub client_id: String,
    // Desktop clients aren't confidential (the secret ships in every binary), but Google
    // still requires it on the token endpoint. PKCE is what actually protects the code exchange.
    pub client_secret: String,
    #[serde(default = "default_auth_uri")]
    pub auth_uri: String,
    #[serde(default = "default_token_uri")]
    pub token_uri: String,
}

fn default_auth_uri() -> String {
    "https://accounts.google.com/o/oauth2/v2/auth".into()
}
fn default_token_uri() -> String {
    "https://oauth2.googleapis.com/token".into()
}

#[derive(Deserialize)]
struct GoogleClientFile {
    installed: Option<ClientCreds>,
    web: Option<serde_json::Value>,
}

fn config_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_config_dir()?.join(FILE_NAME))
}

pub fn parse_client_json(raw: &str) -> AppResult<ClientCreds> {
    let f: GoogleClientFile = serde_json::from_str(raw)?;
    match (f.installed, f.web) {
        (Some(c), _) if !c.client_id.is_empty() => Ok(c),
        (None, Some(_)) => Err(AppError::msg(
            "That's a \"Web application\" client. Create an OAuth client of type \"Desktop app\" instead.",
        )),
        _ => Err(AppError::msg(
            "Not a Google OAuth client file (missing the \"installed\" section).",
        )),
    }
}

pub fn load(app: &AppHandle) -> AppResult<Option<ClientCreds>> {
    let p = config_path(app)?;
    if !p.exists() {
        return Ok(None);
    }
    Ok(Some(parse_client_json(&std::fs::read_to_string(p)?)?))
}

/// Validate first, then copy verbatim into the app config dir, so a bad file never replaces a good one.
pub fn import(app: &AppHandle, src: &Path) -> AppResult<ClientCreds> {
    let raw = std::fs::read_to_string(src)?;
    let creds = parse_client_json(&raw)?;
    let dst = config_path(app)?;
    if let Some(dir) = dst.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(dst, raw)?;
    Ok(creds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_desktop_client() {
        let c = parse_client_json(
            r#"{"installed":{"client_id":"abc.apps.googleusercontent.com","client_secret":"s","redirect_uris":["http://localhost"]}}"#,
        )
        .unwrap();
        assert_eq!(c.client_id, "abc.apps.googleusercontent.com");
        assert_eq!(c.token_uri, "https://oauth2.googleapis.com/token");
    }

    #[test]
    fn rejects_web_client() {
        let e = parse_client_json(r#"{"web":{"client_id":"x"}}"#).unwrap_err();
        assert!(e.to_string().contains("Desktop app"));
    }
}
