use std::path::Path;

use serde::{Deserialize, Serialize};

pub const VIDEO_EXTS: &[&str] = &[
    "mp4", "mov", "mkv", "webm", "avi", "wmv", "flv", "m4v", "mpg", "mpeg", "3gp", "mts", "m2ts",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Privacy {
    Private,
    Unlisted,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub category_id: String,
    pub privacy: Privacy,
    pub made_for_kids: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Draft,
    Queued,
    Uploading,
    Paused,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub path: String,
    pub file_name: String,
    pub size: u64,
    pub mime: String,
    pub metadata: Metadata,
    pub thumbnail: Option<String>,
    pub state: JobState,
    /// Resumable session URI. Persisted so an upload survives an app restart (valid ~1 week).
    pub session_uri: Option<String>,
    /// Bytes the *server* has confirmed. Live in-flight progress goes over the progress event instead.
    pub bytes_sent: u64,
    pub video_id: Option<String>,
    pub error: Option<String>,
    /// Non-fatal problems, e.g. the video uploaded but the custom thumbnail was rejected.
    pub warning: Option<String>,
}

impl Job {
    pub fn from_path(path: &Path, size: u64) -> Self {
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            path: path.to_string_lossy().into_owned(),
            file_name,
            size,
            mime: mime_guess::from_path(path)
                .first()
                .map(|m| m.essence_str().to_owned())
                .unwrap_or_else(|| "video/*".into()),
            metadata: Metadata {
                // Take characters, not bytes, so a multi-byte UTF-8 char is never split.
                title: sanitize(&stem).chars().take(100).collect(),
                description: String::new(),
                tags: vec![],
                category_id: "22".into(), // "People & Blogs", YouTube's own default
                privacy: Privacy::Private,
                made_for_kids: false,
            },
            thumbnail: None,
            state: JobState::Draft,
            session_uri: None,
            bytes_sent: 0,
            video_id: None,
            error: None,
            warning: None,
        }
    }
}

pub fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| VIDEO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
}

fn sanitize(s: &str) -> String {
    s.replace(['<', '>'], "")
}

/// YouTube's own counting rule for tags: commas between tags count, and a tag
/// containing a space is quoted, which adds 2 characters.
pub fn tags_len(tags: &[String]) -> usize {
    let body: usize = tags
        .iter()
        .map(|t| t.chars().count() + if t.contains(' ') { 2 } else { 0 })
        .sum();
    body + tags.len().saturating_sub(1)
}

/// Mirrors YouTube's server-side limits, so the user sees a precise message
/// instead of a generic 400 after the upload session has already burned quota.
pub fn validate(m: &Metadata) -> Result<(), String> {
    let title = m.title.trim();
    if title.is_empty() {
        return Err("Title is required.".into());
    }
    if title.chars().count() > 100 {
        return Err("Title must be 100 characters or fewer.".into());
    }
    if m.description.len() > 5000 {
        return Err("Description must be 5000 bytes or fewer.".into());
    }
    let angle = |s: &str| s.contains(['<', '>']);
    if angle(title) || angle(&m.description) || m.tags.iter().any(|t| angle(t)) {
        return Err("Title, description and tags can't contain < or >.".into());
    }
    if tags_len(&m.tags) > 500 {
        return Err("Tags must total 500 characters or fewer.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> Metadata {
        Metadata {
            title: "ok".into(),
            description: String::new(),
            tags: vec![],
            category_id: "22".into(),
            privacy: Privacy::Private,
            made_for_kids: false,
        }
    }

    #[test]
    fn validation() {
        assert!(validate(&m()).is_ok());
        assert!(validate(&Metadata { title: "  ".into(), ..m() }).is_err());
        assert!(validate(&Metadata { title: "é".repeat(100), ..m() }).is_ok()); // chars, not bytes
        assert!(validate(&Metadata { title: "a".repeat(101), ..m() }).is_err());
        assert!(validate(&Metadata { description: "é".repeat(2501), ..m() }).is_err()); // bytes
        assert!(validate(&Metadata { title: "a<b".into(), ..m() }).is_err());
        assert!(validate(&Metadata { tags: vec!["x>".into()], ..m() }).is_err());
    }

    #[test]
    fn tag_counting() {
        assert_eq!(tags_len(&[]), 0);
        assert_eq!(tags_len(&["ab".into(), "cd".into()]), 5);
        assert_eq!(tags_len(&["a b".into()]), 5);
    }

    #[test]
    fn job_defaults() {
        let j = Job::from_path(Path::new(r"C:\v\My <Clip>.MP4"), 10);
        assert_eq!(j.metadata.title, "My Clip");
        assert_eq!(j.mime, "video/mp4");
        assert!(is_video(Path::new("a.MKV")));
        assert!(!is_video(Path::new("a.txt")));
    }
}
