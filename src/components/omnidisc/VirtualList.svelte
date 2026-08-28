<script lang="ts" generics="T">
  import { onMount, tick, type Snippet } from "svelte";

  let {
    items,
    getKey,
    estimateHeight = 56,
    overscan = 6,
    topThreshold = 120,
    bottomThreshold = 24,
    onReachTop,
    onBottomChange,
    row,
    empty,
  }: {
    items: T[];
    getKey: (item: T) => string;
    estimateHeight?: number;
    overscan?: number;
    topThreshold?: number;
    bottomThreshold?: number;
    onReachTop?: () => void;
    onBottomChange?: (atBottom: boolean) => void;
    row: Snippet<[T, number]>;
    empty?: Snippet;
  } = $props();

  type Anchor = { bottom: true } | { bottom: false; key: string; delta: number };

  let viewport = $state<HTMLDivElement | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(0);
  let heightsVersion = $state(0);
  let stickToBottom = true;
  let pendingAnchor: Anchor | null = null;
  let reachTopArmed = true;
  let lastItemCount = 0;
  let committedKeys: string[] = [];
  let committedOffsets: number[] = [0];

  const heights = new Map<string, number>();
  const elementKeys = new WeakMap<Element, string>();
  let rowObserver: ResizeObserver | null = null;
  let viewportObserver: ResizeObserver | null = null;

  let keys = $derived(items.map(getKey));

  let offsets = $derived.by(() => {
    heightsVersion;
    const out = new Array<number>(keys.length + 1);
    let acc = 0;
    for (let i = 0; i < keys.length; i++) {
      out[i] = acc;
      acc += heights.get(keys[i]) ?? estimateHeight;
    }
    out[keys.length] = acc;
    return out;
  });

  let totalHeight = $derived(offsets[offsets.length - 1] ?? 0);

  function lowerBoundIn(offs: number[], count: number, value: number): number {
    let lo = 0;
    let hi = count;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (offs[mid + 1] <= value) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  }

  function lowerBound(value: number): number {
    return lowerBoundIn(offsets, keys.length, value);
  }

  let range = $derived.by(() => {
    if (keys.length === 0) return { start: 0, end: 0 };
    const start = Math.max(0, lowerBound(scrollTop) - overscan);
    const end = Math.min(keys.length, lowerBound(scrollTop + viewportHeight) + 1 + overscan);
    return { start, end };
  });

  let visible = $derived(items.slice(range.start, range.end));
  let padTop = $derived(offsets[range.start] ?? 0);
  let padBottom = $derived(Math.max(0, totalHeight - (offsets[range.end] ?? totalHeight)));

  function isAtBottom(el: HTMLDivElement): boolean {
    return el.scrollHeight - el.scrollTop - el.clientHeight <= bottomThreshold;
  }

  function captureAnchor(): Anchor {
    const el = viewport;
    if (!el || committedKeys.length === 0 || stickToBottom) return { bottom: true };
    const index = Math.min(committedKeys.length - 1, lowerBoundIn(committedOffsets, committedKeys.length, el.scrollTop));
    return { bottom: false, key: committedKeys[index], delta: committedOffsets[index] - el.scrollTop };
  }

  function commit() {
    committedKeys = keys;
    committedOffsets = offsets;
  }

  function restoreAnchor(anchor: Anchor) {
    commit();
    const el = viewport;
    if (!el) return;
    if (anchor.bottom) {
      el.scrollTop = el.scrollHeight;
      return;
    }
    const index = keys.indexOf(anchor.key);
    if (index < 0) return;
    el.scrollTop = offsets[index] - anchor.delta;
  }

  function handleResizeEntries(entries: ResizeObserverEntry[]) {
    let changed = false;
    for (const entry of entries) {
      const key = elementKeys.get(entry.target);
      if (key === undefined) continue;
      const size = entry.borderBoxSize?.[0]?.blockSize ?? entry.contentRect.height;
      const height = Math.max(1, Math.round(size));
      if (heights.get(key) !== height) {
        heights.set(key, height);
        changed = true;
      }
    }
    if (!changed) return;
    const anchor = captureAnchor();
    heightsVersion += 1;
    tick().then(() => restoreAnchor(anchor));
  }

  function measure(node: HTMLElement, key: string) {
    elementKeys.set(node, key);
    rowObserver?.observe(node);
    return {
      update(next: string) {
        elementKeys.set(node, next);
      },
      destroy() {
        rowObserver?.unobserve(node);
      },
    };
  }

  function onScroll() {
    const el = viewport;
    if (!el) return;
    scrollTop = el.scrollTop;
    const atBottom = isAtBottom(el);
    if (atBottom !== stickToBottom) onBottomChange?.(atBottom);
    stickToBottom = atBottom;
    if (el.scrollTop <= topThreshold) {
      if (reachTopArmed && onReachTop && items.length > 0) {
        reachTopArmed = false;
        onReachTop();
      }
    } else {
      reachTopArmed = true;
    }
  }

  export function scrollToBottom() {
    const el = viewport;
    if (!el) return;
    stickToBottom = true;
    el.scrollTop = el.scrollHeight;
    onBottomChange?.(true);
  }

  export function scrollToKey(key: string) {
    const el = viewport;
    if (!el) return;
    const index = keys.indexOf(key);
    if (index < 0) return;
    stickToBottom = false;
    el.scrollTop = Math.max(0, (offsets[index] ?? 0) - el.clientHeight / 3);
    scrollTop = el.scrollTop;
    onBottomChange?.(isAtBottom(el));
  }

  onMount(() => {
    rowObserver = new ResizeObserver(handleResizeEntries);
    viewportObserver = new ResizeObserver(() => {
      const el = viewport;
      if (!el) return;
      const anchor = captureAnchor();
      viewportHeight = el.clientHeight;
      tick().then(() => restoreAnchor(anchor));
    });
    if (viewport) {
      viewportObserver.observe(viewport);
      viewportHeight = viewport.clientHeight;
    }
    tick().then(() => scrollToBottom());
    return () => {
      rowObserver?.disconnect();
      viewportObserver?.disconnect();
    };
  });

  $effect.pre(() => {
    items;
    pendingAnchor = captureAnchor();
  });

  $effect(() => {
    items;
    const anchor = pendingAnchor;
    pendingAnchor = null;
    if (!anchor) return;
    tick().then(() => {
      restoreAnchor(anchor);
      if (items.length !== lastItemCount) {
        lastItemCount = items.length;
        reachTopArmed = true;
      }
    });
  });
</script>

<div class="virtual-list" bind:this={viewport} onscroll={onScroll} tabindex="-1">
  {#if items.length === 0}
    {#if empty}
      {@render empty()}
    {/if}
  {:else}
    <div class="virtual-list-inner" style:padding-top="{padTop}px" style:padding-bottom="{padBottom}px">
      {#each visible as item, i (getKey(item))}
        <div class="virtual-list-row" use:measure={getKey(item)}>
          {@render row(item, range.start + i)}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .virtual-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    overflow-anchor: none;
    contain: layout paint;
    outline: none;
  }

  .virtual-list-inner {
    box-sizing: border-box;
  }

  .virtual-list-row {
    contain: layout;
  }
</style>
