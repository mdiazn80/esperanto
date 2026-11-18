import { cn } from "@/lib/utils";

/**
 * Esperanto icon, inline so it follows the app theme (the `.dark` class),
 * not only the OS appearance. Mirrors
 * src-tauri/icons/source/icono-{light-L1,dark-D1}.svg.
 */
export function AppLogo({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 100 100"
      aria-hidden="true"
      className={cn("size-8 shrink-0", className)}
    >
      <rect
        x="0.75"
        y="0.75"
        width="98.5"
        height="98.5"
        rx="22"
        strokeWidth="1.5"
        className="fill-white stroke-[#E5E5E5] dark:fill-[#111111] dark:stroke-[#333333]"
      />
      <g className="fill-[#FFD60A]">
        <rect x="16" y="43" width="4" height="14" rx="2" />
        <rect x="23" y="35" width="4" height="30" rx="2" />
        <rect x="30" y="39" width="4" height="22" rx="2" />
        <rect x="37" y="32" width="4" height="36" rx="2" />
      </g>
      <g className="fill-[#111111] dark:fill-white">
        <rect x="50" y="35" width="34" height="5" rx="2.5" />
        <rect x="50" y="47.5" width="26" height="5" rx="2.5" />
        <rect x="50" y="60" width="30" height="5" rx="2.5" />
      </g>
    </svg>
  );
}
