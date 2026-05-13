<script lang="ts">
  import { onMount } from "svelte";
  import { studyYoutubeHumanizeError } from "$lib/study-bridge";

  type Props = {
    error: string;
    onRetry?: () => void;
    fallbackLabel?: string;
    onFallback?: () => void;
  };
  let { error, onRetry, fallbackLabel, onFallback }: Props = $props();

  let humanized = $state<string>("Algo deu errado ao falar com o YouTube");
  let showDetails = $state(false);

  async function resolve(err: string) {
    try {
      const res = await studyYoutubeHumanizeError({ error: err });
      humanized = res.humanized;
    } catch {
      /* keep default */
    }
  }

  $effect(() => {
    void resolve(error);
    console.warn("[study-music] yt error:", error);
  });
</script>

<div class="yt-error" role="alert">
  <svg
    viewBox="0 0 24 24"
    width="22"
    height="22"
    fill="none"
    stroke="currentColor"
    stroke-width="1.6"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <circle cx="12" cy="12" r="10" />
    <line x1="12" y1="8" x2="12" y2="13" />
    <line x1="12" y1="16" x2="12" y2="16.01" />
  </svg>
  <div class="body">
    <p class="msg">{humanized}</p>
    <div class="actions">
      {#if onRetry}
        <button type="button" class="btn primary" onclick={onRetry}>Tentar de novo</button>
      {/if}
      {#if onFallback}
        <button type="button" class="btn ghost" onclick={onFallback}>{fallbackLabel ?? "Ouvir só áudio"}</button>
      {/if}
      <button
        type="button"
        class="btn link"
        onclick={() => (showDetails = !showDetails)}
        aria-expanded={showDetails}
      >
        {showDetails ? "Esconder detalhes" : "Detalhes técnicos"}
      </button>
    </div>
    {#if showDetails}
      <pre class="raw">{error}</pre>
    {/if}
  </div>
</div>

<style>
  .yt-error {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 12px;
    padding: 14px 16px;
    background: color-mix(in oklab, var(--button) 40%, transparent);
    border: 1px solid color-mix(in oklab, var(--content-border) 40%, transparent);
    border-radius: 10px;
    color: var(--secondary);
  }
  .yt-error svg { color: var(--tertiary); margin-top: 2px; }
  .body { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .msg { margin: 0; font-size: 14px; color: var(--primary); }
  .actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .btn {
    padding: 6px 12px;
    border-radius: 999px;
    border: 0;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn.primary { background: var(--accent); color: #fff; }
  .btn.ghost {
    background: transparent;
    color: var(--secondary);
    border: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
  }
  .btn.link {
    background: transparent;
    color: var(--tertiary);
    padding: 6px 0;
    font-weight: 500;
  }
  .btn.link:hover { color: var(--secondary); }
  .raw {
    margin: 0;
    padding: 8px 10px;
    background: color-mix(in oklab, var(--button) 70%, transparent);
    border-radius: 6px;
    font-size: 11px;
    font-family: var(--mono, monospace);
    color: var(--tertiary);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow: auto;
  }
</style>
