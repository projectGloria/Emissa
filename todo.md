# Yutub — TODO

## Bugs

- [ ] **8.3 short paths stored verbatim.** When a file's full path exceeds MAX_PATH (260 chars), Windows hands
  the app its 8.3 alias (e.g. `INTERV~4.MKV`). Effects: wrong file name in the queue, default title becomes
  `INTERV~4`, and duplicate detection misses the same file added via its long path, which risks a double upload and
  wasted quota.
  **Fix:** in `add_files` (`src-tauri/src/commands.rs`), resolve each path with `std::fs::canonicalize` (expands
  short names) and strip the `\\?\` prefix for display with the `dunce` crate. Dedupe on the canonical path.
  Changes Rust code, so it restarts `tauri dev` (in-flight uploads resume afterwards).

## Features

- [ ] Drag-to-reorder in the queue sidebar. The backend `reorder(ids)` command already exists, but the UI has no drag handles.
- [ ] Scheduled publishing (`status.publishAt`) and playlist assignment.
- [ ] Metadata templates (reusable description/tags blocks).
- [ ] Show the remaining daily quota estimate (about 1600 units per upload, 10k per day by default).

## Google project / ops

- [ ] Optional: publish the OAuth app ("In production") so refresh tokens stop expiring every 7 days.
  Requires a homepage URL and privacy policy URL on the Branding page.
- [ ] Optional: apply for the YouTube API compliance audit. Until then, API uploads are forced to private.
- [ ] Phone-verify the channel (youtube.com/verify) so custom thumbnails are accepted.

## Release

- [ ] App icon (`npx tauri icon <source.png>`); currently the default Tauri icon.
- [ ] `npm run tauri build` and check the installer size and idle RAM.
- [ ] MSI requires a numeric-only pre-release version (e.g. `1.0.0-1`, not `1.0.0-beta.1`). Pick a versioning scheme before the first installer.
