<script lang="ts">
  import type { Snippet } from "svelte";
  type Props = {
    title: string;
    subtitle?: string | null;
    href?: string | null;
    onClick?: (() => void) | null;
    icon?: Snippet;
    accent?: string | null;
  };
  let { title, subtitle = null, href = null, onClick = null, icon, accent = null }: Props = $props();
  const tag = href ? "a" : "button";
  function handle() {
    onClick?.();
  }
</script>

{#if href}
  <a class="nav-tile" {href} style:--tile-accent={accent ?? "rgba(255,255,255,0.05)"}>
    <span class="icon">
      {#if icon}{@render icon()}{/if}
    </span>
    <span class="text">
      <span class="title">{title}</span>
      {#if subtitle}<span class="sub">{subtitle}</span>{/if}
    </span>
  </a>
{:else}
  <button
    type="button"
    class="nav-tile"
    style:--tile-accent={accent ?? "rgba(255,255,255,0.05)"}
    onclick={handle}
  >
    <span class="icon">
      {#if icon}{@render icon()}{/if}
    </span>
    <span class="text">
      <span class="title">{title}</span>
      {#if subtitle}<span class="sub">{subtitle}</span>{/if}
    </span>
  </button>
{/if}

<style>
  .nav-tile {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 20px;
    background: linear-gradient(135deg, var(--tile-accent), rgba(255,255,255,0.02));
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    color: inherit;
    cursor: pointer;
    text-align: left;
    text-decoration: none;
    font: inherit;
    transition: transform 160ms ease, background 160ms ease, border-color 160ms ease;
    min-width: 0;
  }
  .nav-tile:hover {
    transform: translateY(-2px);
    border-color: rgba(255, 255, 255, 0.18);
  }
  .icon {
    flex: 0 0 auto;
    width: 36px;
    height: 36px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.07);
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.95);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .title {
    font-size: 14px;
    font-weight: 700;
    color: white;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (prefers-reduced-motion: reduce) {
    .nav-tile { transition: none; }
    .nav-tile:hover { transform: none; }
  }
</style>
