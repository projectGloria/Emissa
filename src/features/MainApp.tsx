import { useEffect, useEffectEvent, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Upload } from "lucide-react";
import { toast } from "sonner";
import { api, errorMessage, EVT, isAppError, type Channel, type Finished } from "@/lib/api";
import { startQueueSync, useJobs } from "@/lib/queueStore";
import { addFiles } from "@/lib/files";
import { QueueSidebar } from "./queue/QueueSidebar";
import { DropZone } from "./queue/DropZone";
import { JobPanel } from "./editor/JobPanel";

export function MainApp({ onSignedOut }: { onSignedOut: () => void }) {
  const jobs = useJobs();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [channel, setChannel] = useState<Channel | null>(null);
  const [dragging, setDragging] = useState(false);

  // Derived, not synced: if the selected job is removed, fall back to the first one
  // during render. No effect running "if selected disappeared, reselect".
  const selected = jobs.find((j) => j.id === selectedId) ?? jobs[0] ?? null;

  // useEffectEvent: always sees the latest `onSignedOut` prop without becoming an effect
  // dependency. App passes a fresh inline arrow every render, which would otherwise
  // re-run the channel fetch on every parent render.
  const handleSignedOut = useEffectEvent(onSignedOut);

  useEffect(() => startQueueSync(), []);

  useEffect(() => {
    let ignore = false;
    api
      .getChannel()
      .then((c) => !ignore && setChannel(c))
      .catch((e) => {
        if (ignore) return;
        if (isAppError(e) && e.kind === "notSignedIn") handleSignedOut();
        else toast.error("Couldn't load your channel", { description: errorMessage(e) });
      });
    return () => {
      ignore = true;
    };
  }, []);

  useEffect(() => {
    const p = listen<Finished>(EVT.finished, ({ payload: f }) => {
      if (f.ok && !f.message) toast.success("Uploaded", { description: f.title });
      else if (f.ok) toast.warning("Uploaded with a warning", { description: f.message });
      else toast.error(`Upload failed: ${f.title}`, { description: f.message });
    });
    return () => void p.then((off) => off());
  }, []);

  useEffect(() => {
    // Native drag-drop. Tauri hands us real filesystem paths, which the HTML5 DataTransfer
    // API deliberately hides. Rust then filters out non-video files.
    const p = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter" || payload.type === "over") setDragging(true);
      else if (payload.type === "leave") setDragging(false);
      else if (payload.type === "drop") {
        setDragging(false);
        void addFiles(payload.paths);
      }
    });
    return () => void p.then((off) => off());
  }, []);

  const signOut = async () => {
    try {
      await api.signOut();
    } finally {
      onSignedOut();
    }
  };

  return (
    <div className="grid h-full grid-cols-[300px_1fr]">
      <QueueSidebar
        jobs={jobs}
        selectedId={selected?.id ?? null}
        onSelect={setSelectedId}
        channel={channel}
        onSignOut={signOut}
      />
      <main className="min-w-0 overflow-hidden">
        {/* key = job id: switching jobs remounts the editor, resetting its local draft
            state from the new job's props. No useEffect "sync props to state" needed. */}
        {selected ? <JobPanel key={selected.id} job={selected} /> : <DropZone />}
      </main>

      {dragging && (
        <div className="pointer-events-none fixed inset-0 z-50 grid place-items-center bg-background/70 backdrop-blur-sm">
          <div className="flex flex-col items-center gap-3 rounded-2xl border-2 border-dashed border-brand/60 px-16 py-12">
            <Upload className="size-8 text-brand" />
            <p className="text-sm font-medium">Drop videos to add them to the queue</p>
          </div>
        </div>
      )}
    </div>
  );
}
