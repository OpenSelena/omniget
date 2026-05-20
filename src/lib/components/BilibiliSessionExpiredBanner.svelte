<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  import { showToast } from "$lib/stores/toast-store.svelte";

  const DISMISS_KEY = "bilibili-session-banner-dismissed-until";
  const DISMISS_WINDOW_MS = 24 * 60 * 60 * 1000;

  let visible = $state(false);
  let busy = $state(false);
  let unlisten: UnlistenFn | null = null;

  function isDismissed(): boolean {
    try {
      const until = parseInt(localStorage.getItem(DISMISS_KEY) || "0", 10);
      return Number.isFinite(until) && Date.now() < until;
    } catch {
      return false;
    }
  }

  onMount(async () => {
    unlisten = await listen("bilibili-session-expired", () => {
      if (!isDismissed()) {
        visible = true;
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  function dismiss() {
    try {
      localStorage.setItem(DISMISS_KEY, String(Date.now() + DISMISS_WINDOW_MS));
    } catch {}
    visible = false;
  }

  async function reLogin() {
    if (busy) return;
    busy = true;
    try {
      const res = await invoke<{ slug: string; uname: string }>("bilibili_webview_login");
      showToast(
        "success",
        ($t("settings.cookies.bilibili.login_qr_success", { uname: res.uname }) as string) ||
          `Signed in as ${res.uname}`,
      );
      try {
        localStorage.removeItem(DISMISS_KEY);
      } catch {}
      visible = false;
    } catch (e) {
      const key = typeof e === "string" ? e : "errors.bilibili.network_failed";
      showToast("error", ($t(key) as string) || String(e));
    } finally {
      busy = false;
    }
  }
</script>

{#if visible}
  <div class="banner" role="alert">
    <div class="banner-body">
      <strong class="banner-title">{$t("platforms.bilibili.notice.session_expired_title")}</strong>
      <span class="banner-text">{$t("platforms.bilibili.notice.session_expired_body")}</span>
    </div>
    <div class="banner-actions">
      <button type="button" class="primary" onclick={reLogin} disabled={busy}>
        {$t("platforms.bilibili.notice.session_expired_action")}
      </button>
      <button type="button" class="ghost" onclick={dismiss}>
        {$t("platforms.bilibili.notice.session_expired_dismiss")}
      </button>
    </div>
  </div>
{/if}

<style>
  .banner {
    position: fixed;
    bottom: 18px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 900;
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 12px 16px;
    background: var(--surface);
    border: 1px solid color-mix(in oklab, var(--accent) 35%, var(--content-border));
    border-radius: 12px;
    box-shadow: 0 6px 20px color-mix(in oklab, var(--accent) 10%, transparent);
    max-width: 720px;
    width: calc(100% - 36px);
  }
  .banner-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }
  .banner-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .banner-text {
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.4;
  }
  .banner-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .primary,
  .ghost {
    padding: 6px 12px;
    border-radius: 8px;
    font-size: 12px;
    cursor: pointer;
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border: 0;
  }
  .ghost {
    background: transparent;
    color: var(--text);
    border: 1px solid var(--content-border);
  }
  .primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .ghost:hover {
    background: color-mix(in oklab, var(--text) 6%, transparent);
  }
</style>
