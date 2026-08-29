export type AnchorRect = {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
};

export type PopoverSize = { width: number; height: number };

export type Viewport = { width: number; height: number };

export type PopoverPlacement = { left: number; top: number };

export type PlaceOptions = {
  align?: "start" | "end";
  gap?: number;
  margin?: number;
};

function clamp(value: number, min: number, max: number): number {
  if (max < min) return min;
  return Math.min(Math.max(value, min), max);
}

export function placeMenu(
  anchor: AnchorRect,
  size: PopoverSize,
  viewport: Viewport,
  options: PlaceOptions = {},
): PopoverPlacement {
  const gap = options.gap ?? 4;
  const margin = options.margin ?? 8;
  const wanted = options.align === "start" ? anchor.left : anchor.right - size.width;
  const left = clamp(wanted, margin, viewport.width - size.width - margin);
  const below = anchor.bottom + gap;
  const above = anchor.top - gap - size.height;
  const fitsBelow = below + size.height <= viewport.height - margin;
  const fitsAbove = above >= margin;
  const top = fitsBelow || !fitsAbove ? clamp(below, margin, viewport.height - size.height - margin) : above;
  return { left, top };
}

// A menu inside `contain: layout` is positioned against that box and clipped by
// the scroller around it, so even `position: fixed` cannot escape. Moving the
// node to the body is the only way out.
export function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return {
    destroy() {
      node.remove();
    },
  };
}
