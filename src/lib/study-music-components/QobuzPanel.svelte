<script lang="ts">
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";

  type QobuzSettings = {
    email: string;
    format_id: number;
    configured: boolean;
  };

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  let settings = $state<QobuzSettings | null>(null);
  let email = $state("");
  let password = $state("");
  let showPassword = $state(false);
  let busy = $state(false);
  let testing = $state(false);

  async function load() {
    try {
      settings = await pluginInvoke<QobuzSettings>("study", "study:music:qobuz:get_settings");
    } catch {
      settings = null;
    }
  }

  $effect(() => {
    if (open) {
      void load();
      email = "";
      password = "";
      showPassword = false;
    }
  });

  async function connect() {
    const trimmedEmail = email.trim();
    const pwd = password;
    if (!trimmedEmail || !pwd || busy) return;
    busy = true;
    try {
      await pluginInvoke("study", "study:music:qobuz:set_credentials", {
        email: trimmedEmail,
        password: pwd,
      });
      testing = true;
      try {
        await pluginInvoke<{ ok: boolean }>("study", "study:music:qobuz:test");
        showToast("success", $t("study.music.qobuz_connected") as string);
        password = "";
        await load();
      } catch (e) {
        showToast(
          "error",
          $t("study.music.qobuz_test_failed", {
            error: e instanceof Error ? e.message : String(e),
          }) as string,
        );
        await load();
      } finally {
        testing = false;
      }
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function disconnect() {
    if (busy) return;
    if (!window.confirm($t("study.music.qobuz_disconnect_confirm") as string)) return;
    busy = true;
    try {
      await pluginInvoke("study", "study:music:qobuz:clear");
      showToast("success", $t("study.music.qobuz_disconnected") as string);
      await load();
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function retest() {
    if (busy || testing) return;
    testing = true;
    try {
      await pluginInvoke<{ ok: boolean }>("study", "study:music:qobuz:test");
      showToast("success", $t("study.music.qobuz_test_ok") as string);
    } catch (e) {
      showToast(
        "error",
        $t("study.music.qobuz_test_failed", {
          error: e instanceof Error ? e.message : String(e),
        }) as string,
      );
    } finally {
      testing = false;
    }
  }

  function maskedEmail(addr: string): string {
    if (!addr) return "";
    const at = addr.indexOf("@");
    if (at < 1) return addr;
    const user = addr.slice(0, at);
    const domain = addr.slice(at);
    if (user.length <= 2) return user[0] + "•" + domain;
    return user[0] + "•".repeat(Math.min(user.length - 2, 6)) + user[user.length - 1] + domain;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }

  const isConfigured = $derived(settings?.configured ?? false);
</script>

{#if open}
  <div
    class="overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    onkeydown={onKey}
  >
    <div class="dialog" role="dialog" aria-modal="true" tabindex="-1">
      <header class="head">
        <div class="head-text">
          <h3>{$t("study.music.qobuz_title")}</h3>
          {#if isConfigured}
            <span class="state on">✓ {$t("study.music.qobuz_state_connected")}</span>
          {:else}
            <span class="state off">{$t("study.music.qobuz_state_disconnected")}</span>
          {/if}
        </div>
        <button type="button" class="close" onclick={onClose} aria-label={$t("study.common.close") as string}>×</button>
      </header>
      <div class="body">
        {#if isConfigured}
          <div class="connected-block">
            <div class="connected-line">
              <span class="connected-label">{$t("study.music.qobuz_signed_in_as")}</span>
              <span class="connected-email">{maskedEmail(settings?.email ?? "")}</span>
            </div>
            <p class="hint">{$t("study.music.qobuz_connected_hint")}</p>
            <div class="row-actions">
              <button type="button" class="cta-secondary" onclick={retest} disabled={busy || testing}>
                {testing ? $t("study.music.qobuz_testing") : $t("study.music.qobuz_test_again")}
              </button>
              <button type="button" class="link danger" onclick={disconnect} disabled={busy}>
                {$t("study.music.qobuz_disconnect")}
              </button>
            </div>
          </div>
        {:else}
          <p class="hint">{$t("study.music.qobuz_intro")}</p>

          <label class="field">
            <span>{$t("study.music.qobuz_email_label")}</span>
            <input
              type="email"
              bind:value={email}
              placeholder="you@example.com"
              autocomplete="email"
              disabled={busy}
              onkeydown={(e) => { if (e.key === "Enter") connect(); }}
            />
          </label>

          <label class="field">
            <span>{$t("study.music.qobuz_password_label")}</span>
            <div class="pwd-wrap">
              {#if showPassword}
                <input
                  type="text"
                  bind:value={password}
                  autocomplete="current-password"
                  disabled={busy}
                  onkeydown={(e) => { if (e.key === "Enter") connect(); }}
                />
              {:else}
                <input
                  type="password"
                  bind:value={password}
                  autocomplete="current-password"
                  disabled={busy}
                  onkeydown={(e) => { if (e.key === "Enter") connect(); }}
                />
              {/if}
              <button
                type="button"
                class="pwd-toggle"
                onclick={() => (showPassword = !showPassword)}
                aria-label={$t(showPassword ? "study.music.qobuz_pwd_hide" : "study.music.qobuz_pwd_show") as string}
                tabindex={-1}
              >
                {showPassword ? "🙈" : "👁"}
              </button>
            </div>
          </label>

          <p class="privacy">{$t("study.music.qobuz_privacy")}</p>

          <div class="row-actions">
            <button
              type="button"
              class="cta"
              onclick={connect}
              disabled={busy || !email.trim() || !password}
            >
              {busy ? $t("study.music.qobuz_connecting") : $t("study.music.qobuz_connect")}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    z-index: 320;
    display: grid;
    place-items: center;
    backdrop-filter: blur(4px);
  }
  .dialog {
    background: rgb(20, 20, 20);
    color: rgba(255, 255, 255, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    width: min(520px, 92vw);
    max-height: 88vh;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; padding: 18px 20px 8px; }
  .head-text { display: flex; flex-direction: column; gap: 4px; }
  .head h3 { margin: 0; font-size: 16px; font-weight: 800; }
  .state { display: inline-flex; align-items: center; gap: 6px; padding: 2px 10px; border-radius: 999px; font-size: 11px; font-weight: 600; width: fit-content; }
  .state.on { background: rgba(34, 197, 94, 0.15); color: rgb(74, 222, 128); }
  .state.off { background: rgba(255, 255, 255, 0.06); color: rgba(255, 255, 255, 0.45); }
  .close { width: 28px; height: 28px; background: transparent; border: 0; border-radius: 50%; color: rgba(255, 255, 255, 0.5); font-size: 18px; cursor: pointer; flex-shrink: 0; }
  .close:hover { color: rgba(255, 255, 255, 0.9); background: rgba(255, 255, 255, 0.06); }
  .body { padding: 8px 20px 20px; overflow-y: auto; display: flex; flex-direction: column; gap: 14px; }
  .hint { margin: 0; color: rgba(255, 255, 255, 0.65); font-size: 13px; line-height: 1.5; }

  .connected-block { display: flex; flex-direction: column; gap: 10px; }
  .connected-line { display: flex; align-items: baseline; gap: 8px; padding: 10px 12px; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 8px; }
  .connected-label { font-size: 11px; color: rgba(255, 255, 255, 0.5); text-transform: uppercase; letter-spacing: 0.06em; }
  .connected-email { font-size: 13px; font-weight: 600; font-family: ui-monospace, monospace; }

  .field { display: flex; flex-direction: column; gap: 4px; }
  .field span { font-size: 11px; color: rgba(255, 255, 255, 0.55); text-transform: uppercase; letter-spacing: 0.06em; font-weight: 600; }
  .field input { padding: 10px 12px; background: rgba(0, 0, 0, 0.3); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 6px; color: rgba(255, 255, 255, 0.95); font-family: inherit; font-size: 13px; outline: none; }
  .field input:focus { border-color: var(--accent); }
  .field input:disabled { opacity: 0.5; }

  .pwd-wrap { position: relative; }
  .pwd-wrap input { width: 100%; padding-right: 40px; box-sizing: border-box; }
  .pwd-toggle {
    position: absolute;
    right: 6px; top: 50%;
    transform: translateY(-50%);
    width: 28px; height: 28px;
    background: transparent; border: 0;
    color: rgba(255, 255, 255, 0.55);
    cursor: pointer;
    border-radius: 4px;
    font-size: 14px;
    display: grid; place-items: center;
  }
  .pwd-toggle:hover { background: rgba(255, 255, 255, 0.08); color: rgba(255, 255, 255, 0.95); }

  .privacy { margin: 0; font-size: 11px; color: rgba(255, 255, 255, 0.5); line-height: 1.5; }

  .row-actions { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .cta { padding: 9px 18px; border: 0; border-radius: 999px; background: var(--accent); color: var(--on-accent, white); font-family: inherit; font-size: 13px; font-weight: 700; cursor: pointer; }
  .cta:disabled { opacity: 0.5; cursor: default; }
  .cta-secondary { padding: 7px 14px; border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 999px; background: transparent; color: rgba(255, 255, 255, 0.95); font-family: inherit; font-size: 12px; cursor: pointer; }
  .cta-secondary:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .cta-secondary:disabled { opacity: 0.4; cursor: default; }
  .link { background: transparent; border: 0; color: rgba(255, 255, 255, 0.5); font-family: inherit; font-size: 11px; cursor: pointer; padding: 0; font-weight: 600; }
  .link.danger:hover { color: rgb(248, 113, 113); }
  .link:disabled { opacity: 0.4; cursor: default; }
</style>
