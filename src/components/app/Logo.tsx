import { cn } from "@/lib/utils";
// Vite resolves this to a content-hashed URL at build time, so swapping the PNG busts the
// WebView cache automatically. Same artwork as src-tauri/icons (256px copy of 128x128@2x).
import logoUrl from "@/assets/emissa-logo.png";

export function Logo({ className }: { className?: string }) {
  return (
    <div className={cn("flex items-center gap-2 font-semibold tracking-tight", className)}>
      {/* The PNG already carries its own rounded pink tile, so no bg/rounded wrapper here,
          or you'd get a square-in-a-square. alt="" + aria-hidden: the "Emissa" text next to it
          is the accessible name; announcing the image too would just say it twice.
          draggable={false} stops WebView2's native image drag from hijacking the title bar's drag region. */}
      <img
        src={logoUrl}
        alt=""
        aria-hidden
        draggable={false}
        // filter: drop-shadow traces the PNG's alpha (rounded tile); box-shadow would glow
        // around the <img>'s square box instead.
        className="size-6 drop-shadow-[0_0_6px_color-mix(in_oklab,var(--brand)_60%,transparent)]"
      />
      Emissa
    </div>
  );
}
