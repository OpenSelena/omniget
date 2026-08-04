<script lang="ts">
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";

  type PresetEntry = {
    id: string;
    label: string;
    preset: unknown;
  };

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  let presets = $state<PresetEntry[]>([]);
  let currentId = $state<string | null>(null);
  let loading = $state(false);
  let saving = $state(false);

  const GROUPS: { headerKey: string; ids: string[] }[] = [
    { headerKey: "study.music.quality_group_recommended", ids: ["aac-192"] },
    { headerKey: "study.music.quality_group_compact", ids: ["mp3-128", "opus-160", "mp3-192"] },
    { headerKey: "study.music.quality_group_high", ids: ["aac-256", "mp3-320"] },
    { headerKey: "study.music.quality_group_lossless", ids: ["flac-cd", "flac-hires", "flac", "wav"] },
  ];

  const HINT_KEYS: Record<string, string> = {
    "mp3-128": "study.music.quality_hint_mp3_128",
    "mp3-192": "study.music.quality_hint_mp3_192",
    "mp3-320": "study.music.quality_hint_mp3_320",
    "aac-192": "study.music.quality_hint_aac_192",
    "aac-256": "study.music.quality_hint_aac_256",
    "opus-160": "study.music.quality_hint_opus_160",
    "flac": "study.music.quality_hint_flac",
    "flac-cd": "study.music.quality_hint_flac_cd",
    "flac-hires": "study.music.quality_hint_flac_hires",
    "wav": "study.music.quality_hint_wav",
  };

  async function load() {
    loading = true;
    try {
      const [listRes, getRes] = await Promise.all([
        pluginInvoke<{ presets: PresetEntry[] }>("study", "study:music:quality:list"),
        pluginInvoke<{ preset: unknown; label: string }>("study", "study:music:quality:get"),
      ]);
      presets = listRes.presets ?? [];
      const getJson = JSON.stringify(getRes.preset);
      const match = presets.find((p) => JSON.stringify(p.preset) === getJson);
      currentId = match?.id ?? null;
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (open) void load();
  });

  async function pick(entry: PresetEntry) {
    if (saving || entry.id === currentId) return;
    const previous = currentId;
    currentId = entry.id;
    saving = true;
    try {
      await pluginInvoke("study", "study:music:quality:set", { preset: entry.preset });
      showToast("success", $t("study.music.quality_saved", { label: entry.label }) as string);
    } catch (e) {
      currentId = previous;
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      saving = false;
    }
  }

  function findPreset(id: string): PresetEntry | undefined {
    return presets.find((p) => p.id === id);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }

  const currentLabel = $derived(
    presets.find((p) => p.id === currentId)?.label ?? "—",
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
          <h3>{$t("study.music.quality_title")}</h3>
          <span class="state on">{$t("study.music.quality_current", { label: currentLabel })}</span>
        </div>
        <button type="button" class="close" onclick={onClose} aria-label={$t("study.common.close") as string}>×</button>
      </header>
      <div class="body">
        <p class="hint">{$t("study.music.quality_intro")}</p>

        {#if loading}
          <div class="loading">{$t("study.common.loading")}</div>
        {:else}
          {#each GROUPS as group}
            <section class="group">
              <h4 class="group-title">{$t(group.headerKey)}</h4>
              <div class="cards">
                {#each group.ids as id}
                  {@const entry = findPreset(id)}
                  {#if entry}
                    {@const isCurrent = currentId === id}
                    <button
                      type="button"
                      class="card"
                      class:selected={isCurrent}
                      disabled={saving}
                      onclick={() => pick(entry)}
                    >
                      <span class="card-title">
                        <span>{entry.label}</span>
                        {#if isCurrent}
                          <svg class="check" viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                            <polyline points="3 9 6.5 12.5 13 4" />
                          </svg>
                        {/if}
                      </span>
                      <span class="card-hint">{$t(HINT_KEYS[id])}</span>
                    </button>
                  {/if}
                {/each}
              </div>
            </section>
          {/each}
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
  .close { width: 28px; height: 28px; background: transparent; border: 0; border-radius: 50%; color: rgba(255, 255, 255, 0.5); font-size: 18px; cursor: pointer; flex-shrink: 0; }
  .close:hover { color: rgba(255, 255, 255, 0.9); background: rgba(255, 255, 255, 0.06); }
  .body { padding: 8px 20px 20px; overflow-y: auto; display: flex; flex-direction: column; gap: 18px; }
  .hint { margin: 0; color: rgba(255, 255, 255, 0.65); font-size: 13px; line-height: 1.5; }
  .loading { padding: 24px; text-align: center; color: rgba(255, 255, 255, 0.5); font-size: 12px; }
  .group { display: flex; flex-direction: column; gap: 8px; }
  .group-title { margin: 0; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: rgba(255, 255, 255, 0.45); }
  .cards { display: flex; flex-direction: column; gap: 6px; }
  .card {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 12px 14px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    color: inherit;
    text-align: left;
    cursor: pointer;
    font-family: inherit;
    transition: border-color 80ms ease, background 80ms ease;
  }
  .card:hover:not(:disabled) {
    border-color: rgba(255, 255, 255, 0.18);
    background: rgba(255, 255, 255, 0.05);
  }
  .card:disabled { opacity: 0.5; cursor: default; }
  .card.selected {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 10%, rgba(255, 255, 255, 0.03));
  }
  .card-title { display: flex; align-items: center; justify-content: space-between; gap: 8px; font-size: 13px; font-weight: 700; }
  .check { color: var(--accent); flex-shrink: 0; }
  .card-hint { font-size: 11px; color: rgba(255, 255, 255, 0.55); line-height: 1.4; }
</style>
