# Yutub

Lightweight desktop app for uploading videos to your own YouTube channel.
Tauri 2 · Rust · React 19 · TypeScript · Tailwind v4 · shadcn/ui.

- Queue many videos, with per-video title, description, tags, visibility, category, audience, and thumbnail.
- Resumable uploads streamed from disk in 8 MiB chunks. Pause and resume work, and uploads survive an app restart.
- Automatic retry with backoff on network errors, and token refresh on a 401.
- OAuth via the system browser (loopback + PKCE). The refresh token is stored in Windows Credential Manager.

## Setup

1. In Google Cloud Console, enable **YouTube Data API v3**.
2. Google Auth Platform → **Get started**, with audience **External**. Don't use "YouTube"-like app names; Google rejects them.
3. **Audience** → add your Google account as a test user.
4. **Clients** → Create client → **Desktop app** → download the JSON.
5. Run the app, pick that JSON, and sign in.

The client JSON is copied to `%APPDATA%\com.yutub.app\` and is never committed.

## Develop

```bash
npm install
npm run tauri dev
```

```bash
cd src-tauri && cargo test
```

## Known limits

- Default quota is 10k units/day, and one upload costs about 1600, so roughly 6 uploads/day.
- Unverified API projects: uploads are forced to private until Google's compliance audit.

See [todo.md](todo.md) for open issues.
