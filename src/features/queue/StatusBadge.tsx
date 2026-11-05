import { Badge } from "@/components/ui/badge";
import type { JobState } from "@/lib/api";
import { cn } from "@/lib/utils";

const LABEL: Record<JobState, string> = {
  draft: "Draft",
  queued: "Queued",
  uploading: "Uploading",
  paused: "Paused",
  done: "Uploaded",
  failed: "Failed",
};

const TONE: Record<JobState, string> = {
  draft: "bg-muted text-muted-foreground",
  queued: "bg-muted text-foreground",
  uploading: "bg-brand/15 text-brand",
  paused: "bg-amber-500/15 text-amber-400",
  done: "bg-success/15 text-success",
  failed: "bg-destructive/15 text-destructive",
};

export function StatusBadge({ state, className }: { state: JobState; className?: string }) {
  return (
    <Badge variant="secondary" className={cn("gap-1.5", TONE[state], className)}>
      {state === "uploading" && <span className="size-1.5 animate-pulse rounded-full bg-current" />}
      {LABEL[state]}
    </Badge>
  );
}
