<script lang="ts">
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { t } from "$lib/i18n";

  type SourceEntry = { slug: string; available: boolean };

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  let activeOrder = $state<SourceEntry[]>([]);
  let allSources = $state<SourceEntry[]>([]);
  let loading = $state(false);
  let saving = $state(false);

  const DEFAULT_ORDER = ["youtube_music", "soundcloud"];

  const SOURCE_LABEL: Record<string, string> = {
    "youtube_music": "YouTube Music",
    "soundcloud": "SoundCloud",
    "qobuz": "Qobuz",
    "tidal": "Tidal",
    "deezer": "Deezer",
  };

  async function load() {
    loading = true;
    try {
      const res = await pluginInvoke<{
        active_order: SourceEntry[];
        all: SourceEntry[];
      }>("study", "study:music:resolver:list_sources");
      activeOrder = res.active_order ?? [];
      allSources = res.all ?? [];
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (open) void load();
  });

  async function commit(newOrder: SourceEntry[]) {
    if (saving) return;
    const previous = activeOrder;
    activeOrder = newOrder;
    saving = true;
    try {
      await pluginInvoke("study", "study:music:resolver:set_order", {
        order: newOrder.map((s) => s.slug),
      });
    } catch (e) {
      activeOrder = previous;
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      saving = false;
    }
  }

  function moveUp(idx: number) {
    if (idx <= 0) return;
    const next = activeOrder.slice();
    [next[idx - 1], next[idx]] = [next[idx], next[idx - 1]];
    void commit(next);
  }

  function moveDown(idx: number) {
    if (idx >= activeOrder.length - 1) return;
    const next = activeOrder.slice();
    [next[idx], next[idx + 1]] = [next[idx + 1], next[idx]];
    void commit(next);
  }

  function resetDefault() {
    if (saving) return;
    const lookup = new Map(allSources.map((s) => [s.slug, s]));
    const next: SourceEntry[] = [];
    for (const slug of DEFAULT_ORDER) {
      const entry = lookup.get(slug);
      if (entry) next.push(entry);
    }
    void commit(next);
  }

  function statusLabel(slug: string, available: boolean): string {
    if (available) return $t("study.music.source_status_connected") as string;
    if (slug === "qobuz") return $t("study.music.source_status_not_connected") as string;
    return $t("study.music.source_status_unavailable") as string;
  }

  function openAuth(slug: string) {
    if (slug === "qobuz") {
      onClose();
      musicUI.openQobuz();
    } else if (slug === "youtube_music") {
      onClose();
      musicUI.openYoutube();
    }
  }

  function canConnect(slug: string): boolean {
    return slug === "qobuz" || slug === "youtube_music";
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }

  const inactiveSources = $derived(
    allSources.filter((s) => !activeOrder.some((a) => a.slug === s.slug)),
  );
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
          <h3>{$t("study.music.sources_title")}</h3>
          <span class="state">{$t("study.music.sources_subtitle")}</span>
        </div>
        <button type="button" class="close" onclick={onClose} aria-label={$t("study.common.close") as string}>×</button>
      </header>
      <div class="body">
        <p class="hint">{$t("study.music.sources_intro")}</p>

        {#if loading}
          <div class="loading">{$t("study.common.loading")}</div>
        {:else}
          <ol class="list">
            {#each activeOrder as src, idx (src.slug)}
              <li class="row" class:dim={!src.available}>
                <span class="rank">{idx + 1}</span>
                <span class="name">{SOURCE_LABEL[src.slug] ?? src.slug}</span>
                <span class="status" class:on={src.available} class:off={!src.available}>
                  {statusLabel(src.slug, src.available)}
                </span>
                {#if !src.available && canConnect(src.slug)}
                  <button type="button" class="link" onclick={() => openAuth(src.slug)}>
                    {$t("study.music.source_connect")} →
                  </button>
                {/if}
                <span class="arrows">
                  <button
                    type="button"
                    class="arrow"
                    disabled={saving || idx === 0}
                    aria-label={$t("study.music.source_move_up") as string}
                    onclick={() => moveUp(idx)}
                  >▲</button>
                  <button
                    type="button"
                    class="arrow"
                    disabled={saving || idx === activeOrder.length - 1}
                    aria-label={$t("study.music.source_move_down") as string}
                    onclick={() => moveDown(idx)}
                  >▼</button>
                </span>
              </li>
            {/each}
          </ol>

          {#if inactiveSources.length > 0}
            <section class="inactive">
              <h4 class="group-title">{$t("study.music.source_inactive_header")}</h4>
              <ul class="inactive-list">
                {#each inactiveSources as src (src.slug)}
                  <li class="row dim">
                    <span class="name">{SOURCE_LABEL[src.slug] ?? src.slug}</span>
                    <span class="status off">
                      {statusLabel(src.slug, src.available)}
                    </span>
                    {#if canConnect(src.slug)}
                      <button type="button" class="link" onclick={() => openAuth(src.slug)}>
                        {$t("study.music.source_connect")} →
                      </button>
                    {/if}
                  </li>
                {/each}
              </ul>
            </section>
          {/if}

          <div class="footer">
            <button type="button" class="link reset" onclick={resetDefault} disabled={saving}>
              {$t("study.music.source_reset_default")}
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
    width: min(620px, 92vw);
    max-height: 88vh;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; padding: 18px 20px 8px; }
  .head-text { display: flex; flex-direction: column; gap: 4px; }
  .head h3 { margin: 0; font-size: 16px; font-weight: 800; }
  .state { font-size: 12px; color: rgba(255, 255, 255, 0.55); }
  .close { width: 28px; height: 28px; background: transparent; border: 0; border-radius: 50%; color: rgba(255, 255, 255, 0.5); font-size: 18px; cursor: pointer; flex-shrink: 0; }
  .close:hover { color: rgba(255, 255, 255, 0.9); background: rgba(255, 255, 255, 0.06); }
  .body { padding: 8px 20px 20px; overflow-y: auto; display: flex; flex-direction: column; gap: 14px; }
  .hint { margin: 0; color: rgba(255, 255, 255, 0.65); font-size: 13px; line-height: 1.5; }
  .loading { padding: 24px; text-align: center; color: rgba(255, 255, 255, 0.5); font-size: 12px; }

  .list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; counter-reset: src; }
  .row {
    display: grid;
    grid-template-columns: auto 1fr auto auto auto;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
  }
  .row.dim { opacity: 0.7; }
  .rank {
    width: 22px; height: 22px;
    display: grid; place-items: center;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 50%;
    font-size: 11px; font-weight: 700;
    color: rgba(255, 255, 255, 0.7);
  }
  .name { font-size: 13px; font-weight: 600; }
  .status { font-size: 11px; padding: 2px 8px; border-radius: 999px; font-weight: 600; }
  .status.on { background: rgba(34, 197, 94, 0.15); color: rgb(74, 222, 128); }
  .status.off { background: rgba(255, 255, 255, 0.06); color: rgba(255, 255, 255, 0.45); }
  .link {
    background: transparent;
    border: 0;
    color: var(--accent);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    padding: 0;
    font-weight: 600;
  }
  .link:hover:not(:disabled) { text-decoration: underline; }
  .link:disabled { opacity: 0.4; cursor: default; }
  .link.reset { color: rgba(255, 255, 255, 0.5); }
  .link.reset:hover { color: rgba(255, 255, 255, 0.85); }
  .arrows { display: flex; flex-direction: column; gap: 2px; }
  .arrow {
    width: 22px;
    height: 16px;
    background: transparent;
    border: 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 9px;
    cursor: pointer;
    border-radius: 3px;
    padding: 0;
  }
  .arrow:hover:not(:disabled) { background: rgba(255, 255, 255, 0.08); color: rgba(255, 255, 255, 0.9); }
  .arrow:disabled { opacity: 0.2; cursor: default; }

  .inactive { display: flex; flex-direction: column; gap: 6px; margin-top: 4px; }
  .group-title { margin: 0; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: rgba(255, 255, 255, 0.45); }
  .inactive-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .inactive-list .row { grid-template-columns: 1fr auto auto; }

  .footer { display: flex; justify-content: flex-end; padding-top: 6px; }
</style>
