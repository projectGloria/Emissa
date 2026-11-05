import { useProgress } from "@/lib/queueStore";
import { formatBytes } from "@/lib/format";
import type { Job } from "@/lib/api";
import { cn } from "@/lib/utils";
import { StatusBadge } from "./StatusBadge";

/** Isolated so the 10 Hz progress subscription re-renders only this bar, not the row. */
function RowProgress({ job }: { job: Job }) {
  const p = useProgress(job.id);
  // Live tick if we have one; otherwise the last server-confirmed offset from the snapshot.
  const sent = job.state === "uploading" && p ? p.bytesSent : job.bytesSent;
  const pct = job.size ? (sent / job.size) * 100 : 0;
  return (
    <div className="mt-2 h-0.5 overflow-hidden rounded-full bg-muted">
      <div
        className={cn("h-full rounded-full transition-[width] duration-200", job.state === "paused" ? "bg-amber-400" : "bg-brand")}
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}

export function QueueItem({ job, selected, onSelect }: { job: Job; selected: boolean; onSelect: (id: string) => void }) {
  const showBar = job.state === "uploading" || job.state === "paused" || (job.state === "queued" && job.bytesSent > 0);
  return (
    <li>
      <button
        onClick={() => onSelect(job.id)}
        className={cn(
          "w-full rounded-lg px-2.5 py-2 text-left transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
          selected ? "bg-muted" : "hover:bg-muted/50",
        )}
      >
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <div className="truncate text-sm font-medium">{job.metadata.title || job.fileName}</div>
            <div className="truncate text-xs text-muted-foreground">
              {job.fileName} · {formatBytes(job.size)}
            </div>
          </div>
          <StatusBadge state={job.state} className="shrink-0" />
        </div>
        {showBar && <RowProgress job={job} />}
      </button>
    </li>
  );
}
