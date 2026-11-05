//! A single serial upload worker. One at a time is deliberate: parallel uploads
//! split the same uplink (no faster overall) and burn quota in bursts.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

use super::job::{Job, JobState};
use super::{EVT_FINISHED, EVT_PROGRESS, Queue};
use crate::auth::Auth;
use crate::error::{AppError, AppResult};
use crate::youtube::upload::{self, ChunkResult, Recovery};
use crate::youtube::{UPLOAD_BASE, thumbnail};

const MAX_RETRIES: u32 = 10;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    id: &'a str,
    bytes_sent: u64,
    total: u64,
    bps: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Finished<'a> {
    id: &'a str,
    title: &'a str,
    ok: bool,
    message: Option<String>,
}

pub async fn run(app: AppHandle, q: Arc<Queue>, auth: Arc<Auth>, http: reqwest::Client) {
    loop {
        match q.claim_next() {
            Some((job, cancel)) => run_job(&app, &q, &auth, &http, job, cancel).await,
            None => q.notify.notified().await,
        }
    }
}

/// Exponential backoff with jitter: 1, 2, 4 … 60 s. The jitter keeps retries from
/// landing in lockstep with Google's own load spikes.
fn backoff(attempt: u32) -> Duration {
    let base = 1u64 << attempt.min(6);
    let jitter_ms = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_millis())
        .unwrap_or(0)) as u64;
    Duration::from_secs(base.min(60)) + Duration::from_millis(jitter_ms)
}

async fn run_job(app: &AppHandle, q: &Queue, auth: &Auth, http: &reqwest::Client, job: Job, cancel: CancellationToken) {
    let id = job.id.clone();
    let total = job.size;
    let sent = Arc::new(AtomicU64::new(job.bytes_sent));
    let ticker = spawn_ticker(app.clone(), id.clone(), total, sent.clone());

    // select! on the token: cancelling drops the whole `drive` future, including any
    // in-flight PUT or backoff sleep, at its next await point. That's pause and cancel for free.
    let result = tokio::select! {
        _ = cancel.cancelled() => None,
        r = drive(q, auth, http, &job, &sent) => Some(r),
    };
    ticker.abort();
    q.update(|inner| {
        inner.cancels.remove(&id);
        Ok(())
    })
    .ok();

    let Some(result) = result else {
        // Cancelled. The command that cancelled us already set the new state
        // (Paused/Draft/removed), so there's nothing to overwrite.
        return;
    };

    match result {
        Ok(video_id) => {
            // Re-read: the user may have picked a thumbnail while the video was uploading.
            let thumb = q.get(&id).and_then(|j| j.thumbnail);
            let warning = match thumb {
                Some(t) => match set_thumb(auth, http, &video_id, Path::new(&t)).await {
                    Ok(()) => None,
                    Err(e) => Some(format!("Video uploaded, but the thumbnail failed: {e}")),
                },
                None => None,
            };
            let _ = q.update_job(&id, |j| {
                j.state = JobState::Done;
                j.video_id = Some(video_id.clone());
                j.bytes_sent = total;
                j.session_uri = None;
                j.warning = warning.clone();
                Ok(())
            });
            let _ = app.emit(
                EVT_FINISHED,
                Finished { id: &id, title: &job.metadata.title, ok: true, message: warning },
            );
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = q.update_job(&id, |j| {
                j.state = JobState::Failed;
                j.error = Some(msg.clone());
                Ok(())
            });
            let _ = app.emit(
                EVT_FINISHED,
                Finished { id: &id, title: &job.metadata.title, ok: false, message: Some(msg) },
            );
        }
    }
}

async fn set_thumb(auth: &Auth, http: &reqwest::Client, video_id: &str, path: &Path) -> AppResult<()> {
    let token = auth.access_token().await?;
    thumbnail::set(http, &token, video_id, path).await
}

