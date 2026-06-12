export function wheelZoomDirection(e: WheelEvent): -1 | 0 | 1 {
  if (!e.ctrlKey && !e.metaKey) return 0;
  if (e.deltaY === 0) return 0;
  return e.deltaY < 0 ? 1 : -1;
}

export function clampZoom(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
