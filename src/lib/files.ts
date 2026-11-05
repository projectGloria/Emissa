import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { api, errorMessage } from "./api";
import { VIDEO_EXTENSIONS } from "./metadata";

export async function addFiles(paths: string[]) {
  if (!paths.length) return;
  try {
    const rejected = await api.addFiles(paths);
    if (rejected.length) {
      toast.warning(`Skipped ${rejected.length} file${rejected.length > 1 ? "s" : ""}`, {
        description: `Not a video, or already in the queue: ${rejected.slice(0, 3).join(", ")}${rejected.length > 3 ? "…" : ""}`,
      });
    }
  } catch (e) {
    toast.error(errorMessage(e));
  }
}

export async function pickFiles() {
  const picked = await open({ multiple: true, filters: [{ name: "Videos", extensions: VIDEO_EXTENSIONS }] });
  if (picked) await addFiles(picked);
}
