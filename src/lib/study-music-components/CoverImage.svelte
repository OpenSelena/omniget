<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { pluginInvoke } from "$lib/plugin-invoke";

  type Props = {
    src: string | null | undefined;
    alt?: string;
    size?: number | "fluid";
    fallbackSeed?: string | null;
    rounded?: "sm" | "md" | "lg" | "xl";
    trackId?: number | null;
  };

  let {
    src,
    alt = "",
    size = "fluid",
    fallbackSeed = null,
    rounded = "md",
    trackId = null,
  }: Props = $props();

  let assetSrc = $state<string | null>(null);
  let dataUrl = $state<string | null>(null);
  let assetFailed = $state(false);

  $effect(() => {
    assetFailed = false;
    dataUrl = null;
    if (!src) {
      assetSrc = null;
      return;
    }
    // URLs HTTP/HTTPS/data passam direto. convertFileSrc é só pra paths locais.
    if (
      src.startsWith("http://") ||
      src.startsWith("https://") ||
      src.startsWith("data:") ||
      src.startsWith("blob:")
    ) {
      assetSrc = src;
      return;
    }
    try {
      assetSrc = convertFileSrc(src);
    } catch {
      assetSrc = src;
    }
  });

  async function onAssetError() {
    if (assetFailed) return;
    assetFailed = true;
    if (!trackId) return;
    try {
      const res = await pluginInvoke<{ data_url: string | null }>(
        "study",
        "study:music:cover:data-url",
        { id: trackId },
      );
      if (res.data_url) dataUrl = res.data_url;
    } catch {
      /* ignore */
    }
  }

  const sizeStyle = $derived(
    size === "fluid"
      ? "width: 100%; height: 100%;"
      : `width: ${size}px; height: ${size}px;`,
  );

  const radiusClass = $derived(`r-${rounded}`);
  const finalSrc = $derived(dataUrl ?? assetSrc);
</script>

<div class="cover {radiusClass}" style={sizeStyle}>
  {#if finalSrc}
    <img src={finalSrc} {alt} loading="lazy" onerror={onAssetError} />
  {:else}
    <div class="fallback">
      <svg viewBox="0 0 24 24" width="40%" height="40%" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M9 18V5l12-2v13"/>
        <circle cx="6" cy="18" r="3"/>
        <circle cx="18" cy="16" r="3"/>
      </svg>
    </div>
  {/if}
</div>

<style>
  .cover {
    position: relative;
    overflow: hidden;
    flex-shrink: 0;
    background: rgba(40, 40, 40, 0.6);
  }
  .r-sm { border-radius: 4px; }
  .r-md { border-radius: 6px; }
  .r-lg { border-radius: 10px; }
  .r-xl { border-radius: 14px; }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.25);
    background: rgba(40, 40, 40, 0.6);
  }
</style>
