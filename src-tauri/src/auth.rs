//! OAuth 2.0 for installed apps: loopback redirect + PKCE (RFC 8252).
//!
//! Hand-rolled on reqwest instead of the `oauth2` crate. The flow is ~150 lines,
//! and this avoids pinning a second reqwest version into the build.

use std::time::{Duration, Instant};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};

use crate::config::{self, ClientCreds};
use crate::error::{AppError, AppResult};

// Legacy ID from when the app was called Yutub, kept on purpose: the stored refresh token
// lives under this key, so renaming it would silently sign every existing user out.
const KEYRING_SERVICE: &str = "com.yutub.app";
const KEYRING_USER: &str = "google-refresh-token";
const SCOPE_UPLOAD: &str = "https://www.googleapis.com/auth/youtube.upload";
const SCOPE_READONLY: &str = "https://www.googleapis.com/auth/youtube.readonly";
/// Needed only for captions.insert. Optional: if the user unticks it, everything except
/// subtitles still works, so it's deliberately not in REQUIRED_SCOPES.
pub const SCOPE_FORCE_SSL: &str = "https://www.googleapis.com/auth/youtube.force-ssl";
/// (scope, what breaks without it). Used to tell the user exactly which checkbox they unticked.
const REQUIRED_SCOPES: &[(&str, &str)] = &[
    (SCOPE_UPLOAD, "Manage your YouTube videos"),
    (SCOPE_READONLY, "View your YouTube account"),
];
/// Refresh this long before actual expiry so a token never dies mid-chunk.
const EXPIRY_MARGIN: Duration = Duration::from_secs(60);
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

struct AccessToken {
    value: String,
    expires_at: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
    /// Space-separated scopes the user ACTUALLY granted. With granular consent, users can untick
    /// individual permissions, so this can be a subset of what we asked for.
    scope: Option<String>,
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
    error_description: Option<String>,
}

pub struct Auth {
    http: reqwest::Client,
    client: RwLock<Option<ClientCreds>>,
    // A tokio Mutex, held across the refresh `.await`, on purpose: concurrent callers
    // queue behind one refresh request instead of all hitting the token endpoint at once.
    token: Mutex<Option<AccessToken>>,
    /// Scopes Google says this grant actually has (refreshed with every token response).
    /// Kept so the UI can tell whether subtitle upload is possible without a doomed API call.
    granted: std::sync::Mutex<Option<String>>,
}

fn keyring_entry() -> AppResult<keyring::Entry> {
    Ok(keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?)
}

