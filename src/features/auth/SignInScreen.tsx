import { useState, useTransition } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { CenteredCard } from "@/components/app/CenteredCard";
import { api, errorMessage } from "@/lib/api";

function GoogleG() {
  return (
    <svg viewBox="0 0 48 48" className="size-4" aria-hidden>
      <path fill="#FFC107" d="M43.6 20.5H42V20H24v8h11.3C33.7 32.7 29.3 36 24 36c-6.6 0-12-5.4-12-12s5.4-12 12-12c3.1 0 5.8 1.2 7.9 3.1l5.7-5.7C34 6.1 29.3 4 24 4 12.9 4 4 12.9 4 24s8.9 20 20 20 20-8.9 20-20c0-1.3-.1-2.4-.4-3.5z" />
      <path fill="#FF3D00" d="m6.3 14.7 6.6 4.8C14.7 15.1 19 12 24 12c3.1 0 5.8 1.2 7.9 3.1l5.7-5.7C34 6.1 29.3 4 24 4 16.3 4 9.7 8.3 6.3 14.7z" />
      <path fill="#4CAF50" d="M24 44c5.2 0 9.9-2 13.4-5.2l-6.2-5.2C29.2 35.1 26.7 36 24 36c-5.3 0-9.7-3.3-11.3-8l-6.5 5C9.5 39.6 16.2 44 24 44z" />
      <path fill="#1976D2" d="M43.6 20.5H42V20H24v8h11.3c-.8 2.2-2.2 4.2-4.1 5.6l6.2 5.2C37 39.2 44 34 44 24c0-1.3-.1-2.4-.4-3.5z" />
    </svg>
  );
}

export function SignInScreen({ onDone }: { onDone: () => void }) {
  const [isPending, startTransition] = useTransition();
  const [error, setError] = useState<string | null>(null);

  const signIn = () =>
    startTransition(async () => {
      setError(null);
      try {
        await api.signIn(); // resolves only once the browser redirect hits our loopback server
        onDone();
      } catch (e) {
        setError(errorMessage(e));
      }
    });

  return (
    <CenteredCard>
      <Card>
        <CardHeader>
          <CardTitle>Sign in to YouTube</CardTitle>
          <CardDescription>
            Your browser opens to Google. Emissa asks to upload videos, read your channel name, and (optionally) add subtitles.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Button className="w-full" size="lg" variant="outline" onClick={signIn} disabled={isPending}>
            {isPending ? <Loader2 className="animate-spin" /> : <GoogleG />}
            {isPending ? "Waiting for your browser…" : "Continue with Google"}
          </Button>
          {error && <p className="rounded-md bg-destructive/10 p-2.5 text-sm text-destructive">{error}</p>}
          <p className="text-xs leading-relaxed text-muted-foreground">
            Google will warn that the app is unverified. That's expected for your own API project: choose{" "}
            <b>Advanced → Go to (your app)</b>.
          </p>
        </CardContent>
      </Card>
    </CenteredCard>
  );
}
