<script lang="ts">
  import type { Snippet } from "svelte";
  type Props = {
    label: string;
    href?: string | null;
    onClick?: (() => void) | null;
    icon?: Snippet;
    accentTint?: string | null;
  };
  let { label, href = null, onClick = null, icon, accentTint = null }: Props = $props();
</script>

{#if href}
  <a class="dial-item" {href} style:--tint={accentTint ?? "rgba(255,255,255,0.08)"}>
    <span class="icon-wrap">
      {#if icon}{@render icon()}{/if}
    </span>
    <span class="label">{label}</span>
  </a>
{:else}
  <button
    type="button"
    class="dial-item"
    style:--tint={accentTint ?? "rgba(255,255,255,0.08)"}
    onclick={() => onClick?.()}
  >
    <span class="icon-wrap">
      {#if icon}{@render icon()}{/if}
    </span>
    <span class="label">{label}</span>
  </button>
{/if}

<style>
  .dial-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 14px 8px 12px;
    background: var(--tint);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    color: white;
    font: inherit;
    cursor: pointer;
    text-decoration: none;
    transition: transform 160ms ease, background 160ms ease, border-color 160ms ease;
    min-width: 0;
  }
  .dial-item:hover {
    transform: translateY(-2px);
    border-color: rgba(255, 255, 255, 0.2);
  }
  .icon-wrap {
    width: 44px;
    height: 44px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.08);
    display: grid;
    place-items: center;
    color: white;
  }
  .label {
    font-size: 12px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.92);
    text-align: center;
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }
  @media (prefers-reduced-motion: reduce) {
    .dial-item { transition: none; }
    .dial-item:hover { transform: none; }
  }
</style>
