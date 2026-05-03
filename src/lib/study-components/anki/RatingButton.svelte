<script lang="ts">
  type Rating = "again" | "hard" | "good" | "easy";

  let {
    rating,
    shortcut,
    interval = "",
    onclick,
    disabled = false,
  }: {
    rating: Rating;
    shortcut: string;
    interval?: string;
    onclick: () => void;
    disabled?: boolean;
  } = $props();

  const meta: Record<Rating, { emoji: string; label: string }> = {
    again: { emoji: "🙈", label: "De novo" },
    hard: { emoji: "😬", label: "Difícil" },
    good: { emoji: "😐", label: "Bom" },
    easy: { emoji: "😊", label: "Fácil" },
  };
  const info = $derived(meta[rating as Rating]);
</script>

<button
  type="button"
  class="rate rate--{rating}"
  {disabled}
  onclick={onclick}
  aria-label="{info.label} ({shortcut})"
>
  <span class="rate-emoji" aria-hidden="true">{info.emoji}</span>
  <span class="rate-label">{info.label}</span>
  {#if interval}
    <span class="rate-interval">{interval}</span>
  {/if}
  <span class="rate-shortcut">{shortcut}</span>
</button>

<style>
  .rate {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-2);
    background: var(--surface-hi);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-family: inherit;
    cursor: pointer;
    min-width: 110px;
    transition: background var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out),
      transform var(--duration-fast) var(--ease-out);
  }

  .rate:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: var(--elev-2);
  }

  .rate:active:not(:disabled) {
    transform: translateY(0);
  }

  .rate:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .rate-emoji {
    font-size: 28px;
    line-height: 1;
  }

  .rate-label {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .rate-interval {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .rate-shortcut {
    font-size: 10px;
    font-family: monospace;
    background: var(--surface-mut);
    padding: 1px 6px;
    border-radius: 4px;
    color: var(--text-muted);
  }

  .rate--again:hover:not(:disabled) {
    border-color: var(--error);
    background: color-mix(in srgb, var(--error) 14%, transparent);
  }
  .rate--hard:hover:not(:disabled) {
    border-color: var(--warning);
    background: color-mix(in srgb, var(--warning) 14%, transparent);
  }
  .rate--good:hover:not(:disabled) {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .rate--easy:hover:not(:disabled) {
    border-color: var(--success);
    background: color-mix(in srgb, var(--success) 14%, transparent);
  }
</style>
