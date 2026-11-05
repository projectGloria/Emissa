import { useEffect, useState } from "react";
import { Loader2, Settings2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { TitleBar } from "@/components/app/TitleBar";
import { api, errorMessage, type SetupStatus } from "@/lib/api";
import { ClientSetup } from "@/features/auth/ClientSetup";
import { SignInScreen } from "@/features/auth/SignInScreen";
import { MainApp } from "@/features/MainApp";

export default function App() {
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [bootError, setBootError] = useState<string | null>(null);
  // Lives here because the button is in the title bar (App-level) while the dialog needs
  // MainApp's channel/sign-out context, so the state is lifted to their common parent.
  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    // `ignore` guards against StrictMode's mount→unmount→mount: the first,
    // discarded effect's promise must not set state.
    let ignore = false;
    api
      .getSetupStatus()
      .then((s) => !ignore && setStatus(s))
      .catch((e) => !ignore && setBootError(errorMessage(e)));
    return () => {
      ignore = true;
    };
  }, []);

  const ready = status?.hasClient && status.signedIn;

  let screen;
  if (bootError) {
    screen = <div className="grid h-full place-items-center p-8 text-sm text-destructive">{bootError}</div>;
  } else if (!status) {
    screen = (
      <div className="grid h-full place-items-center">
        <Loader2 className="size-5 animate-spin text-muted-foreground" />
      </div>
    );
  } else if (!status.hasClient) {
    // A state machine derived purely from `status`. No router: three screens don't need URLs.
    screen = <ClientSetup onDone={setStatus} />;
  } else if (!status.signedIn) {
    screen = <SignInScreen onDone={() => setStatus({ ...status, signedIn: true })} />;
  } else {
    screen = (
      <MainApp
        settingsOpen={settingsOpen}
        onSettingsOpenChange={setSettingsOpen}
        // Replacing the client JSON clears the token too, so both flags can change.
        onStatusChange={(s) => {
          setSettingsOpen(false);
          setStatus(s);
        }}
      />
    );
  }

  return (
    // The title bar sits outside the screen switch: with decorations off, it's the only way
    // to move or close the window, so every state (even boot errors) must render it.
    <div className="flex h-full flex-col">
      <TitleBar>
        {ready && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon-sm" aria-label="Options" onClick={() => setSettingsOpen(true)}>
                <Settings2 />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Options</TooltipContent>
          </Tooltip>
        )}
      </TitleBar>
      <div className="min-h-0 flex-1">{screen}</div>
    </div>
  );
}
