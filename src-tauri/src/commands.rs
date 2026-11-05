use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::auth::Auth;
use crate::config;
use crate::error::{AppError, AppResult};
use crate::queue::Queue;
use crate::queue::job::{self, Job, JobState, Metadata};
use crate::youtube::{channel, thumbnail};

type AuthS<'a> = State<'a, Arc<Auth>>;
type QueueS<'a> = State<'a, Arc<Queue>>;
type HttpS<'a> = State<'a, reqwest::Client>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    has_client: bool,
    signed_in: bool,
}

async fn status(auth: &Auth) -> AppResult<SetupStatus> {
    Ok(SetupStatus {
        has_client: auth.has_client().await,
        signed_in: auth.is_signed_in()?,
    })
}

// ---------- setup / account ----------

#[tauri::command]
pub async fn get_setup_status(auth: AuthS<'_>) -> AppResult<SetupStatus> {
    status(&auth).await
}

#[tauri::command]
pub async fn import_client_json(app: AppHandle, auth: AuthS<'_>, path: String) -> AppResult<SetupStatus> {
    let creds = config::import(&app, Path::new(&path))?;
    auth.set_client(creds).await;
    status(&auth).await
}

#[tauri::command]
pub async fn sign_in(app: AppHandle, auth: AuthS<'_>) -> AppResult<()> {
    auth.sign_in(&app).await
}

#[tauri::command]
pub async fn sign_out(auth: AuthS<'_>, q: QueueS<'_>) -> AppResult<()> {
    // Stop anything in flight first; its requests would fail without a token anyway.
    pause_all_inner(&q)?;
    auth.sign_out().await
}

#[tauri::command]
pub async fn get_channel(auth: AuthS<'_>, http: HttpS<'_>) -> AppResult<channel::Channel> {
    let token = auth.access_token().await?;
    channel::mine(&http, &token).await
}

#[tauri::command]
pub async fn open_in_studio(app: AppHandle, video_id: String) -> AppResult<()> {
    // The ID is interpolated into a URL, so whitelist its charset rather than trusting the caller.
    if video_id.is_empty() || !video_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::msg("Invalid video ID"));
    }
    app.opener()
        .open_url(format!("https://studio.youtube.com/video/{video_id}/edit"), None::<&str>)
        .map_err(|e| AppError::msg(e.to_string()))
}

// ---------- queue ----------

#[tauri::command]
pub fn list_jobs(q: QueueS<'_>) -> Vec<Job> {
    q.snapshot()
}

/// Returns the file names that were rejected (not a video, or already queued).
#[tauri::command]
pub fn add_files(app: AppHandle, q: QueueS<'_>, paths: Vec<String>) -> AppResult<Vec<String>> {
    let scope = app.asset_protocol_scope();
    let mut rejected = vec![];
    let mut fresh = vec![];
    let existing: Vec<String> = q
        .snapshot()
        .into_iter()
        .filter(|j| j.state != JobState::Done)
        .map(|j| j.path)
        .collect();

    for p in paths {
        let path = PathBuf::from(&p);
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(p.clone());
        let Ok(meta) = std::fs::metadata(&path) else {
            rejected.push(name);
            continue;
        };
        if !meta.is_file() || !job::is_video(&path) || existing.contains(&p) {
            rejected.push(name);
            continue;
        }
        // Grant asset:// read access to exactly this file, for the preview player.
        scope.allow_file(&path)?;
        fresh.push(Job::from_path(&path, meta.len()));
    }
    if !fresh.is_empty() {
        q.update(|inner| {
            inner.jobs.extend(fresh);
            Ok(())
        })?;
    }
    Ok(rejected)
}

/// Saved while the user types, so no validation here (drafts may be incomplete);
/// `start` validates. Metadata locks once a session exists, because YouTube got it at session creation.
#[tauri::command]
pub fn update_metadata(q: QueueS<'_>, id: String, metadata: Metadata) -> AppResult<()> {
    q.update_job(&id, |j| {
        if j.session_uri.is_some() || matches!(j.state, JobState::Uploading | JobState::Done) {
            return Err(AppError::msg("Metadata is locked once an upload has started. Cancel it to edit."));
        }
        j.metadata = metadata;
        Ok(())
    })
}

