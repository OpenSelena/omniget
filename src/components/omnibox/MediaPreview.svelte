<script lang="ts">
  import type { MediaPreview } from "$lib/stores/media-preview-store.svelte";

  let { mediaPreview = $bindable(), imageLoading = $bindable(true) } = $props();

  function formatDuration(seconds: number | null): string {
    if (seconds === null) return "";
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    if (h > 0) return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
    return `${m}:${s.toString().padStart(2, "0")}`;
  }
</script>

{#if mediaPreview}
  <div class="preview-container">
    <div class="preview-thumbnail-wrapper">
      {#if imageLoading && mediaPreview.thumbnail_url}
        <div class="preview-skeleton"></div>
      {/if}
      {#if mediaPreview.thumbnail_url}
        <img
          class="preview-thumbnail"
          class:loaded={!imageLoading}
          src={mediaPreview.thumbnail_url}
          alt={mediaPreview.title}
          loading="lazy"
          onload={() => { imageLoading = false; }}
          onerror={() => { imageLoading = false; }}
        />
      {:else}
        <div class="preview-fallback-art">
          <img class="preview-art" src="/emoji/package.png" alt="" width="32" height="32" draggable="false" />
        </div>
      {/if}
    </div>
    <div class="preview-info">
      <p class="preview-title">{mediaPreview.title}</p>
      {#if mediaPreview.author}
        <p class="preview-author">{mediaPreview.author}</p>
      {/if}
      {#if mediaPreview.duration_seconds}
        <p class="preview-duration">{formatDuration(mediaPreview.duration_seconds)}</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .preview-container {
    display: flex;
    gap: var(--space-3);
    align-items: center;
    padding: 0;
    width: 100%;
  }

  .preview-thumbnail-wrapper {
    position: relative;
    flex-shrink: 0;
    width: 128px;
    height: 72px;
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .preview-skeleton {
    width: 128px;
    height: 72px;
    border-radius: var(--radius-sm);
    background: var(--button-elevated);
    animation: skeleton-shimmer 2s infinite;
  }

  .preview-fallback-art {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 128px;
    height: 72px;
    border-radius: var(--radius-sm);
    background: var(--button-elevated);
  }

  @keyframes skeleton-shimmer {
    0%, 100% {
      opacity: 0.6;
    }
    50% {
      opacity: 1;
    }
  }

  .preview-thumbnail {
    width: 128px;
    height: 72px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    opacity: 0;
    transition: opacity 0.2s;
  }

  .preview-thumbnail.loaded {
    opacity: 1;
  }

  .preview-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
    min-width: 0;
  }

  .preview-title {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    line-clamp: 2;
    line-height: 1.35;
  }

  .preview-author {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview-duration {
    margin: 0;
    font-size: var(--text-xs);
    font-weight: 500;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }

  @media (max-width: 535px) {
    .preview-container {
      max-width: 100%;
      gap: calc(var(--padding));
    }

    .preview-thumbnail {
      width: 100px;
      height: 56.25px;
    }

    .preview-skeleton {
      width: 100px;
      height: 56.25px;
    }

    .preview-title {
      font-size: 12px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .preview-skeleton {
      animation: none;
    }
  }
</style>
