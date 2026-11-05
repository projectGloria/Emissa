import { invoke } from "@tauri-apps/api/core";

// Hand-mirrored from the Rust structs (serde camelCase). Keep in sync with
// src-tauri/src/queue/job.rs and src-tauri/src/commands.rs.

export type Privacy = "private" | "unlisted" | "public";
export type JobState = "draft" | "queued" | "uploading" | "paused" | "done" | "failed";

export interface Metadata {
  title: string;
  description: string;
  tags: string[];
  categoryId: string;
  privacy: Privacy;
  madeForKids: boolean;
  containsSyntheticMedia: boolean;
  embeddable: boolean;
  notifySubscribers: boolean;
}

/** Everything a new video starts with except its title (which comes from the file name). */
export type VideoDefaults = Omit<Metadata, "title">;

export interface Settings {
  defaults: VideoDefaults;
}

export interface Caption {
  path: string;
  language: string;
  name: string;
}

export interface Job {
  id: string;
  path: string;
  fileName: string;
  size: number;
  mime: string;
  metadata: Metadata;
  thumbnail: string | null;
  captions: Caption[];
  state: JobState;
  sessionUri: string | null;
  bytesSent: number;
  videoId: string | null;
  error: string | null;
  warning: string | null;
}

export interface Channel {
  id: string;
  title: string;
  avatarUrl: string | null;
}

export interface SetupStatus {
  hasClient: boolean;
  signedIn: boolean;
}

export interface Progress {
  id: string;
  bytesSent: number;
  total: number;
  bps: number;
}

export interface Finished {
  id: string;
  title: string;
  ok: boolean;
  message: string | null;
}

export type AppErrorKind =
  | "message" | "notSignedIn" | "noClient" | "network" | "io" | "json" | "keyring" | "tauri" | "api";

export interface AppError {
  kind: AppErrorKind;
  message: string;
}

export const EVT = {
  changed: "queue://changed",
  progress: "queue://progress",
  finished: "queue://finished",
} as const;

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "kind" in e && "message" in e;
}

export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}

// Tauri converts Rust snake_case params to camelCase on the JS side
// (`video_id` → `videoId`), so the arg keys below are camelCase.
export const api = {
  getSetupStatus: () => invoke<SetupStatus>("get_setup_status"),
  importClientJson: (path: string) => invoke<SetupStatus>("import_client_json", { path }),
  signIn: () => invoke<void>("sign_in"),
  signOut: () => invoke<void>("sign_out"),
  getChannel: () => invoke<Channel>("get_channel"),
  openInStudio: (videoId: string) => invoke<void>("open_in_studio", { videoId }),

  listJobs: () => invoke<Job[]>("list_jobs"),
  addFiles: (paths: string[]) => invoke<string[]>("add_files", { paths }),
  updateMetadata: (id: string, metadata: Metadata) => invoke<void>("update_metadata", { id, metadata }),
  setThumbnail: (id: string, path: string | null) => invoke<void>("set_thumbnail", { id, path }),
  setCaptions: (id: string, captions: Caption[]) => invoke<void>("set_captions", { id, captions }),
  captionAccess: () => invoke<boolean>("caption_access"),
  start: (id: string) => invoke<void>("start", { id }),
  startAll: () => invoke<number>("start_all"),
  pause: (id: string) => invoke<void>("pause", { id }),
  resume: (id: string) => invoke<void>("resume", { id }),
  cancel: (id: string) => invoke<void>("cancel", { id }),
  retry: (id: string) => invoke<void>("retry", { id }),
  remove: (id: string) => invoke<void>("remove", { id }),
  clearDone: () => invoke<void>("clear_done"),
  reorder: (ids: string[]) => invoke<void>("reorder", { ids }),

  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (value: Settings) => invoke<void>("set_settings", { value }),
  openDataDir: () => invoke<void>("open_data_dir"),
};
