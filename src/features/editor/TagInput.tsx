import { useState, type KeyboardEvent } from "react";
import { X } from "lucide-react";
import { LIMITS, tagsLen } from "@/lib/metadata";
import { cn } from "@/lib/utils";

export function TagInput({ tags, onChange }: { tags: string[]; onChange: (tags: string[]) => void }) {
  // Only the half-typed text is local; committed tags live in the parent's draft.
  const [text, setText] = useState("");
  const used = tagsLen(tags);

  const commit = (raw: string) => {
    const incoming = raw.split(",").map((t) => t.trim()).filter(Boolean);
    if (!incoming.length) return;
    const seen = new Set(tags.map((t) => t.toLowerCase()));
    const next = [...tags];
    for (const t of incoming) {
      if (!seen.has(t.toLowerCase())) {
        seen.add(t.toLowerCase());
        next.push(t);
      }
    }
    onChange(next);
    setText("");
  };

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      commit(text);
    } else if (e.key === "Backspace" && !text && tags.length) {
      onChange(tags.slice(0, -1));
    }
  };

  return (
    <div>
      <div className="flex min-h-10 flex-wrap items-center gap-1.5 rounded-lg border border-input bg-input/30 px-2 py-1.5 focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50">
        {tags.map((t) => (
          <span key={t} className="inline-flex items-center gap-1 rounded-md bg-muted py-0.5 pr-1 pl-2 text-xs">
            {t}
            <button
              type="button"
              aria-label={`Remove ${t}`}
              className="rounded p-0.5 text-muted-foreground hover:bg-background/60 hover:text-foreground"
              onClick={() => onChange(tags.filter((x) => x !== t))}
            >
              <X className="size-3" />
            </button>
          </span>
        ))}
        <input
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={onKeyDown}
          onBlur={() => commit(text)}
          onPaste={(e) => {
            const pasted = e.clipboardData.getData("text");
            if (pasted.includes(",")) {
              e.preventDefault();
              commit(text + pasted);
            }
          }}
          placeholder={tags.length ? "" : "Type a tag and press Enter"}
          className="min-w-24 flex-1 bg-transparent px-1 text-sm outline-none placeholder:text-muted-foreground"
        />
      </div>
      <div className={cn("mt-1.5 text-right text-xs tabular-nums", used > LIMITS.tags ? "text-destructive" : "text-muted-foreground")}>
        {used}/{LIMITS.tags}
      </div>
    </div>
  );
}