#[tauri::command]
pub fn set_thumbnail(app: AppHandle, q: QueueS<'_>, id: String, path: Option<String>) -> AppResult<()> {
    if let Some(p) = &path {
        thumbnail::validate(Path::new(p))?;
        app.asset_protocol_scope().allow_file(p)?;
    }
    q.update_job(&id, |j| {
        if j.state == JobState::Done {
            return Err(AppError::msg("Already uploaded. Change the thumbnail in YouTube Studio."));
        }
        j.thumbnail = path;
        Ok(())
    })
}

fn enqueue(j: &mut Job) -> AppResult<()> {
    job::validate(&j.metadata).map_err(AppError::Msg)?;
    j.state = JobState::Queued;
    j.error = None;
    Ok(())
}

#[tauri::command]
pub fn start(q: QueueS<'_>, id: String) -> AppResult<()> {
    q.update_job(&id, |j| match j.state {
        JobState::Draft | JobState::Failed | JobState::Paused => enqueue(j),
        _ => Ok(()),
    })?;
    q.wake();
    Ok(())
}

/// Queues every draft with valid metadata. Returns how many were skipped as invalid.
#[tauri::command]
pub fn start_all(q: QueueS<'_>) -> AppResult<usize> {
    let skipped = q.update(|inner| {
        let mut skipped = 0;
        for j in inner.jobs.iter_mut().filter(|j| j.state == JobState::Draft) {
            if enqueue(j).is_err() {
                skipped += 1;
            }
        }
        Ok(skipped)
    })?;
    q.wake();
    Ok(skipped)
}

#[tauri::command]
pub fn pause(q: QueueS<'_>, id: String) -> AppResult<()> {
    q.update(|inner| {
        if let Some(j) = inner.jobs.iter_mut().find(|j| j.id == id) {
            if matches!(j.state, JobState::Uploading | JobState::Queued) {
                // Set the state BEFORE cancelling, so the worker sees its job already
                // moved on and doesn't overwrite it.
                j.state = JobState::Paused;
            }
        }
        if let Some(t) = inner.cancels.get(&id) {
            t.cancel();
        }
        Ok(())
    })
}

fn pause_all_inner(q: &Queue) -> AppResult<()> {
    q.update(|inner| {
        for j in inner.jobs.iter_mut() {
            if matches!(j.state, JobState::Uploading | JobState::Queued) {
                j.state = JobState::Paused;
            }
        }
        for t in inner.cancels.values() {
            t.cancel();
        }
        Ok(())
    })
}

#[tauri::command]
pub fn resume(q: QueueS<'_>, id: String) -> AppResult<()> {
    start(q, id)
}

/// Abandon the upload session and return to an editable draft. An incomplete resumable
/// session never becomes a video on YouTube; it just expires server-side.
#[tauri::command]
pub fn cancel(q: QueueS<'_>, id: String) -> AppResult<()> {
    q.update(|inner| {
        if let Some(j) = inner.jobs.iter_mut().find(|j| j.id == id) {
            if j.state != JobState::Done {
                j.state = JobState::Draft;
                j.session_uri = None;
                j.bytes_sent = 0;
                j.error = None;
            }
        }
        if let Some(t) = inner.cancels.get(&id) {
            t.cancel();
        }
        Ok(())
    })
}

#[tauri::command]
pub fn retry(q: QueueS<'_>, id: String) -> AppResult<()> {
    start(q, id)
}

#[tauri::command]
pub fn remove(q: QueueS<'_>, id: String) -> AppResult<()> {
    q.update(|inner| {
        if let Some(t) = inner.cancels.get(&id) {
            t.cancel();
        }
        inner.jobs.retain(|j| j.id != id);
        Ok(())
    })
}

#[tauri::command]
pub fn clear_done(q: QueueS<'_>) -> AppResult<()> {
    q.update(|inner| {
        inner.jobs.retain(|j| j.state != JobState::Done);
        Ok(())
    })
}

/// Reorders by the given ID list. Unknown IDs are ignored, and missing jobs keep their
/// relative order at the end, so a stale frontend list can't drop jobs.
#[tauri::command]
pub fn reorder(q: QueueS<'_>, ids: Vec<String>) -> AppResult<()> {
    q.update(|inner| {
        inner
            .jobs
            .sort_by_key(|j| ids.iter().position(|i| *i == j.id).unwrap_or(usize::MAX));
        Ok(())
    })
}
