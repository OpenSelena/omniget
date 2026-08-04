<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { ThumbnailSlice } from "$lib/study-bridge";

  type Props = {
    thumbnails: ThumbnailSlice[];
    duration: number;
    target: HTMLElement | null;
  };

  let { thumbnails, duration, target }: Props = $props();
  let visible = $state(false);
  let leftPx = $state(0);
  let timeMs = $state(0);

  const slice = $derived.by(() => {
    if (!visible || thumbnails.length === 0) return null;
    return thumbnails.find((t) => timeMs >= t.start_ms && timeMs < t.end_ms) ?? null;
  });

  const spriteUrl = $derived.by(() => {
    if (!slice) return null;
    if (slice.sprite_url.startsWith("http://") || slice.sprite_url.startsWith("https://")) {
      return slice.sprite_url;
    }
    try {
      return convertFileSrc(slice.sprite_url);
    } catch {
      return slice.sprite_url;
    }
  });

  const previewWidth = $derived(slice?.w ?? 160);
  const previewHeight = $derived(slice?.h ?? 90);

  function onMove(e: MouseEvent) {
    if (!target || !duration) return;
    const rect = target.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    timeMs = ratio * duration * 1000;
    leftPx = e.clientX - rect.left;
    visible = true;
  }

  function onLeave() {
    visible = false;
  }

  $effect(() => {
    if (!target) return;
    target.addEventListener("mousemove", onMove);
    target.addEventListener("mouseleave", onLeave);
    return () => {
      target.removeEventListener("mousemove", onMove);
      target.removeEventListener("mouseleave", onLeave);
    };
  });

  function fmtTime(ms: number): string {
    const total = Math.max(0, Math.floor(ms / 1000));
    const m = Math.floor(total / 60);
    const s = total % 60;
    return `${m}:${String(s).padStart(2, "0")}`;
  }
</script>

{#if visible && slice && spriteUrl}
  <div
    class="preview"
    style:left={`${leftPx}px`}
    style:width={`${previewWidth}px`}
    style:height={`${previewHeight + 22}px`}
    aria-hidden="true"
  >
    <div
      class="sprite"
      style:width={`${previewWidth}px`}
      style:height={`${previewHeight}px`}
      style:background-image={`url('${spriteUrl}')`}
      style:background-position={`-${slice.x ?? 0}px -${slice.y ?? 0}px`}
    ></div>
    <span class="time">{fmtTime(timeMs)}</span>
  </div>
{/if}

<style>
  .preview {
    position: absolute;
    bottom: calc(100% + 8px);
    transform: translateX(-50%);
    background: black;
    border: 1px solid color-mix(in oklab, white 16%, transparent);
    border-radius: 4px;
    padding: 0;
    pointer-events: none;
    z-index: 10;
    overflow: hidden;
    box-shadow: 0 4px 12px color-mix(in oklab, black 40%, transparent);
  }

  .sprite {
    background-repeat: no-repeat;
    background-color: black;
  }

  .time {
    display: block;
    text-align: center;
    color: white;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    padding: 4px 0;
    background: color-mix(in oklab, black 80%, transparent);
  }
</style>
