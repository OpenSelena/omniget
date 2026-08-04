<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { studyYoutubeHumanizeError } from "$lib/study-bridge";

  type Props = {
    error: string;
    onRetry?: () => void;
    fallbackLabel?: string;
    onFallback?: () => void;
  };
  let { error, onRetry, fallbackLabel, onFallback }: Props = $props();

  let humanized = $state<string>("");
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
    <p class="msg">{humanized || $t("study.music.youtube_error_generic")}</p>
    <div class="actions">
      {#if onRetry}
        <button type="button" class="btn primary" onclick={onRetry}>{$t("study.music.youtube_error_retry")}</button>
      {/if}
      {#if onFallback}
        <button type="button" class="btn ghost" onclick={onFallback}>{fallbackLabel ?? ($t("study.music.youtube_error_audio_only") as string)}</button>
      {/if}
      <button
        type="button"
        class="btn link"
        onclick={() => (showDetails = !showDetails)}
        aria-expanded={showDetails}
      >
        {showDetails ? $t("study.music.youtube_error_hide_details") : $t("study.music.youtube_error_details")}
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
    padding: 16px 18px;
    background: #1f1f1f;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    color: #f1f1f1;
  }
  .yt-error svg { color: #aaa; margin-top: 2px; }
  .body { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .msg { margin: 0; font-size: 14px; color: #f1f1f1; }
  .actions { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .btn {
    padding: 8px 14px;
    border-radius: 999px;
    border: 0;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn.primary { background: #cc0000; color: #fff; }
  .btn.primary:hover { background: #e60000; }
  .btn.ghost {
    background: transparent;
    color: #f1f1f1;
    border: 1px solid rgba(255, 255, 255, 0.22);
  }
  .btn.ghost:hover { background: rgba(255, 255, 255, 0.08); }
  .btn.link {
    background: transparent;
    color: #aaa;
    padding: 8px 0;
    font-weight: 500;
  }
  .btn.link:hover { color: #f1f1f1; }
  .raw {
    margin: 0;
    padding: 10px 12px;
    background: #0f0f0f;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    font-size: 12px;
    font-family: ui-monospace, "Cascadia Mono", Menlo, Consolas, monospace;
    color: #aaa;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow: auto;
  }
</style>
