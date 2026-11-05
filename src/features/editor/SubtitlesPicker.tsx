import { open } from "@tauri-apps/plugin-dialog";
import { Captions, KeyRound, Plus, X } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { api, errorMessage, type Caption, type Job } from "@/lib/api";
import { CAPTION_EXTENSIONS, guessLanguage, LANGUAGES } from "@/lib/metadata";

const baseName = (p: string) => p.split(/[\\/]/).pop() ?? p;
const labelOf = (code: string) => LANGUAGES.find((l) => l.code === code)?.label ?? code;

interface Props {
  job: Job;
  /** null = still checking the grant. */
  canCaption: boolean | null;
  onReauth: () => void;
}

export function SubtitlesPicker({ job, canCaption, onReauth }: Props) {
  const disabled = job.state === "done";
  // No local copy of the list: Rust's snapshot is the source of truth, and every
  // edit round-trips through set_captions. A queue://changed event re-renders us.
  const save = (next: Caption[]) => api.setCaptions(job.id, next).catch((e) => toast.error(errorMessage(e)));

  const add = async () => {
    const picked = await open({ multiple: true, filters: [{ name: "Subtitles", extensions: CAPTION_EXTENSIONS }] });
    if (!picked?.length) return;
    const known = new Set(job.captions.map((c) => c.path));
    const added = picked
      .filter((p) => !known.has(p))
      .map((path) => {
        const language = guessLanguage(baseName(path));
        return { path, language, name: labelOf(language) };
      });
    void save([...job.captions, ...added]);
  };

  const patch = (i: number, p: Partial<Caption>) => save(job.captions.map((c, j) => (j === i ? { ...c, ...p } : c)));

  return (
    <div className="space-y-2">
      <div className="flex items-baseline justify-between text-xs">
        <span className="font-medium">Subtitles</span>
        <span className="text-muted-foreground">.srt .vtt .sbv … · 400 quota each</span>
      </div>

      {canCaption === false && (
        // The token predates the caption scope (or the user unticked it). Explain instead of
        // letting every caption fail after a long upload.
        <div className="space-y-2 rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-200">
          <p>
            Subtitle upload needs one more Google permission. Sign in again and keep{" "}
            <b>"See, edit, and permanently delete your YouTube videos…"</b> ticked.
          </p>
          <Button size="sm" variant="secondary" onClick={onReauth}>
            <KeyRound /> Sign in again
          </Button>
        </div>
      )}

      {job.captions.length > 0 && (
        <ul className="space-y-2">
          {job.captions.map((c, i) => (
            <li key={c.path} className="space-y-2 rounded-xl border border-border p-2.5">
              <div className="flex items-center gap-2">
                <Captions className="size-4 shrink-0 text-muted-foreground" />
                <span className="min-w-0 flex-1 truncate text-xs" title={c.path}>
                  {baseName(c.path)}
                </span>
                {!disabled && (
                  <button
                    type="button"
                    aria-label="Remove subtitles"
                    className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
                    onClick={() => void save(job.captions.filter((_, j) => j !== i))}
                  >
                    <X className="size-3.5" />
                  </button>
                )}
              </div>
              <div className="grid grid-cols-2 gap-2">
                <Select
                  disabled={disabled}
                  value={c.language}
                  // Rename along with the language only if the user never customised the name.
                  onValueChange={(language) =>
                    void patch(i, { language, ...(c.name === labelOf(c.language) ? { name: labelOf(language) } : {}) })
                  }
                >
                  <SelectTrigger size="sm" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {LANGUAGES.map((l) => (
                      <SelectItem key={l.code} value={l.code}>
                        {l.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Input
                  // Uncontrolled + commit on blur: one IPC call per edit, not per keystroke.
                  // `key` includes the saved name so an external change resets the field.
                  key={`${c.path}:${c.name}`}
                  disabled={disabled}
                  defaultValue={c.name}
                  placeholder="Track name"
                  className="h-7 text-xs"
                  onBlur={(e) => e.target.value !== c.name && void patch(i, { name: e.target.value })}
                />
              </div>
            </li>
          ))}
        </ul>
      )}

      {!disabled && (
        <Button variant="outline" size="sm" className="w-full" onClick={() => void add()}>
          <Plus /> Add subtitle file
        </Button>
      )}
    </div>
  );
}
