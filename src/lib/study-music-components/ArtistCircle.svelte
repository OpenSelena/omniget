<script lang="ts">
  import { goto } from "$app/navigation";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { colorFromString } from "$lib/study-music/format";

  type Props = {
    name: string;
    coverPath?: string | null;
    trackCount?: number;
  };

  let { name, coverPath, trackCount }: Props = $props();

  const cover = $derived.by(() => {
    if (!coverPath) return null;
    try {
      return convertFileSrc(coverPath);
    } catch {
      return coverPath;
    }
  });

  const fallbackColor = $derived(colorFromString(name));
  const initial = $derived(name.slice(0, 1).toUpperCase());

  function open() {
    goto(`/study/music/artist/by-name?name=${encodeURIComponent(name)}`);
  }
</script>

<button type="button" class="artist-circle" onclick={open}>
  <span class="circle">
    {#if cover}
      <img src={cover} alt="" loading="lazy" />
    {:else}
      <span class="circle-fallback" style:background={fallbackColor}>{initial}</span>
    {/if}
  </span>
  <span class="name">{name}</span>
  {#if trackCount != null}
    <span class="sub">{trackCount} faixa(s)</span>
  {/if}
</button>

<style>
  .artist-circle {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: 0;
    color: inherit;
    font-family: inherit;
    cursor: pointer;
    padding: 0;
    width: 100%;
    text-align: center;
  }
  .circle {
    width: 100%;
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    overflow: hidden;
    box-shadow: 0 4px 14px color-mix(in oklab, black 24%, transparent);
    transition: transform 120ms ease, box-shadow 120ms ease;
    background: var(--button);
  }
  .artist-circle:hover .circle {
    transform: scale(1.04);
    box-shadow: 0 6px 18px color-mix(in oklab, var(--accent) 40%, transparent);
  }
  .circle img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .circle-fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: white;
    font-size: clamp(28px, 4vw, 48px);
    font-weight: 800;
  }
  .name {
    font-size: 13px;
    font-weight: 600;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }
  .sub {
    font-size: 11px;
    color: var(--tertiary);
  }
</style>
