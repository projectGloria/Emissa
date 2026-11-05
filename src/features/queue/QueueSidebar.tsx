import { CloudUpload, Plus } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { AccountMenu } from "../auth/AccountMenu";
import { pickFiles } from "@/lib/files";
import { QueueItem } from "./QueueItem";
import { api, errorMessage, type Channel, type Job } from "@/lib/api";

interface Props {
  jobs: Job[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  channel: Channel | null;
  onSignOut: () => void;
}

export function QueueSidebar({ jobs, selectedId, onSelect, channel, onSignOut }: Props) {
  // Derived counts: computed during render from the snapshot, so they can never go stale.
  const drafts = jobs.filter((j) => j.state === "draft").length;
  const done = jobs.filter((j) => j.state === "done").length;

  const uploadAll = async () => {
    try {
      const skipped = await api.startAll();
      if (skipped) toast.warning(`${skipped} draft${skipped > 1 ? "s" : ""} need a valid title first`);
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  return (
    <aside className="flex min-h-0 flex-col border-r border-sidebar-border bg-sidebar">
      {/* The logo moved to the custom title bar; this row just needs breathing room under it. */}
      <div className="flex gap-2 px-3 pt-3 pb-3">
        <Button variant="secondary" className="flex-1" onClick={() => void pickFiles()}>
          <Plus /> Add videos
        </Button>
        <Button className="flex-1 bg-brand text-brand-foreground hover:bg-brand/85" disabled={!drafts} onClick={uploadAll}>
          <CloudUpload /> Upload all{drafts > 0 && ` (${drafts})`}
        </Button>
      </div>

      <div className="flex items-center justify-between px-4 pb-1.5 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
        <span>Queue · {jobs.length}</span>
        {done > 0 && (
          <button className="normal-case tracking-normal hover:text-foreground" onClick={() => void api.clearDone()}>
            Clear uploaded
          </button>
        )}
      </div>

      {/* Native overflow, not Radix ScrollArea: its viewport wraps children in a
          `display: table` div that sizes to content, which defeats `truncate` and
          pushed the status badge off the edge. The global ::-webkit-scrollbar styles cover the looks. */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        <ul className="space-y-0.5 px-2 pb-2">
          {jobs.length === 0 && (
            <li className="px-2 py-6 text-center text-xs text-muted-foreground">Nothing queued yet.</li>
          )}
          {jobs.map((job) => (
            <QueueItem key={job.id} job={job} selected={job.id === selectedId} onSelect={onSelect} />
          ))}
        </ul>
      </div>

      <div className="border-t border-sidebar-border p-2">
        <AccountMenu channel={channel} onSignOut={onSignOut} />
      </div>
    </aside>
  );
}
