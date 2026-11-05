import { AlertTriangle, CheckCircle2, XCircle } from "lucide-react";
import type { Job } from "@/lib/api";
import { formatBytes, formatEta, formatSpeed } from "@/lib/format";
import { useProgress } from "@/lib/queueStore";
import { cn } from "@/lib/utils";

// Compact strips designed to sit flush under the thumbnail inside ThumbnailPicker's card.
// Each draws its own top border, so for drafts nothing renders and the card ends at the image.

/** Its own component so the 10 Hz progress subscription doesn't re-render the form. */
function LiveProgress({ job }: { job: Job }) {
  const p = useProgress(job.id);
  const live = job.state === "uploading" && p;
  const sent = live ? p.bytesSent : job.bytesSent;
  const bps = live ? p.bps : 0;
  const pct = job.size ? (sent / job.size) * 100 : 0;
  const right = job.state === "uploading" ? formatSpeed(bps) : job.state === "paused" ? "Paused" : "Waiting…";

  return (
    <div className="border-t border-border">
      {/* A 2px edge-to-edge bar: it reads as part of the card instead of a widget inside it. */}
      <div className="h-0.5 bg-muted">
        <div
          className={cn("h-full transition-[width] duration-200", job.state === "paused" ? "bg-amber-400" : "bg-brand")}
          style={{ width: `${pct}%` }}
        />
      </div>
      <div className="flex items-baseline justify-between gap-2 px-3 py-2 text-xs tabular-nums">
        <span>
          <span className="font-semibold">{pct.toFixed(pct < 10 ? 1 : 0)}%</span>
          <span className="text-muted-foreground">
            {" · "}
            {formatBytes(sent)} / {formatBytes(job.size)}
          </span>
        </span>
        <span className="text-muted-foreground">
          {right}
          {live && bps > 1 && ` · ${formatEta(job.size - sent, bps)}`}
        </span>
      </div>
    </div>
  );
}

export function UploadStatus({ job }: { job: Job }) {
  if (job.state === "uploading" || job.state === "paused" || job.state === "queued") {
    return <LiveProgress job={job} />;
  }
  if (job.state === "failed") {
    return (
      <div className="flex gap-2 border-t border-destructive/30 bg-destructive/10 px-3 py-2 text-xs">
        <XCircle className="mt-px size-3.5 shrink-0 text-destructive" />
        <div className="min-w-0 space-y-0.5">
          <p className="font-medium text-destructive">Upload failed</p>
          <p className="break-words text-destructive/80 select-text">{job.error}</p>
          {job.sessionUri && <p className="text-muted-foreground">Retry resumes from where it stopped.</p>}
        </div>
      </div>
    );
  }
  if (job.state === "done") {
    return (
      <>
        <div className="flex items-center gap-2 border-t border-success/30 bg-success/10 px-3 py-2 text-xs">
          <CheckCircle2 className="size-3.5 shrink-0 text-success" />
          <span className="font-medium text-success">Uploaded</span>
          <span className="ml-auto truncate text-muted-foreground select-text">youtu.be/{job.videoId}</span>
        </div>
        {job.warning && (
          <div className="flex gap-2 border-t border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-300">
            <AlertTriangle className="mt-px size-3.5 shrink-0" />
            <span className="break-words">{job.warning}</span>
          </div>
        )}
      </>
    );
  }
  return null;
}
