<script lang="ts">
  import type { Snippet } from "svelte";

  interface BreadcrumbItem {
    label: string;
    href?: string;
  }

  interface Props {
    breadcrumb?: BreadcrumbItem[];
    title: string;
    subtitle?: string;
    actions?: Snippet;
    backgroundImage?: string | null;
    backgroundBlur?: number;
  }

  let {
    breadcrumb = [],
    title,
    subtitle = "",
    actions,
    backgroundImage = null,
    backgroundBlur = 40,
  }: Props = $props();
</script>

<header class="page-hero" class:has-bg={backgroundImage}>
  {#if backgroundImage}
    <div
      class="hero-bg"
      style:background-image={`url('${backgroundImage}')`}
      style:filter={`blur(${Math.max(0, Math.min(100, backgroundBlur))}px)`}
      aria-hidden="true"
    ></div>
    <div class="hero-overlay" aria-hidden="true"></div>
  {/if}
  <div class="head">
    <div class="titles">
      {#if breadcrumb && breadcrumb.length > 0}
        <nav class="breadcrumb" aria-label="breadcrumb">
          {#each breadcrumb as item, i (i)}
            {#if i > 0}<span class="sep" aria-hidden="true">›</span>{/if}
            {#if i === breadcrumb.length - 1}
              <span class="current">{item.label}</span>
            {:else if item.href}
              <a href={item.href}>{item.label}</a>
            {:else}
              <span>{item.label}</span>
            {/if}
          {/each}
        </nav>
      {/if}
      <h1>{title}</h1>
      {#if subtitle}<p class="subtitle">{subtitle}</p>{/if}
    </div>
    {#if actions}
      <div class="actions">{@render actions()}</div>
    {/if}
  </div>
</header>

<style>
  .page-hero {
    margin-bottom: calc(var(--padding, 10px) * 2);
    position: relative;
  }
  .page-hero.has-bg {
    border-radius: 12px;
    overflow: hidden;
    padding: 32px 28px;
    isolation: isolate;
    min-height: 200px;
  }
  .hero-bg {
    position: absolute;
    inset: -40px;
    background-position: center;
    background-size: cover;
    background-repeat: no-repeat;
    opacity: 0.55;
    z-index: -2;
  }
  .hero-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      135deg,
      color-mix(in oklab, var(--content-bg) 70%, transparent) 0%,
      color-mix(in oklab, var(--content-bg) 92%, transparent) 100%
    );
    z-index: -1;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .titles {
    flex: 1 1 auto;
    min-width: 0;
  }
  .breadcrumb {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    font-size: 12px;
    color: var(--tertiary);
    margin-bottom: 0.35rem;
  }
  .breadcrumb a {
    color: var(--tertiary);
    text-decoration: none;
    transition: color 150ms ease;
  }
  .breadcrumb a:hover {
    color: var(--secondary);
  }
  .breadcrumb .sep {
    opacity: 0.6;
  }
  .breadcrumb .current {
    color: var(--secondary);
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-3xl);
    line-height: var(--leading-3xl);
    font-weight: 600;
    letter-spacing: -0.02em;
    color: var(--text);
  }
  .subtitle {
    margin: 0.25rem 0 0;
    font-size: 14px;
    color: var(--tertiary);
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-shrink: 0;
  }
</style>
