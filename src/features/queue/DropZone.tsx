import { Film, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { pickFiles } from "@/lib/files";

export function DropZone() {
  return (
    <div className="grid h-full place-items-center p-10">
      <div className="flex w-full max-w-lg flex-col items-center gap-5 rounded-2xl border border-dashed border-border px-10 py-16 text-center">
        <div className="grid size-14 place-items-center rounded-2xl bg-muted">
          <Film className="size-6 text-muted-foreground" />
        </div>
        <div className="space-y-1.5">
          <h2 className="text-lg font-semibold">Drop videos here</h2>
          <p className="text-sm text-muted-foreground">
            Add as many as you like. Each one gets its own title, description and thumbnail, and they upload one at a time.
          </p>
        </div>
        <Button onClick={() => void pickFiles()}>
          <Plus /> Choose files
        </Button>
      </div>
    </div>
  );
}
