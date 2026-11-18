/**
 * Formats a byte count with decimal units (like Finder): "3.2 GB", "450 MB".
 */
export function formatBytes(bytes: number, locale: string): string {
  const units = [
    ["gigabyte", 1e9],
    ["megabyte", 1e6],
    ["kilobyte", 1e3],
  ] as const;
  for (const [unit, size] of units) {
    if (bytes >= size) {
      return new Intl.NumberFormat(locale, {
        style: "unit",
        unit,
        maximumFractionDigits: 1,
      }).format(bytes / size);
    }
  }
  return new Intl.NumberFormat(locale, { style: "unit", unit: "byte" }).format(
    bytes,
  );
}

export function formatPercent(fraction: number, locale: string): string {
  return new Intl.NumberFormat(locale, {
    style: "percent",
    maximumFractionDigits: 0,
  }).format(fraction);
}

/** Seconds with one decimal, in the locale's number format: "1.2", "1,2". */
export function formatSeconds(ms: number, locale: string): string {
  return new Intl.NumberFormat(locale, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(ms / 1000);
}

/** Host name of a URL, for display; the input itself if it isn't a URL. */
export function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/** Date and time of a Unix timestamp (ms): "18 Nov 2026, 19:20". */
export function formatDateTime(ms: number, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(ms);
}

/** A duration, rounded for display: "45 s", "12 min", "1 h 5 min". */
export function formatDuration(ms: number, locale: string): string {
  const unit = (value: number, name: "second" | "minute" | "hour") =>
    new Intl.NumberFormat(locale, {
      style: "unit",
      unit: name,
      unitDisplay: "short",
    }).format(value);
  const seconds = Math.round(ms / 1000);
  if (seconds < 60) return unit(seconds, "second");
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return unit(minutes, "minute");
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0
    ? unit(hours, "hour")
    : `${unit(hours, "hour")} ${unit(rest, "minute")}`;
}

/** Time of day of a Unix timestamp (ms), hours and minutes: "11:21". */
export function formatClock(ms: number, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(ms);
}

/** Elapsed time as a stopwatch: "00:12:34". */
export function formatElapsed(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(Math.floor(total / 3600))}:${pad(Math.floor(total / 60) % 60)}:${pad(total % 60)}`;
}