/// The upload state machine. Returns the YouTube video ID.
async fn drive(q: &Queue, auth: &Auth, http: &reqwest::Client, job: &Job, sent: &Arc<AtomicU64>) -> AppResult<String> {
    let path = Path::new(&job.path);
    let total = job.size;
    // If the file changed since it was queued, resuming would splice two different files together.
    if tokio::fs::metadata(path).await?.len() != total {
        return Err(AppError::msg("The file changed size since it was added. Remove it and add it again."));
    }

    let mut session = job.session_uri.clone();
    let mut offset = 0u64;
    // An existing session means we're resuming, so the server knows the true offset, not us.
    let mut need_query = session.is_some();
    let (mut attempt, mut refreshed, mut restarted) = (0u32, false, false);

    loop {
        let step: AppResult<Option<String>> = async {
            let token = auth.access_token().await?;
            let s = match &session {
                Some(s) => s.clone(),
                None => {
                    let s = upload::start_session(http, UPLOAD_BASE, &token, &job.metadata, total, &job.mime).await?;
                    // Persist right away: if we crash in the next second, the session is still recoverable.
                    q.update_job(&job.id, |j| {
                        j.session_uri = Some(s.clone());
                        Ok(())
                    })?;
                    session = Some(s.clone());
                    s
                }
            };
            if need_query {
                match upload::query_offset(http, &s, &token, total).await? {
                    ChunkResult::Complete(id) => return Ok(Some(id)),
                    ChunkResult::Incomplete(n) => {
                        offset = n;
                        need_query = false;
                    }
                }
            }
            // The body stream bumps `sent` as hyper pulls slices; the ticker reads it.
            match upload::put_chunk(http, &s, &token, path, offset, total, sent.clone()).await? {
                ChunkResult::Complete(id) => Ok(Some(id)),
                ChunkResult::Incomplete(n) => {
                    offset = n;
                    Ok(None)
                }
            }
        }
        .await;

        match step {
            Ok(Some(video_id)) => return Ok(video_id),
            Ok(None) => {
                attempt = 0;
                refreshed = false;
                sent.store(offset, Ordering::Relaxed);
                q.update_job(&job.id, |j| {
                    j.bytes_sent = offset;
                    Ok(())
                })?;
            }
            Err(e) => {
                // Bytes that were in flight are not committed; roll the bar back to the truth.
                sent.store(offset, Ordering::Relaxed);
                need_query = session.is_some();
                match upload::recovery_for(&e) {
                    Recovery::Retry if attempt < MAX_RETRIES => {
                        tokio::time::sleep(backoff(attempt)).await;
                        attempt += 1;
                    }
                    Recovery::RefreshToken if !refreshed => {
                        auth.invalidate().await;
                        refreshed = true;
                    }
                    Recovery::RestartSession if !restarted => {
                        restarted = true;
                        session = None;
                        offset = 0;
                        need_query = false;
                        sent.store(0, Ordering::Relaxed);
                        q.update_job(&job.id, |j| {
                            j.session_uri = None;
                            j.bytes_sent = 0;
                            Ok(())
                        })?;
                    }
                    _ => return Err(e),
                }
            }
        }
    }
}

/// Emits progress at most 10×/s from an atomic counter. The uploader never awaits
/// the IPC bridge, and the webview isn't flooded with one event per 256 KiB slice.
fn spawn_ticker(app: AppHandle, id: String, total: u64, sent: Arc<AtomicU64>) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut iv = tokio::time::interval(Duration::from_millis(100));
        let mut last_bytes = sent.load(Ordering::Relaxed);
        let mut last_t = Instant::now();
        let mut bps = 0f64;
        let mut quiet = 0u32;
        loop {
            iv.tick().await;
            let now = sent.load(Ordering::Relaxed);
            let dt = last_t.elapsed().as_secs_f64().max(1e-3);
            let inst = now.saturating_sub(last_bytes) as f64 / dt;
            // EMA smooths out the sawtooth that comes from hyper buffering slices.
            bps = if bps == 0.0 { inst } else { bps * 0.85 + inst * 0.15 };
            last_t = Instant::now();
            let changed = now != last_bytes;
            last_bytes = now;
            quiet = if changed { 0 } else { quiet + 1 };
            // While stalled, still send a heartbeat every second so the speed readout decays.
            if changed || quiet % 10 == 0 {
                let _ = app.emit(EVT_PROGRESS, Progress { id: &id, bytes_sent: now, total, bps });
            }
        }
    })
}
