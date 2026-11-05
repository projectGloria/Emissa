import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { api, errorMessage, type SetupStatus } from "@/lib/api";
import { ClientSetup } from "@/features/auth/ClientSetup";
import { SignInScreen } from "@/features/auth/SignInScreen";
import { MainApp } from "@/features/MainApp";

export default function App() {
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [bootError, setBootError] = useState<string | null>(null);

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

  if (bootError) {
    return <div className="grid h-full place-items-center p-8 text-sm text-destructive">{bootError}</div>;
  }
  if (!status) {
    return (
      <div className="grid h-full place-items-center">
        <Loader2 className="size-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  // A state machine, derived purely from `status`. No router: three screens don't need URLs.
  if (!status.hasClient) return <ClientSetup onDone={setStatus} />;
  if (!status.signedIn) return <SignInScreen onDone={() => setStatus({ ...status, signedIn: true })} />;
  return <MainApp onSignedOut={() => setStatus({ ...status, signedIn: false })} />;
}
