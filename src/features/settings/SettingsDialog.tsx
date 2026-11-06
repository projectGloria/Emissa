import { useEffect, useState, useTransition } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { open as openFile } from "@tauri-apps/plugin-dialog";
import { FileJson, FolderOpen, Loader2, LogOut, RotateCcw } from "lucide-react";
import { toast } from "sonner";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { Logo } from "@/components/app/Logo";
import { api, errorMessage, type Channel, type Privacy, type Settings, type SetupStatus, type VideoDefaults } from "@/lib/api";
import { byteLen, CATEGORIES, LIMITS } from "@/lib/metadata";
import { TagInput } from "../editor/TagInput";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  channel: Channel | null;
  onSignOut: () => void;
  onStatusChange: (s: SetupStatus) => void;
}

export function SettingsDialog({ open, onOpenChange, channel, onSignOut, onStatusChange }: Props) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {/* Radix unmounts DialogContent while closed, so every tab below mounts fresh on each
          open. Their "fetch on mount" effects therefore always load current data, with no
          reset logic needed. */}
      <DialogContent className="max-h-[85vh] overflow-hidden p-0 sm:max-w-2xl">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>Options</DialogTitle>
          <DialogDescription className="sr-only">Emissa preferences</DialogDescription>
        </DialogHeader>
        <Tabs defaultValue="general" className="min-h-0 gap-0">
          <div className="px-6">
            <TabsList>
              <TabsTrigger value="general">General</TabsTrigger>
              <TabsTrigger value="defaults">New video defaults</TabsTrigger>
              <TabsTrigger value="account">Account</TabsTrigger>
              <TabsTrigger value="about">About</TabsTrigger>
            </TabsList>
          </div>
          <div className="max-h-[calc(85vh-7.5rem)] overflow-y-auto px-6 pt-5 pb-6">
            <TabsContent value="general">
              <GeneralTab />
            </TabsContent>
            <TabsContent value="defaults">
              <DefaultsTab />
            </TabsContent>
            <TabsContent value="account">
              <AccountTab channel={channel} onSignOut={onSignOut} onStatusChange={onStatusChange} />
            </TabsContent>
            <TabsContent value="about">
              <AboutTab />
            </TabsContent>
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}

// ---------- General ----------

function GeneralTab() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [isSaving, startSaving] = useTransition();

  useEffect(() => {
    let ignore = false;
    api.getSettings().then((s) => {
      if (!ignore) setSettings(s);
    });
    return () => {
      ignore = true;
    };
  }, []);

  if (!settings) return <Loader2 className="mx-auto my-10 size-5 animate-spin text-muted-foreground" />;

  const handleTrayChange = (allowSystemTray: boolean) => {
    const next: Settings = { ...settings, allowSystemTray };
    setSettings(next);
    startSaving(async () => {
      try {
        await api.setSettings(next);
        toast.success(
          allowSystemTray
            ? "System tray enabled: Emissa will keep running in the tray when closed."
            : "System tray disabled: Emissa will exit when closed.",
        );
      } catch (e) {
        setSettings(settings);
        toast.error(errorMessage(e));
      }
    });
  };

  return (
    <div className="space-y-5">
      <section className="space-y-3">
        <h3 className="text-sm font-medium">Application behavior</h3>
        <div className="divide-y divide-border rounded-lg border border-border">
          <label className="flex cursor-pointer items-center justify-between gap-4 px-3.5 py-3">
            <div className="space-y-0.5">
              <div className="text-sm font-medium">Allow system tray</div>
              <div className="text-xs text-muted-foreground">
                Keep Emissa running in the system tray when closing the window so background uploads continue.
              </div>
            </div>
            <Switch
              checked={settings.allowSystemTray}
              onCheckedChange={handleTrayChange}
              disabled={isSaving}
            />
          </label>
        </div>
      </section>
    </div>
  );
}

// ---------- Defaults ----------

const FACTORY: VideoDefaults = {
  description: "",
  tags: [],
  categoryId: "22",
  privacy: "private",
  madeForKids: false,
  containsSyntheticMedia: false,
  embeddable: true,
  notifySubscribers: true,
};

