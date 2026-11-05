use std::path::Path;

use super::job::Job;
use crate::error::AppResult;

pub fn load(file: &Path) -> AppResult<Vec<Job>> {
    if !file.exists() {
        return Ok(vec![]);
    }
    Ok(serde_json::from_slice(&std::fs::read(file)?)?)
}

/// Write to a temp file, then rename over the old one. A crash mid-write leaves the old
/// queue intact instead of a half-written JSON that would wipe every pending upload.
/// (On Windows, std's rename uses MoveFileExW with REPLACE_EXISTING.)
pub fn save(file: &Path, jobs: &[Job]) -> AppResult<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(jobs)?)?;
    std::fs::rename(tmp, file)?;
    Ok(())
}
