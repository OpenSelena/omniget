<script lang="ts">
  import type { Snippet } from "svelte";
  type Props = {
    title: string;
    body?: string | null;
    ctaLabel?: string | null;
    onCta?: (() => void) | null;
    ctaHref?: string | null;
    icon?: Snippet;
    compact?: boolean;
  };
  let {
    title,
    body = null,
    ctaLabel = null,
    onCta = null,
    ctaHref = null,
    icon,
    compact = false,
  }: Props = $props();
</script>

<div class="empty" class:compact>
  {#if icon}
    <div class="icon">{@render icon()}</div>
  {/if}
  <h3>{title}</h3>
  {#if body}<p>{body}</p>{/if}
  {#if ctaLabel && ctaHref}
    <a class="cta" href={ctaHref}>{ctaLabel}</a>
  {:else if ctaLabel && onCta}
    <button type="button" class="cta" onclick={() => onCta?.()}>{ctaLabel}</button>
  {/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 40px 24px;
    text-align: center;
    color: rgba(255, 255, 255, 0.85);
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px dashed rgba(255, 255, 255, 0.06);
  }
  .empty.compact {
    padding: 24px 16px;
    gap: 6px;
  }
  .icon {
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.55);
    margin-bottom: 4px;
  }
  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
    color: white;
  }
  p {
    margin: 0;
    color: rgba(255, 255, 255, 0.55);
    font-size: 13px;
    line-height: 1.5;
    max-width: 42ch;
  }
  .cta {
    margin-top: 10px;
    padding: 9px 20px;
    background: var(--accent);
    color: var(--on-accent, #000);
    border: 0;
    border-radius: 999px;
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    text-decoration: none;
  }
  .cta:hover {
    filter: brightness(1.08);
  }
</style>
