import type { ReactNode } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ImagePlus, X } from "lucide-react";
import { toast } from "sonner";
import { api, errorMessage, type Job } from "@/lib/api";

interface Props {
  job: Job;
  /** Rendered flush under the image inside the same card (upload progress / result). */
  status?: ReactNode;
}

export function ThumbnailPicker({ job, status }: Props) {
  const disabled = job.state === "done";

  const pick = async () => {
    const path = await open({ multiple: false, filters: [{ name: "Image", extensions: ["jpg", "jpeg", "png"] }] });
    if (!path) return;
    try {
      await api.setThumbnail(job.id, path);
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  return (
    <div className="space-y-2">
      <div className="flex items-baseline justify-between text-xs">
        <span className="font-medium">Thumbnail</span>
        <span className="text-muted-foreground">JPG/PNG · ≤2 MB · 1280×720</span>
      </div>

      {/* One card: the image on top, status glued underneath. overflow-hidden clips the image
          and the progress bar to the card's rounded corners. */}
      <div className="overflow-hidden rounded-xl border border-border bg-card">
        {job.thumbnail ? (
          <div className="group relative">
            <img src={convertFileSrc(job.thumbnail)} alt="" className="aspect-video w-full object-cover" />
            {!disabled && (
              <div className="absolute inset-0 flex items-center justify-center gap-2 bg-black/50 opacity-0 transition-opacity group-hover:opacity-100">
                <button type="button" onClick={pick} className="rounded-md bg-background/90 px-2.5 py-1 text-xs font-medium">
                  Change
                </button>
                <button
                  type="button"
                  aria-label="Remove thumbnail"
                  onClick={() => api.setThumbnail(job.id, null).catch((e) => toast.error(errorMessage(e)))}
                  className="rounded-md bg-background/90 p-1.5"
                >
                  <X className="size-3.5" />
                </button>
              </div>
            )}
          </div>
        ) : (
          <button
            type="button"
            onClick={pick}
            disabled={disabled}
            className="flex aspect-video w-full flex-col items-center justify-center gap-2 bg-muted/20 text-muted-foreground transition-colors hover:bg-muted/40 hover:text-foreground disabled:pointer-events-none disabled:opacity-50"
          >
            <ImagePlus className="size-5" />
            <span className="text-xs">{disabled ? "No custom thumbnail" : "Add a custom thumbnail"}</span>
          </button>
        )}
        {status}
      </div>
    </div>
  );
}
