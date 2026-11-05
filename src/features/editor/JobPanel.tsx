import { useCallback, useEffect, useRef, useState } from "react";
import { ExternalLink, Pause, Pencil, Play, RotateCw, Trash2, Upload, X } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { api, errorMessage, type Job, type Metadata } from "@/lib/api";
import { validate } from "@/lib/metadata";
import { StatusBadge } from "../queue/StatusBadge";
import { MetadataEditor } from "./MetadataEditor";
import { ThumbnailPicker } from "./ThumbnailPicker";
import { UploadStatus } from "./UploadStatus";
import { SubtitlesPicker } from "./SubtitlesPicker";

const run = (p: Promise<unknown>) => p.catch((e) => toast.error(errorMessage(e)));

/**
 * Debounced persistence. Refs, not state: the pending payload and the timer are
 * plumbing, and storing them in state would re-render on every keystroke for nothing.
 */
function useDebouncedSave(id: string, delay = 400) {
  const timer = useRef<number | undefined>(undefined);
  const pending = useRef<Metadata | null>(null);

  const flush = useCallback(async () => {
    window.clearTimeout(timer.current);
    const m = pending.current;
    pending.current = null;
    if (m) await api.updateMetadata(id, m);
  }, [id]);

  const schedule = useCallback(
    (m: Metadata) => {
      pending.current = m;
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => void run(flush()), delay);
    },
    [flush, delay],
  );

  // Flush on unmount: switching jobs remounts this panel (key=id), and the last
  // keystrokes inside the debounce window must not be dropped.
  useEffect(() => () => void run(flush()), [flush]);

  return { schedule, flush };
}

interface JobPanelProps {
  job: Job;
  canCaption: boolean | null;
  onReauth: () => void;
}

export function JobPanel({ job, canCaption, onReauth }: JobPanelProps) {
  // Initial state from props is legit here ONLY because the parent keys us by job.id.
  // This component instance belongs to exactly one job, and this editor is the only writer.
  const [draft, setDraft] = useState<Metadata>(job.metadata);
  const { schedule, flush } = useDebouncedSave(job.id);

  // Metadata goes to YouTube when the session is created, so it can't change after that.
  const locked = job.sessionUri !== null || ["queued", "uploading", "done"].includes(job.state);
  const invalid = validate(draft); // derived each render, so no `errors` state to keep in sync

  const onChange = (patch: Partial<Metadata>) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    schedule(next); // in the event handler, not a useEffect on `draft`, so there's no extra render pass
  };

  const upload = () =>
    run(
      (async () => {
        await flush(); // make sure Rust validates what the user actually sees
        await api.start(job.id);
      })(),
    );

  return (
    <div className="flex h-full flex-col">
      <header className="flex h-14 shrink-0 items-center justify-between gap-4 border-b border-border px-6">
        <div className="flex min-w-0 items-center gap-3">
          <h1 className="truncate text-sm font-semibold">{job.fileName}</h1>
          <StatusBadge state={job.state} />
        </div>
        <Actions job={job} invalid={invalid} onUpload={upload} />
      </header>

      <div className="@container min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto grid max-w-5xl gap-6 p-6 @3xl:grid-cols-[minmax(0,1fr)_300px]">
          <MetadataEditor draft={draft} onChange={onChange} locked={locked} />
          <aside className="space-y-4 @3xl:order-last">
            {/* Status rides on the thumbnail card's bottom edge, so the two read as one unit. */}
            <ThumbnailPicker job={job} status={<UploadStatus job={job} />} />
            <SubtitlesPicker job={job} canCaption={canCaption} onReauth={onReauth} />
          </aside>
        </div>
      </div>
    </div>
  );
}

function Actions({ job, invalid, onUpload }: { job: Job; invalid: string | null; onUpload: () => void }) {
  const remove = (
    <Button variant="ghost" size="icon" aria-label="Remove from queue" onClick={() => run(api.remove(job.id))}>
      <Trash2 />
    </Button>
  );
  const cancel = (
    <Button variant="ghost" onClick={() => run(api.cancel(job.id))}>
      <X /> Cancel
    </Button>
  );

  switch (job.state) {
    case "draft":
      return (
        <div className="flex items-center gap-1.5">
          {remove}
          <Tooltip>
            {/* Disabled buttons swallow pointer events, so the span gives the tooltip a hover target. */}
            <TooltipTrigger asChild>
              <span tabIndex={invalid ? 0 : -1}>
                <Button className="bg-brand text-brand-foreground hover:bg-brand/85" disabled={!!invalid} onClick={onUpload}>
                  <Upload /> Upload
                </Button>
              </span>
            </TooltipTrigger>
            {invalid && <TooltipContent>{invalid}</TooltipContent>}
          </Tooltip>
        </div>
      );
    case "queued":
    case "uploading":
      return (
        <div className="flex items-center gap-1.5">
          {cancel}
          <Button variant="secondary" onClick={() => run(api.pause(job.id))}>
            <Pause /> Pause
          </Button>
        </div>
      );
    case "paused":
      return (
        <div className="flex items-center gap-1.5">
          {cancel}
          <Button onClick={() => run(api.resume(job.id))}>
            <Play /> Resume
          </Button>
        </div>
      );
    case "failed":
      return (
        <div className="flex items-center gap-1.5">
          {remove}
          <Button variant="ghost" onClick={() => run(api.cancel(job.id))}>
            <Pencil /> Edit
          </Button>
          <Button onClick={() => run(api.retry(job.id))}>
            <RotateCw /> Retry
          </Button>
        </div>
      );
    case "done":
      return (
        <div className="flex items-center gap-1.5">
          {remove}
          <Button variant="secondary" onClick={() => job.videoId && run(api.openInStudio(job.videoId))}>
            <ExternalLink /> Open in Studio
          </Button>
        </div>
      );
  }
}
