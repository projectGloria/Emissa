import type { Metadata } from "./api";

// Mirrors src-tauri/src/queue/job.rs::validate. Rust is the authority (it re-checks on start);
// this copy exists only for instant inline feedback while typing.

export const LIMITS = { title: 100, descriptionBytes: 5000, tags: 500 } as const;

const encoder = new TextEncoder();
export const byteLen = (s: string) => encoder.encode(s).length;
// Spread iterates code points, matching Rust's `chars().count()`. `.length` counts UTF-16 units.
export const charLen = (s: string) => [...s].length;

export function tagsLen(tags: string[]): number {
  const body = tags.reduce((n, t) => n + charLen(t) + (t.includes(" ") ? 2 : 0), 0);
  return body + Math.max(tags.length - 1, 0);
}

const ANGLE = /[<>]/;

export function validate(m: Metadata): string | null {
  const title = m.title.trim();
  if (!title) return "Title is required.";
  if (charLen(title) > LIMITS.title) return "Title must be 100 characters or fewer.";
  if (byteLen(m.description) > LIMITS.descriptionBytes) return "Description must be 5000 bytes or fewer.";
  if (ANGLE.test(title) || ANGLE.test(m.description) || m.tags.some((t) => ANGLE.test(t)))
    return "Title, description and tags can't contain < or >.";
  if (tagsLen(m.tags) > LIMITS.tags) return "Tags must total 500 characters or fewer.";
  return null;
}

// Assignable categories (US region). Static, because videoCategories.list would cost
// a quota call for data that basically never changes.
export const CATEGORIES: { id: string; label: string }[] = [
  { id: "1", label: "Film & Animation" },
  { id: "2", label: "Autos & Vehicles" },
  { id: "10", label: "Music" },
  { id: "15", label: "Pets & Animals" },
  { id: "17", label: "Sports" },
  { id: "19", label: "Travel & Events" },
  { id: "20", label: "Gaming" },
  { id: "22", label: "People & Blogs" },
  { id: "23", label: "Comedy" },
  { id: "24", label: "Entertainment" },
  { id: "25", label: "News & Politics" },
  { id: "26", label: "Howto & Style" },
  { id: "27", label: "Education" },
  { id: "28", label: "Science & Technology" },
  { id: "29", label: "Nonprofits & Activism" },
];

export const VIDEO_EXTENSIONS = [
  "mp4", "mov", "mkv", "webm", "avi", "wmv", "flv", "m4v", "mpg", "mpeg", "3gp", "mts", "m2ts",
];
