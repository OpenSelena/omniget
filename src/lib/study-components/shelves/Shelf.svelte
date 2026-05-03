<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    title: string;
    eyebrow?: string;
    isLoading?: boolean;
    seeMoreHref?: string;
    seeMoreLabel?: string;
    children?: Snippet;
    skeleton?: Snippet;
    empty?: Snippet;
    isEmpty?: boolean;
  };

  let {
    title,
    eyebrow,
    isLoading = false,
    seeMoreHref,
    seeMoreLabel = "Ver mais",
    children,
    skeleton,
    empty,
    isEmpty = false,
  }: Props = $props();

  let scroller = $state<HTMLDivElement | null>(null);

  function scrollBy(direction: 1 | -1) {
    if (!scroller) return;
    scroller.scrollBy({
      left: direction * Math.max(scroller.clientWidth * 0.8, 320),
      behavior: "smooth",
    });
  }
</script>

<section class="shelf" aria-label={title}>
  <header class="shelf-head">
    <div class="head-text">
      {#if eyebrow}
        <span class="eyebrow">{eyebrow}</span>
      {/if}
      <h2 class="title">{title}</h2>
    </div>
    <div class="head-actions">
      <button
        type="button"
        class="nav-btn"
        aria-label="Rolar para esquerda"
        onclick={() => scrollBy(-1)}
        disabled={isLoading || isEmpty}
      >
        ‹
      </button>
      <button
        type="button"
        class="nav-btn"
        aria-label="Rolar para direita"
        onclick={() => scrollBy(1)}
        disabled={isLoading || isEmpty}
      >
        ›
      </button>
      {#if seeMoreHref && !isLoading && !isEmpty}
        <a class="see-more" href={seeMoreHref}>{seeMoreLabel} →</a>
      {/if}
    </div>
  </header>
  <div class="track" bind:this={scroller}>
    {#if isLoading}
      {#if skeleton}
        {@render skeleton()}
      {:else}
        {#each Array(4) as _, i (i)}
          <div class="card-skeleton" aria-hidden="true">
            <div class="sk-thumb"></div>
            <div class="sk-line"></div>
          </div>
        {/each}
      {/if}
    {:else if isEmpty}
      {#if empty}
        {@render empty()}
      {:else}
        <div class="empty">Nenhum item</div>
      {/if}
    {:else}
      {@render children?.()}
    {/if}
  </div>
</section>

<style>
  .shelf {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 0;
  }

  .shelf-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 0 4px;
  }

  .head-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .eyebrow {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--accent);
    font-weight: 600;
  }

  .title {
    font-size: 22px;
    font-weight: 600;
    margin: 0;
    line-height: 1.1;
  }

  .head-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .nav-btn {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: color-mix(in oklab, var(--content-bg) 80%, var(--accent) 4%);
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    color: inherit;
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: background var(--tg-duration-fast, 150ms);
  }

  .nav-btn:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 12%, var(--content-bg) 88%);
  }

  .nav-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .see-more {
    font-size: 13px;
    font-weight: 500;
    color: var(--accent);
    text-decoration: none;
    margin-left: 4px;
  }

  .see-more:hover {
    text-decoration: underline;
  }

  .track {
    display: flex;
    gap: 16px;
    overflow-x: auto;
    overflow-y: hidden;
    scroll-snap-type: x proximity;
    padding: 4px 4px 12px;
    scrollbar-width: thin;
  }

  .track > :global(*) {
    scroll-snap-align: start;
  }

  .card-skeleton {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 240px;
    flex: 0 0 240px;
  }

  .sk-thumb {
    aspect-ratio: 16 / 9;
    border-radius: 10px;
    background: linear-gradient(
      90deg,
      color-mix(in oklab, var(--content-bg) 80%, var(--content-border) 20%) 0%,
      color-mix(in oklab, var(--content-bg) 60%, var(--content-border) 40%) 50%,
      color-mix(in oklab, var(--content-bg) 80%, var(--content-border) 20%) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.6s ease-in-out infinite;
  }

  .sk-line {
    height: 14px;
    width: 70%;
    border-radius: 4px;
    background: color-mix(in oklab, var(--content-bg) 70%, var(--content-border) 30%);
  }

  .empty {
    color: color-mix(in oklab, currentColor 60%, transparent);
    font-size: 13px;
    padding: 16px 0;
  }

  @keyframes shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sk-thumb {
      animation: none;
    }
  }
</style>
