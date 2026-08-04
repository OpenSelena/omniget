<script lang="ts">
  import { onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";

  type Status = {
    username: string;
    api_key_set: boolean;
    api_secret_set: boolean;
    session_key_set: boolean;
    enabled: boolean;
  };

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  let status = $state<Status | null>(null);
  let apiKey = $state("");
  let apiSecret = $state("");
  let busy = $state(false);
  let pendingToken = $state<string | null>(null);
  let pendingAuthUrl = $state<string | null>(null);

  async function load() {
    try {
      status = await pluginInvoke<Status>(
        "study",
        "study:music:lastfm:get_settings",
      );
    } catch {
      status = null;
    }
  }

  $effect(() => {
    if (open) void load();
  });

  async function saveKeys() {
    if (busy) return;
    busy = true;
    try {
      const res = await pluginInvoke<Status>("study", "study:music:lastfm:set_settings", {
        api_key: apiKey,
        api_secret: apiSecret,
      });
      status = res;
      apiKey = "";
      apiSecret = "";
      showToast("success", $t("study.music.lastfm_keys_saved") as string);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function startAuth() {
    if (busy) return;
    busy = true;
    try {
      const res = await pluginInvoke<{ token: string; auth_url: string }>(
        "study",
        "study:music:lastfm:auth_token",
      );
      pendingToken = res.token;
      pendingAuthUrl = res.auth_url;
      try {
        const { openUrl } = await import("@tauri-apps/plugin-opener");
        await openUrl(res.auth_url);
      } catch {
        /* user can click link */
      }
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function finalizeAuth() {
    if (!pendingToken || busy) return;
    busy = true;
    try {
      const res = await pluginInvoke<Status>(
        "study",
        "study:music:lastfm:auth_finalize",
        { token: pendingToken },
      );
      status = res;
      pendingToken = null;
      pendingAuthUrl = null;
      showToast(
        "success",
        $t("study.music.lastfm_connected", { username: res.username }) as string,
      );
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function toggleEnabled() {
    if (!status || busy) return;
    busy = true;
    try {
      const res = await pluginInvoke<Status>("study", "study:music:lastfm:set_settings", {
        enabled: !status.enabled,
      });
      status = res;
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function disconnect() {
    if (busy || typeof window === "undefined") return;
    if (!window.confirm($t("study.music.lastfm_disconnect_confirm") as string)) return;
    busy = true;
    try {
      await pluginInvoke("study", "study:music:lastfm:clear");
      await load();
      showToast("success", $t("study.music.lastfm_disconnected") as string);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }
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
        <h3>{$t("study.music.lastfm_title")}</h3>
        <button type="button" class="close" onclick={onClose} aria-label={$t("study.common.close") as string}>×</button>
      </header>
      <div class="body">
        <p class="hint">{$t("study.music.lastfm_intro")}</p>

        {#if status === null}
          <p class="muted">{$t("study.common.loading")}</p>
        {:else if !status.session_key_set}
          <section class="step">
            <span class="step-num">1</span>
            <div class="step-body">
              <h4>{$t("study.music.lastfm_step1_title")}</h4>
              <p class="step-desc">{$t("study.music.lastfm_step1_desc")}</p>
              <a
                class="ext-link"
                href="https://www.last.fm/api/account/create"
                target="_blank"
                rel="noopener"
              >www.last.fm/api/account/create →</a>
            </div>
          </section>

          <section class="step">
            <span class="step-num">2</span>
            <div class="step-body">
              <h4>{$t("study.music.lastfm_step2_title")}</h4>
              <p class="step-desc">{$t("study.music.lastfm_step2_desc")}</p>
              <label class="field">
                <span>API key</span>
                <input
                  type="text"
                  bind:value={apiKey}
                  placeholder={status.api_key_set ? "•••••••• (already set)" : ""}
                  autocomplete="off"
                />
              </label>
              <label class="field">
                <span>Shared secret</span>
                <input
                  type="password"
                  bind:value={apiSecret}
                  placeholder={status.api_secret_set ? "•••••••• (already set)" : ""}
                  autocomplete="off"
                />
              </label>
              <button
                type="button"
                class="cta-secondary"
                onclick={saveKeys}
                disabled={busy || (!apiKey.trim() && !apiSecret.trim())}
              >{$t("study.music.lastfm_save_keys")}</button>
            </div>
          </section>

          {#if status.api_key_set && status.api_secret_set}
            <section class="step">
              <span class="step-num">3</span>
              <div class="step-body">
                <h4>{$t("study.music.lastfm_step3_title")}</h4>
                <p class="step-desc">{$t("study.music.lastfm_step3_desc")}</p>
                {#if !pendingToken}
                  <button
                    type="button"
                    class="cta"
                    onclick={startAuth}
                    disabled={busy}
                  >{$t("study.music.lastfm_authorize")}</button>
                {:else}
                  <div class="pending">
                    <p>{$t("study.music.lastfm_authorize_pending")}</p>
                    {#if pendingAuthUrl}
                      <a class="ext-link" href={pendingAuthUrl} target="_blank" rel="noopener">
                        {$t("study.music.lastfm_open_auth")} →
                      </a>
                    {/if}
                    <button
                      type="button"
                      class="cta"
                      onclick={finalizeAuth}
                      disabled={busy}
                    >{$t("study.music.lastfm_finalize")}</button>
                  </div>
                {/if}
              </div>
            </section>
          {/if}
        {:else}
          <section class="connected">
            <div class="badge-ok">✓</div>
            <div>
              <h4>{$t("study.music.lastfm_connected_as", { username: status.username })}</h4>
              <p class="step-desc">{$t("study.music.lastfm_connected_hint")}</p>
            </div>
          </section>
          <label class="toggle">
            <input
              type="checkbox"
              checked={status.enabled}
              onchange={toggleEnabled}
              disabled={busy}
            />
            <span>{$t("study.music.lastfm_enable_scrobble")}</span>
          </label>
          <button
            type="button"
            class="link danger"
            onclick={disconnect}
            disabled={busy}
          >{$t("study.music.lastfm_disconnect")}</button>
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
    width: min(560px, 92vw);
    max-height: 86vh;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head {
    position: relative;
    padding: 18px 20px 8px;
  }
  .head h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 800;
  }
  .close {
    position: absolute;
    top: 12px;
    right: 14px;
    width: 28px;
    height: 28px;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.5);
    font-size: 18px;
    cursor: pointer;
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .hint {
    margin: 0;
    color: rgba(255, 255, 255, 0.65);
    font-size: 13px;
    line-height: 1.5;
  }
  .muted { color: rgba(255, 255, 255, 0.5); }
  .step {
    display: flex;
    gap: 12px;
    padding: 14px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
  }
  .step-num {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    background: var(--accent);
    color: var(--on-accent, white);
    border-radius: 50%;
    font-size: 12px;
    font-weight: 700;
  }
  .step-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 8px; }
  .step-body h4 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
  }
  .step-desc {
    margin: 0;
    color: rgba(255, 255, 255, 0.6);
    font-size: 12px;
    line-height: 1.5;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 4px 0;
  }
  .field span {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .field input {
    padding: 8px 10px;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    color: rgba(255, 255, 255, 0.95);
    font-family: ui-monospace, monospace;
    font-size: 12px;
    outline: none;
  }
  .field input:focus { border-color: var(--accent); }
  .ext-link {
    color: var(--accent);
    font-size: 12px;
    text-decoration: none;
  }
  .ext-link:hover { text-decoration: underline; }
  .cta {
    align-self: flex-start;
    padding: 8px 18px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }
  .cta:disabled { opacity: 0.5; cursor: default; }
  .cta-secondary {
    align-self: flex-start;
    padding: 7px 14px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .cta-secondary:hover { border-color: var(--accent); color: var(--accent); }
  .pending {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .connected {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px;
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 30%, transparent);
    border-radius: 10px;
  }
  .badge-ok {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    background: var(--accent);
    color: var(--on-accent, white);
    border-radius: 50%;
    font-size: 16px;
    font-weight: 700;
  }
  .connected h4 { margin: 0; font-size: 13px; font-weight: 700; }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.85);
  }
  .toggle input { width: 14px; height: 14px; accent-color: var(--accent); }
  .link {
    align-self: flex-start;
    background: transparent;
    border: 0;
    color: rgba(255, 255, 255, 0.5);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    padding: 0;
  }
  .link.danger:hover { color: rgb(248, 113, 113); }
</style>
