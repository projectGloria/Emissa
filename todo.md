# Emissa — TODO

## Bugs

- [x] ~~8.3 short paths stored verbatim (`INTERV~4.MKV`).~~ Fixed: `add_files` canonicalizes + `dunce`.
  Jobs added before the fix keep their short path (harmless for upload; just the display name).

## Features

- [ ] Drag-to-reorder in the queue sidebar. The backend `reorder(ids)` command already exists, but the UI has no drag handles.
- [ ] Scheduled publishing (`status.publishAt`) and playlist assignment.
- [ ] Metadata templates (reusable description/tags blocks).
- [ ] Show quota usage: uploads use the separate "Video Uploads" bucket (1 unit each, 100/day cap);
  captions (400 each), thumbnails (50) and other calls come from the regular 10k/day pool.
- [ ] "Resume all" button for paused jobs.
- [ ] Studio-only settings (no API write access, checked Nov 2026): paid promotion, automatic chapters,
  featured places, automatic concepts, Shorts remixing. Re-check the API changelog periodically.

## Google project / ops

- [ ] Optional: publish the OAuth app ("In production") so refresh tokens stop expiring every 7 days.
  Requires a homepage URL and privacy policy URL on the Branding page.
- [ ] Optional: apply for the YouTube API compliance audit. Until then, API uploads are forced to private.
- [ ] Phone-verify the channel (youtube.com/verify) so custom thumbnails are accepted.

## Release

- [x] ~~App icon~~: `app-icon.svg` → `npx tauri icon app-icon.svg` (delete the generated android/ios folders).
- [ ] `npm run tauri build` and check the installer size and idle RAM.
- [ ] MSI requires a numeric-only pre-release version (e.g. `1.0.0-1`, not `1.0.0-beta.1`). Pick a versioning scheme before the first installer.