function DefaultsTab() {
  const [fullSettings, setFullSettings] = useState<Settings | null>(null);
  const [draft, setDraft] = useState<VideoDefaults | null>(null);
  // What's on disk, kept to derive `dirty` instead of tracking it in another state.
  const [saved, setSaved] = useState<VideoDefaults | null>(null);
  const [isSaving, startSaving] = useTransition();

  useEffect(() => {
    let ignore = false;
    api.getSettings().then((s) => {
      if (ignore) return;
      setFullSettings(s);
      setDraft(s.defaults);
      setSaved(s.defaults);
    });
    return () => {
      ignore = true;
    };
  }, []);

  if (!draft) return <Loader2 className="mx-auto my-10 size-5 animate-spin text-muted-foreground" />;

  // Derived: a structural compare is cheap for an object this size and can't go stale.
  const dirty = JSON.stringify(draft) !== JSON.stringify(saved);
  const set = (patch: Partial<VideoDefaults>) => setDraft({ ...draft, ...patch });

  const save = () =>
    startSaving(async () => {
      try {
        const base = fullSettings ?? (await api.getSettings());
        const next: Settings = { ...base, defaults: draft };
        await api.setSettings(next);
        setFullSettings(next);
        setSaved(draft);
        toast.success("Defaults saved", { description: "Applied to videos you add from now on." });
      } catch (e) {
        toast.error(errorMessage(e));
      }
    });

  return (
    <div className="space-y-5">
      <p className="text-xs text-muted-foreground">
        Pre-filled on every video you add from now on. Videos already in the queue keep their own settings.
      </p>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label className="text-xs">Visibility</Label>
          <Select value={draft.privacy} onValueChange={(v) => set({ privacy: v as Privacy })}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="private">Private</SelectItem>
              <SelectItem value="unlisted">Unlisted</SelectItem>
              <SelectItem value="public">Public</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-2">
          <Label className="text-xs">Category</Label>
          <Select value={draft.categoryId} onValueChange={(categoryId) => set({ categoryId })}>
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
      </div>

      <div className="space-y-2">
        <div className="flex items-baseline justify-between text-xs">
          <Label>Default description</Label>
          <span className="text-muted-foreground tabular-nums">
            {byteLen(draft.description)}/{LIMITS.descriptionBytes}
          </span>
        </div>
        <Textarea
          value={draft.description}
          onChange={(e) => set({ description: e.target.value })}
          placeholder="Channel boilerplate, links, socials…"
          className="min-h-24"
        />
      </div>

      <div className="space-y-2">
        <Label className="text-xs">Default tags</Label>
        <TagInput tags={draft.tags} onChange={(tags) => set({ tags })} />
      </div>

      <div className="divide-y divide-border rounded-lg border border-border">
        <SwitchRow label="Made for kids" checked={draft.madeForKids} onChange={(madeForKids) => set({ madeForKids })} />
        <SwitchRow
          label="Contains altered or synthetic content"
          checked={draft.containsSyntheticMedia}
          onChange={(containsSyntheticMedia) => set({ containsSyntheticMedia })}
        />
        <SwitchRow label="Allow embedding" checked={draft.embeddable} onChange={(embeddable) => set({ embeddable })} />
        <SwitchRow
          label="Notify subscribers"
          checked={draft.notifySubscribers}
          onChange={(notifySubscribers) => set({ notifySubscribers })}
        />
      </div>

      <div className="flex items-center justify-between pt-1">
        <Button variant="ghost" size="sm" onClick={() => setDraft(FACTORY)}>
          <RotateCcw /> Reset to factory
        </Button>
        <Button onClick={save} disabled={!dirty || isSaving}>
          {isSaving && <Loader2 className="animate-spin" />} Save defaults
        </Button>
      </div>
    </div>
  );
}

function SwitchRow({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="flex cursor-pointer items-center justify-between gap-4 px-3 py-2.5 text-sm">
      {label}
      <Switch checked={checked} onCheckedChange={onChange} />
    </label>
  );
}

// ---------- Account ----------

function AccountTab({ channel, onSignOut, onStatusChange }: Omit<Props, "open" | "onOpenChange">) {
  const [isPending, startTransition] = useTransition();

  const replaceClient = () =>
    startTransition(async () => {
      const path = await openFile({ multiple: false, filters: [{ name: "Google client JSON", extensions: ["json"] }] });
      if (!path) return;
      try {
        // Rust clears the old refresh token (it belongs to the old client), so this returns
        // signedIn: false and App routes back to the sign-in screen.
        onStatusChange(await api.importClientJson(path));
      } catch (e) {
        toast.error(errorMessage(e));
      }
    });

  return (
    <div className="space-y-5">
      <section className="flex items-center gap-3 rounded-xl border border-border p-4">
        <Avatar className="size-11">
          {channel?.avatarUrl && <AvatarImage src={channel.avatarUrl} alt="" referrerPolicy="no-referrer" />}
          <AvatarFallback>{channel?.title.charAt(0).toUpperCase() ?? "?"}</AvatarFallback>
        </Avatar>
        <div className="min-w-0 flex-1">
          <div className="truncate font-medium">{channel?.title ?? "Loading…"}</div>
          <div className="text-xs text-muted-foreground">Signed in with Google</div>
        </div>
        <Button variant="outline" onClick={onSignOut}>
          <LogOut /> Sign out
        </Button>
      </section>

      <section className="space-y-2">
        <h3 className="text-sm font-medium">Google Cloud client</h3>
        <p className="text-xs text-muted-foreground">
          Switch to a different OAuth client (e.g. a new Cloud project with fresh quota). You'll need to sign in
          again. Queued uploads continue afterwards.
        </p>
        <Button variant="secondary" onClick={replaceClient} disabled={isPending}>
          {isPending ? <Loader2 className="animate-spin" /> : <FileJson />} Replace client JSON…
        </Button>
      </section>
    </div>
  );
}

// ---------- About ----------

function AboutTab() {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  return (
    <div className="space-y-5">
      <div className="flex items-center gap-3">
        <Logo className="text-lg" />
        <span className="rounded-md bg-muted px-2 py-0.5 text-xs text-muted-foreground tabular-nums">
          v{version ?? "…"} · beta
        </span>
      </div>
      <p className="text-sm text-muted-foreground">
        Lightweight YouTube uploader. Tauri 2, Rust, React 19. Uploads stream from disk in resumable 8 MiB chunks
        and survive restarts.
      </p>
      <section className="space-y-2">
        <h3 className="text-sm font-medium">Data folder</h3>
        <p className="text-xs text-muted-foreground">
          Holds your Google client file, the upload queue and these settings. Your sign-in token is not here; it's in
          Windows Credential Manager.
        </p>
        <Button variant="secondary" onClick={() => api.openDataDir().catch((e) => toast.error(errorMessage(e)))}>
          <FolderOpen /> Open data folder
        </Button>
      </section>
    </div>
  );
}
