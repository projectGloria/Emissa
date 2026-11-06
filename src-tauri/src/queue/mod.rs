pub mod job;
mod persist;
pub mod worker;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, AppResult};
use job::{Job, JobState};

pub const EVT_CHANGED: &str = "queue://changed";
pub const EVT_PROGRESS: &str = "queue://progress";
pub const EVT_FINISHED: &str = "queue://finished";

pub struct Inner {
    pub jobs: Vec<Job>,
    /// One token per in-flight job. Cancelling it drops the worker's upload future at its
    /// next `.await`, which tears down the HTTP request. No polling flags needed.
    pub cancels: HashMap<String, CancellationToken>,
}

/// The single source of truth for the upload queue. The frontend never holds its own
/// copy: it renders the snapshots we broadcast, so the two can't drift apart.
pub struct Queue {
    app: AppHandle,
    // std (not tokio) Mutex: it's never held across an `.await`, and a sync lock is
    // cheaper and can't be accidentally kept across a suspension point.
    inner: Mutex<Inner>,
    pub notify: Notify,
    file: PathBuf,
}

impl Queue {
    pub fn load(app: AppHandle) -> AppResult<Self> {
        let file = app.path().app_data_dir()?.join("queue.json");
        let mut jobs = persist::load(&file).unwrap_or_else(|e| {
            eprintln!("queue.json unreadable, starting empty: {e}");
            vec![]
        });
        let scope = app.asset_protocol_scope();
        for j in &mut jobs {
            // Quit or crashed mid-upload: put it back at the head of the line. The worker sees
            // the saved session URI, asks the server for the committed offset, and carries on.
            // Queued jobs stay queued, so the whole queue continues on launch, in order.
            if j.state == JobState::Uploading {
                j.state = if j.video_id.is_some() { JobState::Done } else { JobState::Queued };
            }
            // Asset scope is runtime-only; re-grant preview access for persisted files.
            let _ = scope.allow_file(&j.path);
            if let Some(t) = &j.thumbnail {
                let _ = scope.allow_file(t);
            }
        }
        Ok(Self {
            app,
            inner: Mutex::new(Inner {
                jobs,
                cancels: HashMap::new(),
            }),
            notify: Notify::new(),
            file,
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // A poisoned lock means a panic mid-mutation. With panic=abort in release that
        // can't happen; in dev, keep going with the data rather than cascading panics.
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn snapshot(&self) -> Vec<Job> {
        self.lock().jobs.clone()
    }

    pub fn get(&self, id: &str) -> Option<Job> {
        self.lock().jobs.iter().find(|j| j.id == id).cloned()
    }

    /// Mutate under the lock, then persist and broadcast outside it.
    /// Every state change goes through here, so disk, memory and UI stay in sync.
    pub fn update<R>(&self, f: impl FnOnce(&mut Inner) -> AppResult<R>) -> AppResult<R> {
        let (out, snapshot) = {
            let mut g = self.lock();
            let out = f(&mut g)?;
            (out, g.jobs.clone())
        };
        if let Err(e) = persist::save(&self.file, &snapshot) {
            eprintln!("failed to persist queue: {e}");
        }
        let _ = self.app.emit(EVT_CHANGED, &snapshot);
        Ok(out)
    }

    pub fn update_job(&self, id: &str, f: impl FnOnce(&mut Job) -> AppResult<()>) -> AppResult<()> {
        self.update(|inner| {
            let job = inner
                .jobs
                .iter_mut()
                .find(|j| j.id == id)
                .ok_or_else(|| AppError::msg("Job not found"))?;
            f(job)
        })
    }

    /// Atomically pick the next queued job and mark it Uploading, so two
    /// workers (or a racing command) can never grab the same job.
    pub fn claim_next(&self) -> Option<(Job, CancellationToken)> {
        self.update(|inner| {
            let Some(job) = inner.jobs.iter_mut().find(|j| j.state == JobState::Queued) else {
                return Ok(None);
            };
            job.state = JobState::Uploading;
            job.error = None;
            let token = CancellationToken::new();
            inner.cancels.insert(job.id.clone(), token.clone());
            Ok(Some((job.clone(), token)))
        })
        .ok()
        .flatten()
    }

    pub fn wake(&self) {
        // notify_one stores a permit when the worker isn't waiting yet, so no wakeup is lost.
        self.notify.notify_one();
    }
}
