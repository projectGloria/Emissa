import { useCallback, useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, EVT, type Job, type Progress } from "./api";

// External store, not React state. Rust owns the queue, and this module just mirrors
// the latest snapshot. useSyncExternalStore makes the read tear-free under concurrent
// rendering: every component in one render sees the same `jobs` reference.

type Listener = () => void;

let jobs: Job[] = [];
const jobListeners = new Set<Listener>();

// Progress is split out and keyed per job. A 10 Hz tick for job A only notifies
// subscribers of A: one tiny progress bar re-renders, not the whole sidebar and editor.
const progress = new Map<string, Progress>();
const progressListeners = new Map<string, Set<Listener>>();

function setJobs(next: Job[]) {
  jobs = next; // new array reference = the signal React compares with Object.is
  // Drop progress for jobs that no longer exist, so the map doesn't grow forever.
  const live = new Set(next.map((j) => j.id));
  for (const id of progress.keys()) if (!live.has(id)) progress.delete(id);
  jobListeners.forEach((l) => l());
}

function setProgress(p: Progress) {
  progress.set(p.id, p);
  progressListeners.get(p.id)?.forEach((l) => l());
}

/** Start mirroring the Rust queue. Returns a cleanup that is safe under StrictMode's double-invoke. */
export function startQueueSync(): () => void {
  let disposed = false;
  let gotEvent = false;

  // Subscribe BEFORE fetching. If a `changed` event lands while list_jobs is in flight,
  // the event is newer, so the fetch result must not clobber it (hence `gotEvent`).
  const unlisteners = Promise.all([
    listen<Job[]>(EVT.changed, (e) => {
      gotEvent = true;
      setJobs(e.payload);
    }),
    listen<Progress>(EVT.progress, (e) => setProgress(e.payload)),
  ]);

  api.listJobs().then((initial) => {
    if (!disposed && !gotEvent) setJobs(initial);
  });

  return () => {
    disposed = true;
    // listen() is async, so cleanup has to wait for the handles before calling them.
    unlisteners.then((fns) => fns.forEach((off) => off()));
  };
}

const subscribeJobs = (l: Listener) => {
  jobListeners.add(l);
  return () => jobListeners.delete(l);
};
const getJobs = () => jobs;

export function useJobs(): Job[] {
  return useSyncExternalStore(subscribeJobs, getJobs);
}

export function useProgress(id: string): Progress | undefined {
  // useCallback matters here: a fresh `subscribe` identity on each render makes
  // useSyncExternalStore unsubscribe and resubscribe on every render.
  const subscribe = useCallback(
    (l: Listener) => {
      let set = progressListeners.get(id);
      if (!set) progressListeners.set(id, (set = new Set()));
      set.add(l);
      return () => {
        set.delete(l);
        if (set.size === 0) progressListeners.delete(id);
      };
    },
    [id],
  );
  // Returns the stored object itself (stable reference), so React bails out when nothing changed.
  const get = useCallback(() => progress.get(id), [id]);
  return useSyncExternalStore(subscribe, get);
}
