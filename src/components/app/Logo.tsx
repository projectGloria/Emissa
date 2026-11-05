import { cn } from "@/lib/utils";

/** Paper-plane mark: Emissa (from "emissary") sends your videos off. Same path as app-icon.svg. */
export function LogoMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={cn("fill-current", className)} aria-hidden>
      <path d="M21 3 3 10.5l7 3.5 3.5 7Z" className="stroke-current stroke-[1.6]" strokeLinejoin="round" />
      {/* The fold: a cut-out line from the tail toward the nose, which gives the plane depth. */}
      <path d="M10 14 16 8" className="fill-none stroke-brand stroke-[1.6]" strokeLinecap="round" />
    </svg>
  );
}

export function Logo({ className }: { className?: string }) {
  return (
    <div className={cn("flex items-center gap-2 font-semibold tracking-tight", className)}>
      <span className="grid size-6 place-items-center rounded-md bg-brand text-brand-foreground shadow-[0_0_20px_-4px_var(--brand)]">
        <LogoMark className="size-3.5" />
      </span>
      Emissa
    </div>
  );
}