fn stored_refresh_token() -> AppResult<Option<String>> {
    match keyring_entry()?.get_password() {
        Ok(t) => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn clear_refresh_token() -> AppResult<()> {
    match keyring_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// PKCE verifier: 64 hex chars from two v4 UUIDs (OS CSPRNG via getrandom, ~244 bits).
/// Hex is inside the RFC 7636 unreserved charset, so no extra encoding is needed.
fn pkce_pair() -> (String, String) {
    let verifier = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

impl Auth {
    pub fn new(app: &AppHandle, http: reqwest::Client) -> AppResult<Self> {
        Ok(Self {
            http,
            client: RwLock::new(config::load(app)?),
            token: Mutex::new(None),
            granted: std::sync::Mutex::new(None),
        })
    }

    pub async fn set_client(&self, creds: ClientCreds) {
        *self.client.write().await = Some(creds);
        // A new client invalidates any tokens issued to the old one.
        *self.token.lock().await = None;
        let _ = clear_refresh_token();
    }

    pub async fn has_client(&self) -> bool {
        self.client.read().await.is_some()
    }

    pub fn is_signed_in(&self) -> AppResult<bool> {
        Ok(stored_refresh_token()?.is_some())
    }

    async fn creds(&self) -> AppResult<ClientCreds> {
        self.client.read().await.clone().ok_or(AppError::NoClient)
    }

    pub async fn sign_in(&self, app: &AppHandle) -> AppResult<()> {
        let creds = self.creds().await?;
        // Port 0 = the OS picks a free port. Google allows any port on loopback for Desktop clients.
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let redirect_uri = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
        let (verifier, challenge) = pkce_pair();
        let state = uuid::Uuid::new_v4().simple().to_string();

        let url = url::Url::parse_with_params(
            &creds.auth_uri,
            &[
                ("client_id", creds.client_id.as_str()),
                ("redirect_uri", redirect_uri.as_str()),
                ("response_type", "code"),
                ("scope", &format!("{SCOPE_UPLOAD} {SCOPE_READONLY} {SCOPE_FORCE_SSL}")),
                ("code_challenge", challenge.as_str()),
                ("code_challenge_method", "S256"),
                ("state", state.as_str()),
                // offline + consent: makes Google return a refresh_token every time,
                // not only on the very first grant.
                ("access_type", "offline"),
                ("prompt", "consent"),
            ],
        )
        .map_err(|e| AppError::msg(e.to_string()))?;

        // The system browser, not the webview: Google blocks OAuth in embedded webviews,
        // and the user's existing Google session and password manager live there anyway.
        app.opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|e| AppError::msg(format!("Could not open browser: {e}")))?;

        let code = tokio::time::timeout(SIGN_IN_TIMEOUT, accept_code(&listener, &state))
            .await
            .map_err(|_| AppError::msg("Sign-in timed out. Try again."))??;

        let resp = self
            .http
            .post(&creds.token_uri)
            .form(&[
                ("code", code.as_str()),
                ("client_id", creds.client_id.as_str()),
                ("client_secret", creds.client_secret.as_str()),
                ("redirect_uri", redirect_uri.as_str()),
                ("grant_type", "authorization_code"),
                ("code_verifier", verifier.as_str()),
            ])
            .send()
            .await?;
        let tok = parse_token_response(resp).await?;

        // Reject a partial grant now. Otherwise it fails later as an opaque 403 halfway through
        // the first upload, after the session has already cost quota.
        let missing = missing_scopes(tok.scope.as_deref().unwrap_or(""));
        if !missing.is_empty() {
            // Don't keep a half-useful grant around. Revoking it also makes Google show every
            // checkbox again on the next attempt.
            let _ = self
                .http
                .post("https://oauth2.googleapis.com/revoke")
                .form(&[("token", tok.access_token.as_str())])
                .send()
                .await;
            return Err(AppError::msg(format!(
                "Emissa needs every permission to work. Sign in again and keep these ticked: {}.",
                missing.join(", ")
            )));
        }

        let refresh = tok
            .refresh_token
            .clone()
            .ok_or_else(|| AppError::msg("Google did not return a refresh token."))?;
        keyring_entry()?.set_password(&refresh)?;
        self.set_granted(tok.scope.clone());
        self.store(tok).await;
        Ok(())
    }

    pub async fn sign_out(&self) -> AppResult<()> {
        if let Some(rt) = stored_refresh_token()? {
            // Best effort: revoking the refresh token also kills its access tokens server-side.
            let _ = self
                .http
                .post("https://oauth2.googleapis.com/revoke")
                .form(&[("token", rt.as_str())])
                .send()
                .await;
        }
        *self.token.lock().await = None;
        self.set_granted(None);
        clear_refresh_token()
    }

    fn set_granted(&self, scope: Option<String>) {
        *self.granted.lock().unwrap_or_else(|p| p.into_inner()) = scope;
    }

    /// Whether the current grant includes `scope`. Forces a token refresh if we haven't
    /// talked to Google since launch, because that response is where the scope list comes from.
    pub async fn has_scope(&self, scope: &str) -> AppResult<bool> {
        self.access_token().await?;
        let g = self.granted.lock().unwrap_or_else(|p| p.into_inner());
        Ok(g.as_deref().is_some_and(|s| s.split_whitespace().any(|x| x == scope)))
    }

    async fn store(&self, tok: TokenResponse) {
        *self.token.lock().await = Some(AccessToken {
            value: tok.access_token,
            expires_at: Instant::now() + Duration::from_secs(tok.expires_in),
        });
    }

    /// Drop the cached access token, e.g. after a 401, so the next call refreshes.
    pub async fn invalidate(&self) {
        *self.token.lock().await = None;
    }

    /// Every API call goes through here: returns the cached token, refreshing it if it's near expiry.
    pub async fn access_token(&self) -> AppResult<String> {
        let mut guard = self.token.lock().await;
        if let Some(t) = guard.as_ref() {
            if t.expires_at > Instant::now() + EXPIRY_MARGIN {
                return Ok(t.value.clone());
            }
        }
        let refresh = stored_refresh_token()?.ok_or(AppError::NotSignedIn)?;
        let creds = self.creds().await?;
        let resp = self
            .http
            .post(&creds.token_uri)
            .form(&[
                ("client_id", creds.client_id.as_str()),
                ("client_secret", creds.client_secret.as_str()),
                ("refresh_token", refresh.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await?;
        let tok = match parse_token_response(resp).await {
            Ok(t) => t,
            // invalid_grant = refresh token revoked or expired (7-day limit in "Testing"
            // consent mode). It's dead, so forget it and make the UI sign in again.
            Err(AppError::Api { reason, .. }) if reason == "invalid_grant" => {
                clear_refresh_token()?;
                return Err(AppError::NotSignedIn);
            }
            Err(e) => return Err(e),
        };
        let value = tok.access_token.clone();
        if tok.scope.is_some() {
            self.set_granted(tok.scope.clone());
        }
        *guard = Some(AccessToken {
            value: tok.access_token,
            expires_at: Instant::now() + Duration::from_secs(tok.expires_in),
        });
        Ok(value)
    }
}

/// Labels of required scopes absent from Google's space-separated `scope` string.
/// Exact token match, not substring: `youtube.upload` must not satisfy anything else by accident.
fn missing_scopes(granted: &str) -> Vec<&'static str> {
    let granted: std::collections::HashSet<&str> = granted.split_whitespace().collect();
    REQUIRED_SCOPES
        .iter()
        .filter(|(scope, _)| !granted.contains(scope))
        .map(|(_, label)| *label)
        .collect()
}

async fn parse_token_response(resp: reqwest::Response) -> AppResult<TokenResponse> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp.json().await?);
    }
    let body = resp.text().await.unwrap_or_default();
    let (reason, message) = serde_json::from_str::<TokenError>(&body)
        .map(|e| (e.error, e.error_description.unwrap_or_default()))
        .unwrap_or_else(|_| ("unknown".into(), body));
    Err(AppError::Api {
        status: status.as_u16(),
        reason,
        message,
    })
}

const DONE_HTML: &str = "<!doctype html><meta charset=utf-8><title>Emissa</title>\
<body style=\"font:16px system-ui;background:#0a0a0a;color:#eee;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center\"><h2>Signed in to Emissa</h2><p style=\"opacity:.6\">You can close this tab.</p></div>";

/// A minimal one-shot HTTP server. We only need the request line, so no HTTP crate.
/// We keep looping because browsers may hit `/favicon.ico` first.
async fn accept_code(listener: &TcpListener, expected_state: &str) -> AppResult<String> {
    loop {
        let (mut sock, _) = listener.accept().await?;
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await?;
        let req = String::from_utf8_lossy(&buf[..n]);
        let target = req
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("/");
        let parsed = url::Url::parse(&format!("http://127.0.0.1{target}"))
            .map_err(|e| AppError::msg(e.to_string()))?;
        let q: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        let respond = |status: &str, body: &str| {
            format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
        };

        if let Some(err) = q.get("error") {
            let _ = sock
                .write_all(respond("400 Bad Request", "Sign-in was cancelled. You can close this tab.").as_bytes())
                .await;
            return Err(AppError::msg(format!("Google sign-in failed: {err}")));
        }
        let Some(code) = q.get("code") else {
            let _ = sock.write_all(respond("404 Not Found", "").as_bytes()).await;
            continue;
        };
        // CSRF check: the code must come back to the same auth request we started.
        if q.get("state").map(String::as_str) != Some(expected_state) {
            let _ = sock.write_all(respond("400 Bad Request", "State mismatch.").as_bytes()).await;
            return Err(AppError::msg("OAuth state mismatch, possible CSRF. Sign-in aborted."));
        }
        let _ = sock.write_all(respond("200 OK", DONE_HTML).as_bytes()).await;
        let _ = sock.shutdown().await;
        return Ok(code.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_check() {
        assert!(missing_scopes(&format!("{SCOPE_READONLY} {SCOPE_UPLOAD}")).is_empty());
        assert_eq!(missing_scopes(SCOPE_READONLY), vec!["Manage your YouTube videos"]);
        assert_eq!(missing_scopes(""), vec!["Manage your YouTube videos", "View your YouTube account"]);
        // A broader scope string that merely contains the text must not count.
        assert_eq!(missing_scopes("https://www.googleapis.com/auth/youtube.upload.extra"),
            vec!["Manage your YouTube videos", "View your YouTube account"]);
    }

    #[test]
    fn pkce_shapes() {
        let (v, c) = pkce_pair();
        assert_eq!(v.len(), 64);
        assert_eq!(c.len(), 43); // base64url(no pad) of 32 bytes
        assert_eq!(c, URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes())));
    }
}
