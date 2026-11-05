import { AlertTriangle, CheckCircle2, XCircle } from "lucide-react";
import { Progress } from "@/components/ui/progress";
import type { Job } from "@/lib/api";
import { formatBytes, formatEta, formatSpeed } from "@/lib/format";
import { useProgress } from "@/lib/queueStore";

/** Its own component so the progress subscription doesn't re-render the form. */
function LiveProgress({ job }: { job: Job }) {
  const p = useProgress(job.id);
  const live = job.state === "uploading" && p;
  const sent = live ? p.bytesSent : job.bytesSent;
  const bps = live ? p.bps : 0;
  const pct = job.size ? (sent / job.size) * 100 : 0;

  return (
    <div className="space-y-2.5 rounded-xl border border-border bg-card p-4">
      <div className="flex items-baseline justify-between">
        <span className="text-2xl font-semibold tabular-nums">{pct.toFixed(pct < 10 ? 1 : 0)}%</span>
        <span className="text-xs text-muted-foreground tabular-nums">
          {job.state === "uploading" ? formatSpeed(bps) : job.state === "paused" ? "Paused" : "Waiting…"}
        </span>
      </div>
      <Progress value={pct} className="h-1.5 [&>[data-slot=progress-indicator]]:bg-brand" />
      <div className="flex justify-between text-xs text-muted-foreground tabular-nums">
        <span>
          {formatBytes(sent)} of {formatBytes(job.size)}
        </span>
        <span>{live ? formatEta(job.size - sent, bps) : ""}</span>
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
      <div className="flex gap-2.5 rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-sm">
        <XCircle className="mt-0.5 size-4 shrink-0 text-destructive" />
        <div className="min-w-0 space-y-1">
          <p className="font-medium text-destructive">Upload failed</p>
          <p className="text-xs break-words text-destructive/80 select-text">{job.error}</p>
          {job.sessionUri && <p className="text-xs text-muted-foreground">Retry resumes from where it stopped.</p>}
        </div>
      </div>
    );
  }
  if (job.state === "done") {
    return (
      <div className="space-y-2">
        <div className="flex gap-2.5 rounded-xl border border-success/30 bg-success/10 p-3 text-sm">
          <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-success" />
          <div>
            <p className="font-medium text-success">Uploaded</p>
            <p className="text-xs text-muted-foreground select-text">youtu.be/{job.videoId}</p>
          </div>
        </div>
        {job.warning && (
          <div className="flex gap-2.5 rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-300">
            <AlertTriangle className="mt-0.5 size-4 shrink-0" />
            <span className="break-words">{job.warning}</span>
          </div>
        )}
      </div>
    );
  }
  return null;
}
