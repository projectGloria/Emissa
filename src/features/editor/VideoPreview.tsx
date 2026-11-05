import { useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { FileVideo } from "lucide-react";

export function VideoPreview({ path }: { path: string }) {
  // No reset effect needed: JobPanel is keyed by job id, so a new file gets a fresh `failed`.
  const [failed, setFailed] = useState(false);

  if (failed) {
    return (
      <div className="grid aspect-video place-items-center rounded-xl border border-border bg-muted/30 text-center">
        <div className="space-y-1 px-6">
          <FileVideo className="mx-auto size-6 text-muted-foreground" />
          {/* WebView2 can't decode every container (MKV/AVI/…); YouTube can. */}
          <p className="text-xs text-muted-foreground">No preview for this format. It will still upload fine.</p>
        </div>
      </div>
    );
  }

  return (
    <video
      // asset:// streams straight from disk, with HTTP range requests for seeking.
      // No bytes go through IPC.
      src={convertFileSrc(path)}
      controls
      preload="metadata"
      onError={() => setFailed(true)}
      className="aspect-video w-full rounded-xl bg-black"
    />
  );
}
