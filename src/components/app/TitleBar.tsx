import { useEffect, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Logo } from "./Logo";
import { cn } from "@/lib/utils";

// Resolved once at module load; it's a cheap handle, not a window lookup per render.
const appWindow = getCurrentWindow();

type Light = "close" | "minimize" | "maximize";

// Each color is spelled out twice as complete literals (`on` + `hover`) on purpose: Tailwind
// only generates classes it finds verbatim in source, so `group-hover:${color}` would ship no CSS.
const LIGHTS: { kind: Light; on: string; hover: string; label: string; glyph: ReactNode; action: () => Promise<void> }[] = [
  {
    kind: "close",
    on: "bg-[#ff5f57] border-[#e0443e]",
    hover: "group-hover:bg-[#ff5f57] group-hover:border-[#e0443e]",
    label: "Close",
    glyph: <path d="M3 3l4 4M7 3l-4 4" />,
    action: () => appWindow.close(),
  },
  {
    kind: "minimize",
    on: "bg-[#febc2e] border-[#dea123]",
    hover: "group-hover:bg-[#febc2e] group-hover:border-[#dea123]",
    label: "Minimize",
    glyph: <path d="M2.5 5h5" />,
    action: () => appWindow.minimize(),
  },
  {
    kind: "maximize",
    on: "bg-[#28c840] border-[#1aab29]",
    hover: "group-hover:bg-[#28c840] group-hover:border-[#1aab29]",
    label: "Maximize",
    // macOS "zoom" glyph: two inward-pointing corners
    glyph: <path d="M3 6.8V3h3.8zM7 3.2V7H3.2z" className="fill-current stroke-none" />,
    action: () => appWindow.toggleMaximize(),
  },
];

function TrafficLights() {
  // macOS greys the lights out when the window loses focus; mirroring that is half
  // of what makes them read as "real".
  const [focused, setFocused] = useState(true);

  useEffect(() => {
    const p = appWindow.onFocusChanged(({ payload }) => setFocused(payload));
    return () => void p.then((off) => off());
  }, []);

  return (
    // `group`: hovering ANY light reveals the glyphs on ALL three, like macOS.
    <div className="group flex items-center gap-2 px-1">
      {LIGHTS.map(({ kind, on, hover, label, glyph, action }) => (
        <button
          key={kind}
          type="button"
          aria-label={label}
          onClick={() => void action()}
          // Stop the double-click on a light from bubbling into the drag region's toggle-maximize.
          onDoubleClick={(e) => e.stopPropagation()}
          className={cn(
            "grid size-3 place-items-center rounded-full border-[0.5px] outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/60",
            // Focused: full color. Blurred: grey, regaining color on hover (macOS behavior).
            focused ? on : ["border-white/10 bg-white/15", hover],
          )}
        >
          <svg
            viewBox="0 0 10 10"
            className="size-2 stroke-black/60 stroke-[1.4] text-black/60 opacity-0 transition-opacity group-hover:opacity-100"
            strokeLinecap="round"
            aria-hidden
          >
            {glyph}
          </svg>
        </button>
      ))}
    </div>
  );
}

/**
 * Replaces the native title bar (decorations: false). `data-tauri-drag-region` makes Tauri
 * start a native window drag on mousedown, and toggle maximize on double-click. It only
 * applies to the element that carries the attribute, not its children, so every
 * non-interactive child carries it too.
 */
export function TitleBar({ children }: { children?: ReactNode }) {
  return (
    <header
      data-tauri-drag-region
      className="relative flex h-10 shrink-0 items-center justify-between border-b border-sidebar-border bg-sidebar pr-2 pl-3.5"
    >
      <TrafficLights />
      {/* Absolutely centered on the WINDOW, not the leftover space between the lights and the
          right-side buttons; otherwise it drifts whenever those groups differ in width.
          pointer-events-none lets mousedown fall through to the header's drag region. */}
      <Logo className="pointer-events-none absolute left-1/2 -translate-x-1/2 text-sm" />
      <div className="flex items-center">{children}</div>
    </header>
  );
}
