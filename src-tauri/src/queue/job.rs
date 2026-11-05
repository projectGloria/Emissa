use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::settings::VideoDefaults;

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
    // Every field added after beta v1 gets a serde default, so a queue.json written by an
    // older build still deserializes. Without it, one missing field makes the whole file
    // unreadable, and load() would start empty, dropping in-flight upload sessions.
    /// "Altered or synthetic content" disclosure (YouTube's AI-use question).
    #[serde(default)]
    pub contains_synthetic_media: bool,
    #[serde(default = "yes")]
    pub embeddable: bool,
    /// Not part of the video resource; sent as the `notifySubscribers` query param on insert.
    #[serde(default = "yes")]
    pub notify_subscribers: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Caption {
    pub path: String,
    /// BCP-47 code, e.g. "en", "tr", "zh-Hans".
    pub language: String,
    /// Track name shown in the player's CC menu; may be empty.
    pub name: String,
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
    /// Uploaded after the video finishes, like the thumbnail.
    #[serde(default)]
    pub captions: Vec<Caption>,
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
    pub fn from_path(path: &Path, size: u64, defaults: &VideoDefaults) -> Self {
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
            // Take characters, not bytes, so a multi-byte UTF-8 char is never split.
            metadata: defaults.to_metadata(sanitize(&stem).chars().take(100).collect()),
            thumbnail: None,
            captions: vec![],
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
            contains_synthetic_media: false,
            embeddable: true,
            notify_subscribers: true,
        }
    }

    /// A queue.json written by beta v1 (none of the newer fields) must still load.
    #[test]
    fn loads_beta_v1_queue() {
        let v1 = r#"[{"id":"a","path":"C:/v.mp4","fileName":"v.mp4","size":1,"mime":"video/mp4",
            "metadata":{"title":"t","description":"","tags":[],"categoryId":"22","privacy":"private","madeForKids":false},
            "thumbnail":null,"state":"uploading","sessionUri":"https://x/s","bytesSent":8388608,
            "videoId":null,"error":null,"warning":null}]"#;
        let jobs: Vec<Job> = serde_json::from_str(v1).unwrap();
        assert_eq!(jobs[0].session_uri.as_deref(), Some("https://x/s"));
        assert!(jobs[0].metadata.embeddable && jobs[0].metadata.notify_subscribers);
        assert!(!jobs[0].metadata.contains_synthetic_media);
        assert!(jobs[0].captions.is_empty());
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
        let d = VideoDefaults { privacy: Privacy::Unlisted, ..VideoDefaults::default() };
        let j = Job::from_path(Path::new(r"C:\v\My <Clip>.MP4"), 10, &d);
        assert_eq!(j.metadata.title, "My Clip");
        assert_eq!(j.metadata.privacy, Privacy::Unlisted); // settings defaults are applied
        assert_eq!(j.mime, "video/mp4");
        assert!(is_video(Path::new("a.MKV")));
        assert!(!is_video(Path::new("a.txt")));
    }
}

/// Opt-in check against the real on-disk queue: `YUTUB_QUEUE=path cargo test real_queue -- --ignored`
#[cfg(test)]
#[test]
#[ignore]
fn real_queue_loads() {
    let p = std::env::var("YUTUB_QUEUE").expect("set YUTUB_QUEUE");
    let jobs: Vec<Job> = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    for j in &jobs {
        println!("{:?} {} sent={} session={}", j.state, j.file_name, j.bytes_sent, j.session_uri.is_some());
    }
}
