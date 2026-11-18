/** Level reported for digital silence, in dBFS (matches the Rust listener). */
export const SILENCE_DB = -80;
/** Quietest level shown on the meter. Below this the bar is empty. */
const METER_FLOOR_DB = -60;

/** Maps dBFS to 0–1 on a perceptual (dB) scale. */
export function meterFraction(db: number): number {
  if (db <= METER_FLOOR_DB) return 0;
  return Math.min(1, (db - METER_FLOOR_DB) / -METER_FLOOR_DB);
}
