import { useId } from "react";
import { Globe, Link2, Lock } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import type { Metadata, Privacy } from "@/lib/api";
import { byteLen, CATEGORIES, charLen, LIMITS } from "@/lib/metadata";
import { cn } from "@/lib/utils";
import { TagInput } from "./TagInput";

const PRIVACY: { value: Privacy; label: string; icon: typeof Lock; hint: string }[] = [
  { value: "private", label: "Private", icon: Lock, hint: "Only you" },
  { value: "unlisted", label: "Unlisted", icon: Link2, hint: "Anyone with the link" },
  { value: "public", label: "Public", icon: Globe, hint: "Everyone" },
];

function Counter({ n, max }: { n: number; max: number }) {
  return <span className={cn("tabular-nums", n > max ? "text-destructive" : "text-muted-foreground")}>{n}/{max}</span>;
}

interface Props {
  draft: Metadata;
  onChange: (patch: Partial<Metadata>) => void;
  locked: boolean;
}

export function MetadataEditor({ draft, onChange, locked }: Props) {
  const id = useId(); // stable, SSR-safe unique prefix for label/htmlFor pairs

  return (
    // <fieldset disabled> natively disables every descendant control (Radix triggers are
    // <button>s, so they're covered too). That beats threading `disabled` through each field.
    <fieldset disabled={locked} className="min-w-0 space-y-6">
      {locked && (
        <p className="rounded-lg border border-border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
          Details are locked once the upload has started. Cancel the upload to edit them.
        </p>
      )}

      <div className="space-y-2">
        <div className="flex items-baseline justify-between text-xs">
          <Label htmlFor={`${id}-title`}>Title</Label>
          <Counter n={charLen(draft.title)} max={LIMITS.title} />
        </div>
        <Input
          id={`${id}-title`}
          value={draft.title}
          onChange={(e) => onChange({ title: e.target.value })}
          placeholder="Add a title that describes your video"
          className="h-10 text-base"
        />
      </div>

      <div className="space-y-2">
        <div className="flex items-baseline justify-between text-xs">
          <Label htmlFor={`${id}-desc`}>Description</Label>
          {/* Bytes, not chars: YouTube's 5000 limit is on the UTF-8 encoding. */}
          <Counter n={byteLen(draft.description)} max={LIMITS.descriptionBytes} />
        </div>
        <Textarea
          id={`${id}-desc`}
          value={draft.description}
          onChange={(e) => onChange({ description: e.target.value })}
          placeholder="Tell viewers about your video"
          className="min-h-40 resize-y"
        />
      </div>

      <div className="space-y-2">
        <Label className="text-xs">Tags</Label>
        <TagInput tags={draft.tags} onChange={(tags) => onChange({ tags })} />
      </div>

      <div className="space-y-2">
        <Label className="text-xs">Visibility</Label>
        <div role="radiogroup" className="grid grid-cols-3 gap-2">
          {PRIVACY.map(({ value, label, icon: Icon, hint }) => {
            const active = draft.privacy === value;
            return (
              <button
                key={value}
                type="button"
                role="radio"
                aria-checked={active}
                onClick={() => onChange({ privacy: value })}
                className={cn(
                  "flex flex-col items-start gap-1 rounded-lg border p-3 text-left transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50",
                  active ? "border-brand/60 bg-brand/10" : "border-border hover:bg-muted/50",
                )}
              >
                <Icon className={cn("size-4", active ? "text-brand" : "text-muted-foreground")} />
                <span className="text-sm font-medium">{label}</span>
                <span className="text-xs text-muted-foreground">{hint}</span>
              </button>
            );
          })}
        </div>
        {draft.privacy !== "private" && (
          <p className="text-xs text-muted-foreground">
            Heads-up: while your Google API project is unverified, YouTube forces API uploads to private.
            You can change visibility in Studio afterwards.
          </p>
        )}
      </div>

      <div className="grid gap-6 @lg:grid-cols-2">
        <div className="space-y-2">
          <Label className="text-xs">Category</Label>
          <Select value={draft.categoryId} onValueChange={(categoryId) => onChange({ categoryId })}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {CATEGORIES.map((c) => (
                <SelectItem key={c.id} value={c.id}>
                  {c.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2">
          <Label className="text-xs">Audience</Label>
          <label className="flex h-9 items-center justify-between gap-3 rounded-lg border border-border px-3 text-sm">
            Made for kids
            <Switch checked={draft.madeForKids} onCheckedChange={(madeForKids) => onChange({ madeForKids })} />
          </label>
        </div>
      </div>
    </fieldset>
  );
}
