<script lang="ts">
  import { musicPlayer, type PlayerErrorKind } from "$lib/study-music/player-store.svelte";
  import { t } from "$lib/i18n";

  const err = $derived(musicPlayer.lastError);

  const messageKeys: Record<PlayerErrorKind, string> = {
    network: "study.music.player_error_network",
    auth_expired: "study.music.player_error_auth_expired",
    track_unavailable: "study.music.player_error_track_unavailable",
    region_blocked: "study.music.player_error_region_blocked",
    unsupported_format: "study.music.player_error_unsupported_format",
    format_unavailable: "study.music.player_error_format_unavailable",
    unknown: "study.music.player_error_unknown",
  };

  function primary() {
    if (!err) return;
    if (err.primaryAction === "refresh") {
      void musicPlayer.refreshAndRetry();
    } else if (err.primaryAction === "retry") {
      musicPlayer.retry();
    } else {
      musicPlayer.skipOnError();
    }
  }

  function secondary() {
    if (!err) return;
    musicPlayer.skipOnError();
  }

  function dismiss() {
    musicPlayer.clearError();
  }

  function toggleAutoSkip(e: Event) {
    const target = e.currentTarget as HTMLInputElement;
    void musicPlayer.setAutoSkipOnError(target.checked);
  }
</script>

{#if err}
  <div class="banner" role="alert" aria-live="polite">
    <div class="msg">{$t(messageKeys[err.kind])}</div>
    <div class="actions">
      <button type="button" class="primary" onclick={primary}>
        {#if err.primaryAction === "refresh"}
          {$t("study.music.player_action_refresh")}
        {:else if err.primaryAction === "retry"}
          {$t("study.music.player_action_retry")}
        {:else}
          {$t("study.music.player_action_skip")}
        {/if}
      </button>
      {#if err.primaryAction !== "skip"}
        <button type="button" class="secondary" onclick={secondary}>
          {$t("study.music.player_action_skip")}
        </button>
      {/if}
      <label class="auto-skip" title={$t("study.music.player_auto_skip_label") as string}>
        <input
          type="checkbox"
          checked={musicPlayer.autoSkipOnError}
          onchange={toggleAutoSkip}
        />
        <span>{$t("study.music.player_auto_skip_label")}</span>
      </label>
      <button
        type="button"
        class="dismiss"
        onclick={dismiss}
        aria-label={$t("study.music.player_dismiss_error") as string}
        title={$t("study.music.player_dismiss_error") as string}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <line x1="18" y1="6" x2="6" y2="18"/>
          <line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>
  </div>
{/if}

<style>
  .banner {
    position: fixed;
    right: 18px;
    bottom: 96px;
    z-index: 100;
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: min(520px, calc(100vw - 36px));
    padding: 10px 14px;
    background: color-mix(in oklab, var(--accent) 8%, var(--primary));
    border: 1px solid color-mix(in oklab, var(--accent) 35%, transparent);
    border-radius: 12px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    font-size: 13px;
    color: var(--secondary);
    animation: banner-in 220ms ease-out both;
  }
  @keyframes banner-in {
    from { transform: translateY(8px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
  :global(.music-shell[data-reduce-animations="true"]) .banner {
    animation: none;
  }
  .msg {
    flex: 0 1 auto;
    font-weight: 500;
    line-height: 1.35;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    flex-shrink: 0;
  }
  .primary {
    padding: 5px 12px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: filter 120ms ease, transform 100ms ease;
  }
  .primary:hover { filter: brightness(1.1); }
  .primary:active { transform: scale(0.97); }
  .secondary {
    padding: 5px 10px;
    border: 1px solid color-mix(in oklab, var(--content-border) 80%, transparent);
    border-radius: 999px;
    background: transparent;
    color: var(--secondary);
    font-size: 12px;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .secondary:hover {
    background: color-mix(in oklab, var(--secondary) 8%, transparent);
  }
  .auto-skip {
    display: none;
  }
  .dismiss {
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--tertiary);
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .dismiss:hover { color: var(--secondary); }
  @media (max-width: 760px) {
    .banner {
      left: 12px;
      right: 12px;
      bottom: 92px;
      max-width: none;
      flex-wrap: wrap;
      padding: 10px 12px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .primary { transition: none; }
  }
</style>
