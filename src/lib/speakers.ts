/** Colors of the speakers, by speaker number (the backend keeps at most 8
 * speakers per session): `text` for their name, `border` for the bar that
 * marks the phrase being spoken and `swatch` for their color chip. */
const SPEAKER_COLORS = [
  {
    text: "text-yellow-700 dark:text-yellow-300",
    border: "border-yellow-500 dark:border-yellow-300",
    swatch: "bg-yellow-400 dark:bg-yellow-300",
  },
  {
    text: "text-sky-700 dark:text-sky-300",
    border: "border-sky-500 dark:border-sky-300",
    swatch: "bg-sky-500 dark:bg-sky-300",
  },
  {
    text: "text-pink-700 dark:text-pink-300",
    border: "border-pink-500 dark:border-pink-300",
    swatch: "bg-pink-500 dark:bg-pink-300",
  },
  {
    text: "text-emerald-700 dark:text-emerald-300",
    border: "border-emerald-500 dark:border-emerald-300",
    swatch: "bg-emerald-500 dark:bg-emerald-300",
  },
  {
    text: "text-violet-700 dark:text-violet-300",
    border: "border-violet-500 dark:border-violet-300",
    swatch: "bg-violet-500 dark:bg-violet-300",
  },
  {
    text: "text-orange-700 dark:text-orange-300",
    border: "border-orange-500 dark:border-orange-300",
    swatch: "bg-orange-500 dark:bg-orange-300",
  },
  {
    text: "text-teal-700 dark:text-teal-300",
    border: "border-teal-500 dark:border-teal-300",
    swatch: "bg-teal-500 dark:bg-teal-300",
  },
  {
    text: "text-lime-700 dark:text-lime-300",
    border: "border-lime-600 dark:border-lime-300",
    swatch: "bg-lime-500 dark:bg-lime-300",
  },
] as const;

export function speakerColor(speaker: number) {
  return SPEAKER_COLORS[speaker % SPEAKER_COLORS.length];
}
