<script lang="ts">
  import { t } from "$lib/i18n";

  type Props = {
    text?: string | null;
    maxLines?: number;
    moreLabel?: string;
    lessLabel?: string;
  };

  let {
    text = null,
    maxLines = 3,
    moreLabel,
    lessLabel,
  }: Props = $props();

  let expanded = $state(false);
  let overflows = $state(false);
  let contentEl: HTMLDivElement | null = $state(null);

  const showButton = $derived(overflows);
  const labelMore = $derived(moreLabel ?? $t("study.music.expandable_more"));
  const labelLess = $derived(lessLabel ?? $t("study.music.expandable_less"));

  $effect(() => {
    if (!contentEl) return;
    if (!text) {
      overflows = false;
      return;
    }
    const el = contentEl;
    requestAnimationFrame(() => {
      overflows = el.scrollHeight > el.clientHeight + 1;
    });
  });
</script>

{#if text && text.trim() !== ""}
  <div class="expandable">
    <div
      class="content"
      class:clamped={!expanded && showButton}
      style:--max-lines={maxLines}
      bind:this={contentEl}
    >
      {text}
    </div>
    {#if showButton}
      <button
        type="button"
        class="toggle"
        onclick={() => (expanded = !expanded)}
        aria-expanded={expanded}
      >
        {expanded ? labelLess : labelMore}
      </button>
    {/if}
  </div>
{/if}

<style>
  .expandable {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .content {
    font-size: 13px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.78);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .clamped {
    display: -webkit-box;
    -webkit-line-clamp: var(--max-lines);
    line-clamp: var(--max-lines);
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .toggle {
    align-self: flex-start;
    margin-top: 2px;
    padding: 4px 0;
    background: transparent;
    border: 0;
    color: var(--accent);
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }
  .toggle:hover {
    text-decoration: underline;
  }
</style>
