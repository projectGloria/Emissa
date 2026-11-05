use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::AppResult;
use crate::queue::job::{Metadata, Privacy};

/// Metadata pre-filled on every newly added video (everything except the title,
/// which comes from the file name).
#[derive(Debug, Clone, Serialize, Deserialize)]
// Container-level default: any field missing from an older settings.json is filled
// from `Default`, so adding a field later never invalidates the file.
#[serde(rename_all = "camelCase", default)]
pub struct VideoDefaults {
    pub description: String,
    pub tags: Vec<String>,
    pub category_id: String,
    pub privacy: Privacy,
    pub made_for_kids: bool,
    pub contains_synthetic_media: bool,
    pub embeddable: bool,
    pub notify_subscribers: bool,
}

impl Default for VideoDefaults {
    fn default() -> Self {
        Self {
            description: String::new(),
            tags: vec![],
            category_id: "22".into(), // "People & Blogs", YouTube's own default
            privacy: Privacy::Private,
            made_for_kids: false,
            contains_synthetic_media: false,
            embeddable: true,
            notify_subscribers: true,
        }
    }
}

impl VideoDefaults {
    pub fn to_metadata(&self, title: String) -> Metadata {
        Metadata {
            title,
            description: self.description.clone(),
            tags: self.tags.clone(),
            category_id: self.category_id.clone(),
            privacy: self.privacy,
            made_for_kids: self.made_for_kids,
            contains_synthetic_media: self.contains_synthetic_media,
            embeddable: self.embeddable,
            notify_subscribers: self.notify_subscribers,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub defaults: VideoDefaults,
}

pub struct SettingsStore {
    file: PathBuf,
    inner: Mutex<Settings>,
}

impl SettingsStore {
    pub fn load(app: &AppHandle) -> AppResult<Self> {
        let file = app.path().app_config_dir()?.join("settings.json");
        // A corrupt file shouldn't brick startup: fall back to defaults and overwrite on the next save.
        let settings = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Ok(Self { file, inner: Mutex::new(settings) })
    }

    pub fn get(&self) -> Settings {
        self.inner.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn set(&self, s: Settings) -> AppResult<()> {
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // Same temp-file-then-rename pattern as queue.json: never leave a half-written file.
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&s)?)?;
        std::fs::rename(tmp, &self.file)?;
        *self.inner.lock().unwrap_or_else(|p| p.into_inner()) = s;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_file_fills_defaults() {
        let s: Settings = serde_json::from_str(r#"{"defaults":{"privacy":"unlisted"}}"#).unwrap();
        assert_eq!(s.defaults.privacy, Privacy::Unlisted);
        assert!(s.defaults.embeddable);
        assert_eq!(s.defaults.category_id, "22");
        let empty: Settings = serde_json::from_str("{}").unwrap();
        assert!(empty.defaults.notify_subscribers);
    }
}
