<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    description?: string;
    tone?: "neutral" | "warn" | "err";
    icon?: Snippet;
    actions?: Snippet;
  }

  let {
    title,
    description,
    tone = "neutral",
    icon,
    actions,
  }: Props = $props();
</script>

<div class="empty-state" data-tone={tone}>
  {#if icon}
    <span class="icon-slot">{@render icon()}</span>
  {/if}
  <h3>{title}</h3>
  {#if description}<p>{description}</p>{/if}
  {#if actions}
    <div class="actions">{@render actions()}</div>
  {/if}
</div>

<style>
  .empty-state {
    --tone-color: var(--tertiary);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: calc(var(--padding, 10px) * 2);
    background: var(--button-elevated);
    border: 1px dashed color-mix(in srgb, var(--tone-color) 40%, var(--content-border));
    border-radius: var(--border-radius, 11px);
  }

  .empty-state[data-tone="warn"] { --tone-color: var(--warning, var(--orange)); }
  .empty-state[data-tone="err"] { --tone-color: var(--error, var(--red)); }

  .icon-slot {
    display: inline-flex;
    width: 40px;
    height: 40px;
    align-items: center;
    justify-content: center;
    color: var(--tone-color);
    background: color-mix(in srgb, var(--tone-color) 12%, transparent);
    border-radius: var(--border-radius, 11px);
  }

  h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--secondary);
    letter-spacing: 0.2px;
  }

  p {
    margin: 0;
    color: var(--tertiary);
    font-size: 13px;
    line-height: 1.5;
  }

  .actions {
    display: flex;
    gap: var(--padding, 10px);
    margin-top: 4px;
  }
</style>
