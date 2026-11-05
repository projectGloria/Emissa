import type { ReactNode } from "react";
import { Logo } from "./Logo";

/** Full-window shell for the pre-app gates (client setup, sign-in). */
export function CenteredCard({ children }: { children: ReactNode }) {
  return (
    <div className="relative grid h-full place-items-center overflow-auto p-8">
      {/* Soft brand glow: cheap atmosphere, a single radial gradient, no image assets */}
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(60%_50%_at_50%_0%,color-mix(in_oklch,var(--brand)_18%,transparent),transparent)]" />
      <div className="relative w-full max-w-md">
        <Logo className="mb-6 justify-center text-lg" />
        {children}
      </div>
    </div>
  );
}
