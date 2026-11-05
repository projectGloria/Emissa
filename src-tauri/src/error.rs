use serde::{Serialize, Serializer};

/// One error type for every command. Tauri needs command errors to be `Serialize`;
/// we flatten to `{ kind, message }` so the frontend can branch on `kind`
/// (e.g. bounce to the sign-in screen on `notSignedIn`) without string-matching.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Msg(String),
    #[error("Not signed in")]
    NotSignedIn,
    #[error("Google OAuth client is not configured")]
    NoClient,
    #[error("Network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Credential store error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("YouTube API {status} ({reason}): {message}")]
    Api {
        status: u16,
        reason: String,
        message: String,
    },
}

impl AppError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Msg(s.into())
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Msg(_) => "message",
            Self::NotSignedIn => "notSignedIn",
            Self::NoClient => "noClient",
            Self::Http(_) => "network",
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::Keyring(_) => "keyring",
            Self::Tauri(_) => "tauri",
            Self::Api { .. } => "api",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
