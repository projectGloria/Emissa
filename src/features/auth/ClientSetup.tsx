import { useState, useTransition } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FileJson, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { CenteredCard } from "@/components/app/CenteredCard";
import { api, errorMessage, type SetupStatus } from "@/lib/api";

// Labels match Google's current "Google Auth Platform" console (Branding / Audience / Clients),
// not the legacy "OAuth consent screen / Credentials" pages.
const STEPS = [
  <>In <b>console.cloud.google.com</b>, search <b>YouTube Data API v3</b> and click <b>Enable</b>.</>,
  <>Google Auth Platform → <b>Get started</b>. Set the audience to <b>External</b>.</>,
  <><b>Audience</b> → Test users → add the Google account that owns your channel.</>,
  <><b>Clients</b> → Create client → type <b>Desktop app</b> → Download JSON.</>,
  <>Pick that JSON below.</>,
];

export function ClientSetup({ onDone }: { onDone: (s: SetupStatus) => void }) {
  // React 19: startTransition accepts an async function, and isPending stays true until it
  // settles. That replaces the old hand-rolled `const [loading, setLoading]` pair.
  const [isPending, startTransition] = useTransition();
  const [error, setError] = useState<string | null>(null);

  const pick = () =>
    startTransition(async () => {
      setError(null);
      const path = await open({ multiple: false, filters: [{ name: "Google client JSON", extensions: ["json"] }] });
      if (!path) return;
      try {
        onDone(await api.importClientJson(path));
      } catch (e) {
        setError(errorMessage(e));
      }
    });

  return (
    <CenteredCard>
      <Card>
        <CardHeader>
          <CardTitle>Connect a Google Cloud client</CardTitle>
          <CardDescription>
            One-time setup. Yutub talks to YouTube through your own API project, so your quota and data stay yours.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ol className="space-y-2.5 text-sm text-muted-foreground">
            {STEPS.map((s, i) => (
              <li key={i} className="flex gap-3">
                <span className="grid size-5 shrink-0 place-items-center rounded-full bg-muted text-[11px] font-medium text-foreground">
                  {i + 1}
                </span>
                <span>{s}</span>
              </li>
            ))}
          </ol>
          {error && <p className="mt-4 rounded-md bg-destructive/10 p-2.5 text-sm text-destructive">{error}</p>}
        </CardContent>
        <CardFooter>
          <Button className="w-full" size="lg" onClick={pick} disabled={isPending}>
            {isPending ? <Loader2 className="animate-spin" /> : <FileJson />}
            Choose client JSON…
          </Button>
        </CardFooter>
      </Card>
      <p className="mt-4 text-center text-xs text-muted-foreground">
        The file is copied to your app data folder and never leaves this machine.
      </p>
    </CenteredCard>
  );
}
