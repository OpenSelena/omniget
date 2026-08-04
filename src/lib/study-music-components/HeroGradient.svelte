<script lang="ts">
  import type { Snippet } from "svelte";
  import { palettePathOrUrl, rgbToCssAlpha, type RGB } from "$lib/study-music/dominant-color";

  type Props = {
    coverUrl?: string | null;
    minHeight?: number;
    children: Snippet;
  };
  let { coverUrl = null, minHeight = 240, children }: Props = $props();

  let palette = $state<RGB[] | null>(null);

  $effect(() => {
    let cancelled = false;
    palette = null;
    if (!coverUrl) return;
    void palettePathOrUrl(coverUrl).then((p) => {
      if (!cancelled) palette = p;
    });
    return () => {
      cancelled = true;
    };
  });

  const c1 = $derived(rgbToCssAlpha(palette?.[0] ?? null, 0.55));
  const c2 = $derived(rgbToCssAlpha(palette?.[1] ?? palette?.[0] ?? null, 0.35));
</script>

<div
  class="hero-gradient"
  style:--c1={c1}
  style:--c2={c2}
  style:min-height={`${minHeight}px`}
>
  <div class="bg" aria-hidden="true"></div>
  <div class="content">
    {@render children()}
  </div>
</div>

<style>
  .hero-gradient {
    position: relative;
    border-radius: 24px;
    overflow: hidden;
    isolation: isolate;
  }
  .bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(120% 80% at 0% 0%, var(--c1) 0%, transparent 60%),
      radial-gradient(120% 80% at 100% 100%, var(--c2) 0%, transparent 60%),
      linear-gradient(135deg, rgba(20, 20, 24, 0.6), rgba(10, 10, 12, 0.85));
    backdrop-filter: blur(40px);
    z-index: 0;
    transition: opacity 400ms ease;
  }
  .content {
    position: relative;
    z-index: 1;
    height: 100%;
  }
  @media (prefers-reduced-motion: reduce) {
    .bg { transition: none; }
  }
</style>
