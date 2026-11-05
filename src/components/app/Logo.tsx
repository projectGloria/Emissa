import { cn } from "@/lib/utils";

export function Logo({ className }: { className?: string }) {
  return (
    <div className={cn("flex items-center gap-2 font-semibold tracking-tight", className)}>
      <span className="grid size-6 place-items-center rounded-md bg-brand text-brand-foreground shadow-[0_0_20px_-4px_var(--brand)]">
        <svg viewBox="0 0 24 24" className="size-3.5 fill-current" aria-hidden>
          <path d="M8 5.5v13l11-6.5z" />
        </svg>
      </span>
      Yutub
    </div>
  );
}
