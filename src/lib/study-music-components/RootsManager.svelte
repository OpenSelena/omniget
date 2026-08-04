<script lang="ts">
  import { onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { t } from "$lib/i18n";

  type RootEntry = {
    path: string;
    enabled: boolean;
    added_at: number;
    exists: boolean;
  };

  type Props = {
    open: boolean;
    onClose: () => void;
    onChanged?: () => void;
  };

  let { open = $bindable(), onClose, onChanged }: Props = $props();

  let roots = $state<RootEntry[]>([]);
  let loading = $state(false);
  let busy = $state(false);
  let diagnosticsOpen = $state(false);
  let diagnostics = $state<unknown>(null);

  async function load() {
    loading = true;
    try {
      const res = await pluginInvoke<{ roots: RootEntry[] }>(
        "study",
        "study:music:roots:list",
      );
      roots = res.roots ?? [];
    } finally {
      loading = false;
    }
  }

  async function addRoot() {
    if (busy) return;
    busy = true;
    try {
      const dialog = await import("@tauri-apps/plugin-dialog");
      const picked = await dialog.open({ directory: true, multiple: false });
      if (typeof picked !== "string" || !picked) return;
      await pluginInvoke("study", "study:music:roots:add", { path: picked });
      showToast("success", $t("study.music.root_added", { path: picked }) as string);
      await load();
      onChanged?.();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    } finally {
      busy = false;
    }
  }

  async function toggleRoot(r: RootEntry) {
    try {
      await pluginInvoke("study", "study:music:roots:toggle", {
        path: r.path,
        enabled: !r.enabled,
      });
      await load();
      onChanged?.();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    }
  }

  async function clearCovers() {
    if (busy) return;
    if (
      typeof window !== "undefined" &&
      !window.confirm($t("study.music.clear_covers_confirm") as string)
    ) {
      return;
    }
    busy = true;
    try {
      const res = await pluginInvoke<{ files_removed: number; tracks_cleared: number }>(
        "study",
        "study:music:covers:clear",
      );
      showToast(
        "success",
        $t("study.music.clear_covers_done", {
          files: res.files_removed,
          tracks: res.tracks_cleared,
        }) as string,
      );
      const scanRes = await pluginInvoke<{
        added: number;
        updated: number;
        removed: number;
      }>("study", "study:music:scan");
      showToast(
        "success",
        $t("study.music.scan_done", {
          added: scanRes.added,
          updated: scanRes.updated,
          removed: scanRes.removed,
        }) as string,
      );
      onChanged?.();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    } finally {
      busy = false;
    }
  }

  async function runDiagnostics() {
    if (busy) return;
    busy = true;
    try {
      const res = await pluginInvoke(
        "study",
        "study:music:debug:cover-status",
      );
      diagnostics = res;
      diagnosticsOpen = true;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    } finally {
      busy = false;
    }
  }

  async function copyDiagnostics() {
    if (!diagnostics) return;
    try {
      await navigator.clipboard.writeText(JSON.stringify(diagnostics, null, 2));
      showToast("success", $t("study.common.copied") as string);
    } catch {
      /* ignore */
    }
  }

  async function removeRoot(r: RootEntry) {
    if (busy) return;
    busy = true;
    try {
      await pluginInvoke("study", "study:music:roots:remove", { path: r.path });
      showToast(
        "success",
        $t("study.music.root_removed", { path: r.path }) as string,
      );
      await load();
      onChanged?.();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    } finally {
      busy = false;
    }
  }

  function compactPath(path: string, max = 60): string {
    if (path.length <= max) return path;
    return `…${path.slice(path.length - max + 1)}`;
  }

  $effect(() => {
    if (open) void load();
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }
</script>

{#if diagnosticsOpen && diagnostics}
  <div
    class="overlay diag-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) diagnosticsOpen = false;
    }}
  >
    <div class="dialog diag-dialog" role="dialog" aria-modal="true" tabindex="-1">
      <header class="head">
        <h3>{$t("study.music.diagnostics_title")}</h3>
        <button
          type="button"
          class="close"
          onclick={() => (diagnosticsOpen = false)}
          aria-label={$t("study.common.close") as string}
        >×</button>
      </header>
      <div class="diag-body">
        <pre>{JSON.stringify(diagnostics, null, 2)}</pre>
      </div>
      <footer class="foot">
        <button
          type="button"
          class="reset-btn"
          onclick={copyDiagnostics}
        >{$t("study.common.copy")}</button>
        <button
          type="button"
          class="done-btn"
          onclick={() => (diagnosticsOpen = false)}
        >{$t("study.common.close")}</button>
      </footer>
    </div>
  </div>
{/if}

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
        <h3>{$t("study.music.roots_title")}</h3>
        <button type="button" class="close" onclick={onClose} aria-label={$t("study.common.close") as string}>×</button>
      </header>
      <p class="hint">{$t("study.music.roots_hint")}</p>

      <div class="content">
        {#if loading}
          <p class="muted">{$t("study.common.loading")}</p>
        {:else if roots.length === 0}
          <div class="empty">
            <p>{$t("study.music.roots_empty")}</p>
          </div>
        {:else}
          <ul class="list">
            {#each roots as r (r.path)}
              <li class="row" class:disabled={!r.enabled} class:missing={!r.exists}>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={r.enabled}
                    onchange={() => toggleRoot(r)}
                  />
                </label>
                <div class="info">
                  <span class="path" title={r.path}>{compactPath(r.path)}</span>
                  {#if !r.exists}
                    <span class="badge missing-badge">{$t("study.music.root_missing")}</span>
                  {/if}
                </div>
                <button
                  type="button"
                  class="remove"
                  onclick={() => removeRoot(r)}
                  disabled={busy}
                  aria-label={$t("study.music.root_remove") as string}
                  title={$t("study.music.root_remove") as string}
                >×</button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <footer class="foot">
        <button
          type="button"
          class="add-btn"
          onclick={addRoot}
          disabled={busy}
        >+ {$t("study.music.add_root")}</button>
        <div class="tools-grid">
          <button
            type="button"
            class="reset-btn"
            onclick={clearCovers}
            disabled={busy}
            title={$t("study.music.clear_covers_hint") as string}
          >{$t("study.music.clear_covers")}</button>
          <button
            type="button"
            class="reset-btn"
            onclick={runDiagnostics}
            disabled={busy}
            title={$t("study.music.diagnostics_hint") as string}
          >{$t("study.music.diagnostics")}</button>
          <button
            type="button"
            class="reset-btn"
            onclick={() => {
              onClose();
              musicUI.openLastFm();
            }}
            disabled={busy}
            title={$t("study.music.lastfm_title") as string}
          >Last.fm</button>
          <button
            type="button"
            class="reset-btn"
            onclick={() => {
              onClose();
              musicUI.openTheme();
            }}
            disabled={busy}
            title={$t("study.music.music_theme_title") as string}
          >{$t("study.music.music_theme_title")}</button>
          <button
            type="button"
            class="reset-btn"
            onclick={() => {
              onClose();
              musicUI.openQuality();
            }}
            disabled={busy}
            title={$t("study.music.quality_title") as string}
          >{$t("study.music.quality_short")}</button>
          <button
            type="button"
            class="reset-btn"
            onclick={() => {
              onClose();
              musicUI.openSources();
            }}
            disabled={busy}
            title={$t("study.music.sources_title") as string}
          >{$t("study.music.sources_short")}</button>
          <button
            type="button"
            class="reset-btn"
            onclick={() => {
              onClose();
              musicUI.openQobuz();
            }}
            disabled={busy}
            title={$t("study.music.qobuz_title") as string}
          >Qobuz</button>
          <button type="button" class="done-btn" onclick={onClose}>
            {$t("study.common.close")}
          </button>
        </div>
      </footer>
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
    -webkit-backdrop-filter: blur(4px);
  }
  .dialog {
    background: rgb(20, 20, 20);
    color: rgba(255, 255, 255, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    width: min(540px, 92vw);
    max-height: 80vh;
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
    letter-spacing: -0.01em;
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
  .close:hover { color: white; background: rgba(255, 255, 255, 0.08); }
  .hint {
    margin: 0;
    padding: 0 20px 12px;
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;
    line-height: 1.5;
  }
  .content {
    flex: 1;
    overflow-y: auto;
    padding: 4px 12px 12px;
  }
  .empty {
    padding: 32px 12px;
    text-align: center;
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
  }
  .muted { color: rgba(255, 255, 255, 0.5); font-size: 13px; padding: 8px; }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 8px;
  }
  .row.missing {
    background: rgba(217, 119, 6, 0.08);
    border-color: rgba(217, 119, 6, 0.25);
  }
  .row.disabled .path {
    opacity: 0.5;
    text-decoration: line-through;
    text-decoration-color: rgba(255, 255, 255, 0.25);
  }
  .toggle input {
    width: 16px;
    height: 16px;
    cursor: pointer;
    accent-color: var(--accent);
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .path {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: rgba(255, 255, 255, 0.95);
  }
  .badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 2px 6px;
    border-radius: 4px;
    align-self: flex-start;
  }
  .missing-badge {
    background: rgba(217, 119, 6, 0.18);
    color: rgb(251, 191, 36);
  }
  .remove {
    width: 26px;
    height: 26px;
    padding: 0;
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.5);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    transition: color 120ms ease, border-color 120ms ease, background 120ms ease;
  }
  .remove:hover:not(:disabled) {
    color: rgb(248, 113, 113);
    border-color: rgb(248, 113, 113);
    background: rgba(248, 113, 113, 0.1);
  }
  .remove:disabled { opacity: 0.5; cursor: default; }
  .foot {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 20px 16px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.02);
  }
  .add-btn {
    width: 100%;
    padding: 10px 16px;
    border: 1px dashed rgba(255, 255, 255, 0.25);
    border-radius: 10px;
    background: transparent;
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 120ms ease, color 120ms ease, background 120ms ease;
  }
  .add-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .add-btn:disabled { opacity: 0.5; cursor: default; }
  .tools-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 6px;
  }
  .reset-btn {
    padding: 8px 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    background: transparent;
    color: rgba(255, 255, 255, 0.7);
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color 120ms ease, border-color 120ms ease, background 120ms ease;
  }
  .reset-btn:hover:not(:disabled) {
    color: rgba(255, 255, 255, 0.95);
    border-color: rgba(255, 255, 255, 0.3);
    background: rgba(255, 255, 255, 0.05);
  }
  .reset-btn:disabled { opacity: 0.4; cursor: default; }
  .diag-overlay { z-index: 340; }
  .diag-dialog {
    width: min(720px, 94vw);
    max-height: 86vh;
  }
  .diag-body {
    flex: 1;
    overflow: auto;
    padding: 0 20px 16px;
  }
  .diag-body pre {
    margin: 0;
    padding: 12px;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    font-family: ui-monospace, monospace;
    font-size: 11px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.85);
    white-space: pre-wrap;
    word-break: break-all;
  }
  .done-btn {
    padding: 8px 12px;
    border: 0;
    border-radius: 8px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
    text-align: center;
  }
  .done-btn:hover {
    filter: brightness(1.1);
  }
</style>
