<script lang="ts">
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";
  import AboutLink from "./AboutLink.svelte";

  type Status = {
    enabled: boolean;
    app_id: string;
    large_image_key: string;
    connected: boolean;
  };

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  let status = $state<Status | null>(null);
  let appIdInput = $state("");
  let imageKeyInput = $state("");
  let busy = $state(false);
  let testResult = $state<"ok" | "fail" | null>(null);

  async function load() {
    try {
      status = await pluginInvoke<Status>("study", "study:music:rpc:get_settings");
      appIdInput = status.app_id;
      imageKeyInput = status.large_image_key;
    } catch {
      status = null;
    }
  }

  $effect(() => {
    if (open) {
      testResult = null;
      void load();
    }
  });

  async function toggleEnabled() {
    if (!status || busy) return;
    busy = true;
    try {
      const res = await pluginInvoke<Status>(
        "study",
        "study:music:rpc:set_settings",
        { enabled: !status.enabled },
      );
      status = res;
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function saveFields() {
    if (!status || busy) return;
    busy = true;
    try {
      const res = await pluginInvoke<Status>(
        "study",
        "study:music:rpc:set_settings",
        {
          app_id: appIdInput.trim(),
          large_image_key: imageKeyInput.trim(),
        },
      );
      status = res;
      appIdInput = res.app_id;
      imageKeyInput = res.large_image_key;
      showToast("success", $t("study.music.rpc_saved") as string);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function testConnection() {
    if (busy) return;
    busy = true;
    testResult = null;
    try {
      const res = await pluginInvoke<{ ok: boolean }>(
        "study",
        "study:music:rpc:test",
      );
      testResult = res.ok ? "ok" : "fail";
      if (!res.ok) {
        showToast("error", $t("study.music.rpc_test_fail") as string);
      } else {
        showToast("success", $t("study.music.rpc_test_ok") as string);
      }
      await load();
    } catch (e) {
      testResult = "fail";
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
        <div class="head-text">
          <h3>{$t("study.music.rpc_title")}</h3>
          {#if status}
            <span class="state" class:on={status.connected} class:off={!status.connected}>
              {status.connected
                ? $t("study.music.rpc_connected")
                : $t("study.music.rpc_disconnected")}
            </span>
          {/if}
        </div>
        <button type="button" class="close" onclick={onClose} aria-label={$t("study.common.close") as string}>×</button>
      </header>
      <div class="body">
        <p class="hint">{$t("study.music.rpc_intro")}</p>

        {#if status === null}
          <p class="muted">{$t("study.common.loading")}</p>
        {:else}
          <label class="toggle">
            <input
              type="checkbox"
              checked={status.enabled}
              onchange={toggleEnabled}
              disabled={busy}
            />
            <span>{$t("study.music.rpc_enable")}</span>
          </label>

          <details class="advanced">
            <summary>{$t("study.music.rpc_advanced")}</summary>
            <div class="adv-body">
              <label class="field">
                <span>Application ID</span>
                <input
                  type="text"
                  bind:value={appIdInput}
                  placeholder="1502867748353478656"
                  autocomplete="off"
                />
                <small>{$t("study.music.rpc_appid_hint")}</small>
              </label>
              <label class="field">
                <span>{$t("study.music.rpc_large_image")}</span>
                <input
                  type="text"
                  bind:value={imageKeyInput}
                  placeholder="omniget_music"
                  autocomplete="off"
                />
                <small>{$t("study.music.rpc_image_hint")}</small>
              </label>
              <button
                type="button"
                class="cta-secondary"
                onclick={saveFields}
                disabled={busy}
              >{$t("study.music.rpc_save")}</button>
            </div>
          </details>

          <div class="actions">
            <button type="button" class="cta" onclick={testConnection} disabled={busy || !status.enabled}>
              {$t("study.music.rpc_test")}
            </button>
            {#if testResult === "ok"}
              <span class="ok-badge">✓ {$t("study.music.rpc_test_ok")}</span>
            {:else if testResult === "fail"}
              <span class="fail-badge">✗ {$t("study.music.rpc_test_fail")}</span>
            {/if}
          </div>

          <p class="footnote">{$t("study.music.rpc_discord_running")}</p>
          <AboutLink variant="card" />
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
    width: min(540px, 92vw);
    max-height: 86vh;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head {
    position: relative;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 18px 20px 8px;
  }
  .head-text { display: flex; flex-direction: column; gap: 4px; }
  .head h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 800;
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 10px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    width: fit-content;
  }
  .state::before {
    content: "";
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .state.on { background: rgba(34, 197, 94, 0.15); color: rgb(74, 222, 128); }
  .state.off { background: rgba(255, 255, 255, 0.06); color: rgba(255, 255, 255, 0.45); }
  .close {
    width: 28px;
    height: 28px;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.5);
    font-size: 18px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .body {
    padding: 8px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .hint {
    margin: 0;
    color: rgba(255, 255, 255, 0.65);
    font-size: 13px;
    line-height: 1.5;
  }
  .muted { color: rgba(255, 255, 255, 0.5); }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.85);
    padding: 10px 14px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
  }
  .toggle input { width: 14px; height: 14px; accent-color: var(--accent); }
  .advanced {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    padding: 10px 14px;
  }
  .advanced summary {
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.7);
    user-select: none;
  }
  .adv-body {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field span {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .field small {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    line-height: 1.4;
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
  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .cta {
    padding: 8px 18px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }
  .cta:disabled { opacity: 0.5; cursor: default; }
  .ok-badge {
    color: rgb(74, 222, 128);
    font-size: 12px;
    font-weight: 600;
  }
  .fail-badge {
    color: rgb(248, 113, 113);
    font-size: 12px;
    font-weight: 600;
  }
  .footnote {
    margin: 8px 0 0;
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    font-style: italic;
  }
</style>
