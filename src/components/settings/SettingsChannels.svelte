<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t, locale as i18nLocale } from "$lib/i18n";
  import { get } from "svelte/store";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import timeAgo from "$lib/time-ago";

  type Channel = {
    id: string;
    url: string;
    title: string;
    added_at_ms: number;
    last_checked_ms: number | null;
    enabled: boolean;
    auto_download: boolean;
    interval_minutes: number;
  };

  let channels = $state<Channel[]>([]);
  let newUrl = $state("");
  let adding = $state(false);

  async function syncTray() {
    try {
      await invoke("sync_channels_tray", {
        header: $t("settings.channels.tray_header"),
        channels: channels.map((c) => [c.id, c.title]),
      });
    } catch {
      // tray sync is best-effort
    }
  }

  async function load() {
    try {
      channels = await invoke<Channel[]>("channels_list");
      await syncTray();
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : $t("common.error"));
    }
  }

  onMount(load);

  async function addChannel() {
    const url = newUrl.trim();
    if (!url || adding) return;
    adding = true;
    try {
      await invoke("channel_add", { url });
      newUrl = "";
      await load();
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : $t("common.error"));
    } finally {
      adding = false;
    }
  }

  async function removeChannel(id: string) {
    if (!confirm($t("settings.channels.remove_confirm"))) return;
    try {
      await invoke("channel_remove", { id });
      await load();
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : $t("common.error"));
    }
  }

  async function patch(
    id: string,
    fields: { enabled?: boolean; autoDownload?: boolean; intervalMinutes?: number },
  ) {
    try {
      await invoke("channel_update", { id, ...fields });
      await load();
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : $t("common.error"));
    }
  }

  async function checkNow(id: string) {
    try {
      const n = await invoke<number>("channel_check_now", { id });
      showToast(
        "success",
        $t("settings.channels.checked", { count: n }) as string,
      );
      await load();
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : $t("common.error"));
    }
  }

  function lastChecked(ms: number | null): string {
    if (!ms) return $t("settings.channels.never") as string;
    const loc = get(i18nLocale) || "en";
    return timeAgo(ms, loc.startsWith("pt") ? "pt" : "en");
  }

  function onIntervalChange(id: string, e: Event) {
    const v = parseInt((e.target as HTMLInputElement).value, 10);
    if (!Number.isNaN(v)) patch(id, { intervalMinutes: v });
  }
</script>

<section class="section">
  <h5 class="section-title">{$t('settings.channels.title')}</h5>

  <div class="card">
    <div class="add-row">
      <input
        type="text"
        class="input-text"
        placeholder={$t('settings.channels.add_placeholder')}
        bind:value={newUrl}
        onkeydown={(e) => { if (e.key === "Enter") addChannel(); }}
      />
      <button class="add-btn" disabled={!newUrl.trim() || adding} onclick={addChannel}>
        {$t('settings.channels.add')}
      </button>
    </div>
    <span class="setting-path add-hint">{$t('settings.channels.add_hint')}</span>
  </div>

  {#if channels.length === 0}
    <div class="card empty">
      <span class="empty-title">{$t('settings.channels.empty_title')}</span>
      <span class="setting-path">{$t('settings.channels.empty_hint')}</span>
    </div>
  {:else}
    <div class="card">
      {#each channels as ch, i (ch.id)}
        {#if i > 0}<div class="divider"></div>{/if}
        <div class="channel">
          <div class="channel-head">
            <div class="channel-id">
              <span class="channel-title" title={ch.url}>{ch.title}</span>
              <span class="setting-path">{$t('settings.channels.last_checked', { when: lastChecked(ch.last_checked_ms) })}</span>
            </div>
            <div class="channel-actions">
              <button class="link-btn" onclick={() => checkNow(ch.id)}>{$t('settings.channels.check_now')}</button>
              <button class="link-btn danger" onclick={() => removeChannel(ch.id)}>{$t('settings.channels.remove')}</button>
            </div>
          </div>
          <div class="channel-controls">
            <label class="ctrl">
              <button
                class="toggle"
                class:on={ch.enabled}
                onclick={() => patch(ch.id, { enabled: !ch.enabled })}
                role="switch"
                aria-checked={ch.enabled}
                aria-label={$t('settings.channels.enabled') as string}
              ><span class="toggle-knob"></span></button>
              <span>{$t('settings.channels.enabled')}</span>
            </label>
            <label class="ctrl">
              <button
                class="toggle"
                class:on={ch.auto_download}
                onclick={() => patch(ch.id, { autoDownload: !ch.auto_download })}
                role="switch"
                aria-checked={ch.auto_download}
                aria-label={$t('settings.channels.auto_download') as string}
              ><span class="toggle-knob"></span></button>
              <span>{$t('settings.channels.auto_download')}</span>
            </label>
            <label class="ctrl interval">
              <span>{$t('settings.channels.interval')}</span>
              <input
                type="number"
                class="input-number"
                min="5"
                max="1440"
                value={ch.interval_minutes}
                onchange={(e) => onIntervalChange(ch.id, e)}
              />
            </label>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .add-row {
    display: flex;
    gap: 8px;
    width: 100%;
  }

  .add-row .input-text {
    flex: 1;
    min-width: 0;
  }

  .add-btn {
    flex-shrink: 0;
    padding: 0 16px;
    border: none;
    border-radius: var(--border-radius);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
    cursor: pointer;
  }

  .add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .add-hint {
    display: block;
    margin-top: 8px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    gap: 4px;
    text-align: center;
    padding: 24px 16px;
  }

  .empty-title {
    font-weight: 600;
  }

  .channel {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 4px 0;
  }

  .channel-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .channel-id {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .channel-title {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .channel-actions {
    display: flex;
    gap: 12px;
    flex-shrink: 0;
  }

  .link-btn {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 13px;
    padding: 0;
  }

  .link-btn.danger {
    color: var(--error);
  }

  .channel-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
  }

  .ctrl {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .ctrl.interval .input-number {
    width: 72px;
  }
</style>
