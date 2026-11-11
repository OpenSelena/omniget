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
      {#if mediaPreview.duration_seconds}
        <span class="preview-duration-badge">
          {formatDuration(mediaPreview.duration_seconds)}
        </span>
      {/if}
    </div>
    <div class="preview-info">
      <p class="preview-title">{mediaPreview.title}</p>
      {#if mediaPreview.author}
        <p class="preview-author">{mediaPreview.author}</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .preview-container {
    display: flex;
    gap: calc(var(--padding) * 1.5);
    align-items: flex-start;
    padding: 0 calc(var(--padding) * 1.5);
    max-width: 400px;
  }

  .preview-thumbnail-wrapper {
    position: relative;
    flex-shrink: 0;
  }

  .preview-skeleton {
    width: 120px;
    height: 67.5px;
    border-radius: calc(var(--border-radius) - 4px);
    background: var(--button-elevated);
    animation: skeleton-shimmer 2s infinite;
  }

  .preview-fallback-art {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 120px;
    height: 67.5px;
    border-radius: calc(var(--border-radius) - 4px);
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
    width: 120px;
    height: 67.5px;
    border-radius: calc(var(--border-radius) - 4px);
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
    gap: 4px;
    flex: 1;
    min-width: 0;
    padding-top: 4px;
  }

  .preview-title {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    line-clamp: 2;
    line-height: 1.4;
  }

  .preview-author {
    margin: 0;
    font-size: var(--text-xs);
    font-weight: 500;
    color: var(--gray);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview-duration-badge {
    position: absolute;
    bottom: 4px;
    right: 4px;
    padding: 1px 5px;
    font-size: var(--text-xs);
    font-weight: 500;
    line-height: 1.3;
    color: #fff;
    background: rgba(0, 0, 0, 0.75);
    border-radius: var(--radius-xs);
    font-variant-numeric: tabular-nums;
    pointer-events: none;
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
