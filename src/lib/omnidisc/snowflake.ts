export const OMNIDISC_EPOCH_MS = 1_785_542_400_000;
const TIMESTAMP_SHIFT = 22n;

export function snowflakeTime(id: string): number {
  if (!/^\d+$/.test(id)) return Date.now();
  try {
    return Number(BigInt(id) >> TIMESTAMP_SHIFT) + OMNIDISC_EPOCH_MS;
  } catch {
    return Date.now();
  }
}

export function compareSnowflakes(a: string, b: string): number {
  if (a === b) return 0;
  if (a.length !== b.length) return a.length < b.length ? -1 : 1;
  return a < b ? -1 : 1;
}

export function isAfter(a: string | undefined, b: string | undefined): boolean {
  if (!a) return false;
  if (!b) return true;
  return compareSnowflakes(a, b) > 0;
}

export function snowflakeForTime(ms: number): string {
  const delta = Math.max(0, Math.floor(ms) - OMNIDISC_EPOCH_MS);
  return (BigInt(delta) << TIMESTAMP_SHIFT).toString();
}
