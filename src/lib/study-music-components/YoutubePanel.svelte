<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";

  type Settings = {
    browser: string;
    audio_format_preference: string;
    sapisid_set: boolean;
    cookie_set: boolean;
    visitor_data_set: boolean;
  };

  type Props = {
    open: boolean;
    onClose: () => void;
    onImported?: () => void;
  };

  let { open, onClose, onImported }: Props = $props();

  let settings = $state<Settings | null>(null);
  let importUrl = $state("");
  let busy = $state(false);
  let sapisidInput = $state("");
  let cookieInput = $state("");
  let syncProgress = $state<{ stage: string; current?: number; total?: number; name?: string } | null>(null);
  let unlistenFns: UnlistenFn[] = [];

  async function load() {
    try {
      settings = await pluginInvoke<Settings>("study", "study:music:youtube:get_settings");
    } catch {
      settings = null;
    }
  }

  $effect(() => {
    if (open) void load();
  });

  type Status = {
    file_exists: boolean;
    file_path: string;
    last_modified_secs: number | null;
    age_seconds: number | null;
    total_cookies: number;
    domains: string[];
  };

  function fmtAge(secs: number | null | undefined): string {
    if (secs == null) return "—";
    if (secs < 60) return `${secs}s atrás`;
    if (secs < 3600) return `${Math.floor(secs / 60)} min atrás`;
    if (secs < 86400) return `${Math.floor(secs / 3600)} h atrás`;
    return `${Math.floor(secs / 86400)} dias atrás`;
  }

  async function importFromExtension() {
    if (busy) return;
    busy = true;
    try {
      const status = await invoke<Status>("extension_cookies_status");

      if (!status.file_exists) {
        showToast(
          "error",
          $t("study.music.yt_ext_no_file", {
            path: status.file_path,
          }) as string,
        );
        return;
      }

      const ageMin = status.age_seconds != null
        ? Math.floor(status.age_seconds / 60)
        : null;
      const hasYoutube = status.domains.some(
        (d) => d.endsWith("youtube.com") || d.endsWith("google.com"),
      );

      if (!hasYoutube) {
        showToast(
          "error",
          $t("study.music.yt_ext_no_youtube", {
            domains: status.domains.slice(0, 3).join(", ") || "(nenhum)",
            age: fmtAge(status.age_seconds),
          }) as string,
        );
        return;
      }

      type Bundle = {
        domain: string;
        cookie_string: string;
        well_known: Record<string, string>;
        cookies: Array<{ name: string; value: string }>;
        fetched_at: number;
      };
      const bundle = await invoke<Bundle | null>("read_extension_cookies", {
        domain: "youtube.com",
      });
      if (!bundle) {
        showToast("error", $t("study.music.yt_ext_empty") as string);
        return;
      }
      const sapisid = bundle.well_known["__Secure-3PAPISID"]
        ?? bundle.well_known["SAPISID"]
        ?? "";
      if (!sapisid) {
        showToast(
          "error",
          $t("study.music.yt_ext_no_sapisid_detail", {
            count: bundle.cookies.length,
            age: fmtAge(status.age_seconds),
          }) as string,
        );
        return;
      }
      const res = await pluginInvoke<Settings>(
        "study",
        "study:music:youtube:set_settings",
        {
          sapisid,
          cookie_string: bundle.cookie_string,
        },
      );
      settings = res;
      showToast(
        "success",
        $t("study.music.yt_ext_imported_detail", {
          count: bundle.cookies.length,
          age: fmtAge(status.age_seconds),
        }) as string,
      );
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function saveAuth() {
    if (busy) return;
    busy = true;
    try {
      const args: Record<string, string> = {};
      if (sapisidInput.trim()) args.sapisid = sapisidInput.trim();
      if (cookieInput.trim()) args.cookie_string = cookieInput.trim();
      const res = await pluginInvoke<Settings>(
        "study",
        "study:music:youtube:set_settings",
        args,
      );
      settings = res;
      sapisidInput = "";
      cookieInput = "";
      showToast("success", $t("study.music.yt_auth_saved") as string);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function autoVisitorData() {
    if (busy) return;
    busy = true;
    try {
      await pluginInvoke("study", "study:music:youtube:auto_visitor_data");
      await load();
      showToast("success", $t("study.music.yt_visitor_ok") as string);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function disconnect() {
    if (busy || !window.confirm($t("study.music.yt_disconnect_confirm") as string)) return;
    busy = true;
    try {
      await pluginInvoke("study", "study:music:youtube:set_settings", {
        sapisid: "",
        cookie_string: "",
        visitor_data: "",
      });
      await load();
      showToast("success", $t("study.music.yt_disconnected") as string);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function importPlaylist() {
    if (!importUrl.trim() || busy) return;
    busy = true;
    try {
      const res = await pluginInvoke<{ title: string; imported: number }>(
        "study",
        "study:music:youtube:import_playlist",
        { url: importUrl.trim() },
      );
      showToast(
        "success",
        $t("study.music.yt_imported", {
          title: res.title || "playlist",
          count: res.imported,
        }) as string,
      );
      importUrl = "";
      onImported?.();
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function syncLibrary() {
    if (busy) return;
    busy = true;
    syncProgress = { stage: "starting" };
    try {
      const res = await pluginInvoke<{ playlists: number; tracks: number }>(
        "study",
        "study:music:youtube:sync_library",
      );
      showToast(
        "success",
        $t("study.music.yt_sync_done", {
          playlists: res.playlists,
          tracks: res.tracks,
        }) as string,
      );
      onImported?.();
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
      setTimeout(() => (syncProgress = null), 2000);
    }
  }

  onMount(async () => {
    try {
      unlistenFns.push(
        await listen<{ stage: string; current?: number; total?: number; name?: string }>(
          "study:music:youtube:sync_progress",
          (e) => {
            syncProgress = e.payload;
          },
        ),
      );
    } catch {
      /* ignore */
    }
  });

  onDestroy(() => {
    for (const fn of unlistenFns) fn();
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }

  const isAuthed = $derived((settings?.sapisid_set ?? false) || (settings?.cookie_set ?? false));
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
          <h3>{$t("study.music.yt_title")}</h3>
          {#if isAuthed}
            <span class="state on">✓ {$t("study.music.yt_state_authed")}</span>
          {:else}
            <span class="state off">{$t("study.music.yt_state_anon")}</span>
          {/if}
        </div>
        <button type="button" class="close" onclick={onClose} aria-label={$t("study.common.close") as string}>×</button>
      </header>
      <div class="body">
        <p class="hint">{$t("study.music.yt_intro")}</p>

        <section class="step">
          <span class="step-num">1</span>
          <div class="step-body">
            <h4>{$t("study.music.yt_auth_title")}</h4>
            <p class="step-desc">{$t("study.music.yt_auth_desc")}</p>
            <div class="ext-shortcut">
              <button type="button" class="cta-secondary" onclick={importFromExtension} disabled={busy}>
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
                {$t("study.music.yt_ext_import")}
              </button>
              <span class="ext-hint">{$t("study.music.yt_ext_hint")}</span>
            </div>
            <div class="manual-divider"><span>{$t("study.music.yt_or_manual")}</span></div>
            <details class="howto">
              <summary>{$t("study.music.yt_auth_how")}</summary>
              <ol>
                <li>{$t("study.music.yt_auth_step1")}</li>
                <li>{$t("study.music.yt_auth_step2")}</li>
                <li>{$t("study.music.yt_auth_step3")}</li>
                <li>{$t("study.music.yt_auth_step4")}</li>
              </ol>
            </details>
            <label class="field">
              <span>__Secure-3PAPISID</span>
              <input
                type="password"
                bind:value={sapisidInput}
                placeholder={settings?.sapisid_set ? "•••••• (já configurado)" : "AbcDef123..."}
                autocomplete="off"
              />
            </label>
            <label class="field">
              <span>{$t("study.music.yt_cookie_full")}</span>
              <textarea
                bind:value={cookieInput}
                placeholder={settings?.cookie_set ? "(já configurado)" : "VISITOR_INFO1_LIVE=...; SID=...; HSID=...; ..."}
                rows="2"
              ></textarea>
              <small>{$t("study.music.yt_cookie_hint")}</small>
            </label>
            <div class="row-actions">
              <button type="button" class="cta-secondary" onclick={saveAuth} disabled={busy || (!sapisidInput.trim() && !cookieInput.trim())}>
                {$t("study.music.yt_save_auth")}
              </button>
              {#if isAuthed}
                <button type="button" class="link danger" onclick={disconnect} disabled={busy}>
                  {$t("study.music.yt_disconnect")}
                </button>
              {/if}
            </div>
          </div>
        </section>

        <section class="step">
          <span class="step-num">2</span>
          <div class="step-body">
            <h4>{$t("study.music.yt_step2_title")}</h4>
            <p class="step-desc">{$t("study.music.yt_step2_desc")}</p>
            <div class="import-row">
              <input
                type="text"
                bind:value={importUrl}
                placeholder="https://music.youtube.com/playlist?list=..."
                onkeydown={(e) => { if (e.key === "Enter") importPlaylist(); }}
                disabled={busy}
              />
              <button type="button" class="cta" onclick={importPlaylist} disabled={busy || !importUrl.trim()}>
                {$t("study.music.yt_import")}
              </button>
            </div>
            <ul class="example-list">
              <li><code>music.youtube.com/playlist?list=LM</code> — <span>{$t("study.music.yt_lm_label")}</span></li>
              <li><code>music.youtube.com/playlist?list=PLxxxx</code> — <span>{$t("study.music.yt_playlist_label")}</span></li>
            </ul>
          </div>
        </section>

        {#if isAuthed}
          <section class="step accent">
            <span class="step-num">3</span>
            <div class="step-body">
              <h4>{$t("study.music.yt_sync_title")}</h4>
              <p class="step-desc">{$t("study.music.yt_sync_desc")}</p>
              <button type="button" class="cta wide" onclick={syncLibrary} disabled={busy}>
                {busy ? $t("study.music.yt_syncing") : $t("study.music.yt_sync_now")}
              </button>
              {#if syncProgress}
                <div class="progress-line">
                  {#if syncProgress.stage === "starting"}
                    {$t("study.music.yt_sync_starting")}
                  {:else if syncProgress.stage === "liked"}
                    {$t("study.music.yt_sync_liked")}
                  {:else if syncProgress.stage === "playlists"}
                    {$t("study.music.yt_sync_playlists", { total: syncProgress.total ?? 0 })}
                  {:else if syncProgress.stage === "playlist_fetch"}
                    {syncProgress.current}/{syncProgress.total}: {syncProgress.name}
                  {/if}
                </div>
              {/if}
            </div>
          </section>
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
    width: min(620px, 92vw);
    max-height: 88vh;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head { position: relative; display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; padding: 18px 20px 8px; }
  .head-text { display: flex; flex-direction: column; gap: 4px; }
  .head h3 { margin: 0; font-size: 16px; font-weight: 800; }
  .state { display: inline-flex; align-items: center; gap: 6px; padding: 2px 10px; border-radius: 999px; font-size: 11px; font-weight: 600; width: fit-content; }
  .state.on { background: rgba(34, 197, 94, 0.15); color: rgb(74, 222, 128); }
  .state.off { background: rgba(255, 255, 255, 0.06); color: rgba(255, 255, 255, 0.45); }
  .close { width: 28px; height: 28px; background: transparent; border: 0; border-radius: 50%; color: rgba(255, 255, 255, 0.5); font-size: 18px; cursor: pointer; flex-shrink: 0; }
  .body { padding: 8px 20px 20px; overflow-y: auto; display: flex; flex-direction: column; gap: 16px; }
  .hint { margin: 0; color: rgba(255, 255, 255, 0.65); font-size: 13px; line-height: 1.5; }
  .step { display: flex; gap: 12px; padding: 14px; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 10px; }
  .step.accent { background: color-mix(in oklab, var(--accent) 10%, rgba(255,255,255,0.03)); border-color: color-mix(in oklab, var(--accent) 30%, transparent); }
  .step-num { flex-shrink: 0; width: 24px; height: 24px; display: grid; place-items: center; background: var(--accent); color: var(--on-accent, white); border-radius: 50%; font-size: 12px; font-weight: 700; }
  .step-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 8px; }
  .step-body h4 { margin: 0; font-size: 13px; font-weight: 700; }
  .step-desc { margin: 0; color: rgba(255, 255, 255, 0.6); font-size: 12px; line-height: 1.5; }
  .howto summary { cursor: pointer; font-size: 11px; color: rgba(255, 255, 255, 0.55); margin-bottom: 4px; }
  .howto ol { margin: 4px 0 8px 16px; padding: 0; font-size: 11px; color: rgba(255, 255, 255, 0.7); line-height: 1.5; }
  .howto li { margin-bottom: 2px; }
  .field { display: flex; flex-direction: column; gap: 4px; margin: 4px 0; }
  .field span { font-size: 11px; color: rgba(255, 255, 255, 0.5); text-transform: uppercase; letter-spacing: 0.06em; }
  .field input, .field textarea { padding: 8px 10px; background: rgba(0, 0, 0, 0.3); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 6px; color: rgba(255, 255, 255, 0.95); font-family: ui-monospace, monospace; font-size: 11px; outline: none; resize: vertical; }
  .field input:focus, .field textarea:focus { border-color: var(--accent); }
  .field small { font-size: 11px; color: rgba(255, 255, 255, 0.4); }
  .row-actions { display: flex; align-items: center; gap: 8px; }
  .cta-secondary { padding: 7px 14px; border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 999px; background: transparent; color: rgba(255, 255, 255, 0.95); font-family: inherit; font-size: 12px; cursor: pointer; }
  .cta-secondary:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .cta-secondary:disabled { opacity: 0.4; cursor: default; }
  .cta { padding: 8px 18px; border: 0; border-radius: 999px; background: var(--accent); color: var(--on-accent, white); font-family: inherit; font-size: 13px; font-weight: 700; cursor: pointer; }
  .cta.wide { width: 100%; padding: 10px 18px; }
  .cta:disabled { opacity: 0.5; cursor: default; }
  .link { background: transparent; border: 0; color: rgba(255, 255, 255, 0.5); font-family: inherit; font-size: 11px; cursor: pointer; padding: 0; }
  .link.danger:hover { color: rgb(248, 113, 113); }
  .import-row { display: flex; gap: 8px; }
  .import-row input { flex: 1; padding: 9px 12px; background: rgba(0, 0, 0, 0.3); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; color: rgba(255, 255, 255, 0.95); font-family: ui-monospace, monospace; font-size: 12px; outline: none; }
  .import-row input:focus { border-color: var(--accent); }
  .example-list { list-style: none; margin: 4px 0 0; padding: 0; display: flex; flex-direction: column; gap: 4px; font-size: 11px; }
  .example-list code { background: rgba(0, 0, 0, 0.4); padding: 1px 6px; border-radius: 4px; color: rgba(255, 255, 255, 0.75); font-size: 10px; margin-right: 6px; }
  .example-list span { color: rgba(255, 255, 255, 0.55); }
  .progress-line { margin-top: 4px; padding: 6px 10px; background: rgba(0, 0, 0, 0.3); border-radius: 6px; font-size: 11px; color: rgba(255, 255, 255, 0.7); font-family: ui-monospace, monospace; }
  .ext-shortcut { display: flex; align-items: center; gap: 10px; padding: 10px 12px; background: color-mix(in oklab, var(--accent) 8%, transparent); border: 1px solid color-mix(in oklab, var(--accent) 30%, transparent); border-radius: 8px; }
  .ext-shortcut .cta-secondary { display: inline-flex; align-items: center; gap: 6px; }
  .ext-hint { font-size: 11px; color: rgba(255, 255, 255, 0.65); flex: 1; }
  .manual-divider { display: flex; align-items: center; margin: 4px 0; color: rgba(255, 255, 255, 0.35); font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; }
  .manual-divider::before, .manual-divider::after { content: ""; flex: 1; height: 1px; background: rgba(255, 255, 255, 0.08); }
  .manual-divider span { padding: 0 10px; }
</style>
