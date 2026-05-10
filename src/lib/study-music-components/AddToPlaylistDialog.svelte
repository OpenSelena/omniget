<script lang="ts">
  import { onMount } from "svelte";
  import { playlistsStore } from "$lib/study-music/playlists-store.svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";

  type Props = {
    open: boolean;
    trackId: number | null;
    trackName?: string | null;
    onClose: () => void;
  };

  let { open = $bindable(), trackId, trackName, onClose }: Props = $props();

  let creatingNew = $state(false);
  let newName = $state("");
  let newRef = $state<HTMLInputElement | null>(null);
  let busy = $state(false);

  $effect(() => {
    if (open) {
      void playlistsStore.load();
      creatingNew = false;
      newName = "";
    }
  });

  async function addToExisting(playlistId: number) {
    if (busy || trackId == null) return;
    busy = true;
    try {
      const res = await playlistsStore.addTrack(playlistId, trackId);
      const pl = playlistsStore.list.find((p) => p.id === playlistId);
      if (res.duplicate) {
        showToast("info", $t("study.music.playlist_duplicate", { name: pl?.name ?? "" }) as string);
      } else {
        showToast(
          "success",
          $t("study.music.added_to_playlist", { name: pl?.name ?? "" }) as string,
        );
      }
      onClose();
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
    }
  }

  async function createAndAdd() {
    const trimmed = newName.trim();
    if (!trimmed || trackId == null || busy) return;
    busy = true;
    try {
      const id = await playlistsStore.create(trimmed);
      if (id) {
        await playlistsStore.addTrack(id, trackId);
        showToast(
          "success",
          $t("study.music.created_playlist_with_track", { name: trimmed }) as string,
        );
      }
      onClose();
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busy = false;
      newName = "";
      creatingNew = false;
    }
  }

  function startNew() {
    creatingNew = true;
    queueMicrotask(() => newRef?.focus());
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
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
        <h3>{$t("study.music.add_to_playlist")}</h3>
        {#if trackName}
          <p class="track-name">{trackName}</p>
        {/if}
        <button type="button" class="close" onclick={onClose} aria-label={$t("study.common.close")}>×</button>
      </header>
      <div class="content">
        <button type="button" class="new-btn" onclick={startNew} disabled={creatingNew}>
          + {$t("study.music.new_playlist")}
        </button>
        {#if creatingNew}
          <div class="new-row">
            <input
              type="text"
              bind:this={newRef}
              bind:value={newName}
              placeholder={$t("study.music.playlist_name_placeholder")}
              onkeydown={(e) => {
                if (e.key === "Enter") createAndAdd();
                else if (e.key === "Escape") {
                  e.stopPropagation();
                  creatingNew = false;
                  newName = "";
                }
              }}
            />
            <button type="button" class="confirm" onclick={createAndAdd} disabled={!newName.trim() || busy}>
              {$t("study.common.create")}
            </button>
          </div>
        {/if}
        {#if playlistsStore.list.length === 0}
          <p class="muted">{$t("study.music.no_playlists_yet")}</p>
        {:else}
          <ul class="list">
            {#each playlistsStore.list as p (p.id)}
              <li>
                <button
                  type="button"
                  class="row"
                  onclick={() => addToExisting(p.id)}
                  disabled={busy}
                >
                  <span class="row-name">{p.name}</span>
                  <span class="row-count">{p.track_count}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: color-mix(in oklab, black 50%, transparent);
    z-index: 300;
    display: grid;
    place-items: center;
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }
  .dialog {
    background: var(--surface, var(--button-elevated));
    border: 1px solid var(--content-border);
    border-radius: 12px;
    width: min(440px, 92vw);
    max-height: 80vh;
    box-shadow: 0 16px 48px color-mix(in oklab, black 40%, transparent);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head {
    position: relative;
    padding: 16px 18px 12px;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
  }
  .head h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    color: var(--secondary);
  }
  .track-name {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .close {
    position: absolute;
    top: 10px;
    right: 12px;
    width: 28px;
    height: 28px;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: var(--tertiary);
    font-size: 18px;
    cursor: pointer;
  }
  .close:hover { color: var(--secondary); background: color-mix(in oklab, var(--accent) 8%, transparent); }
  .content {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
  }
  .new-btn {
    align-self: flex-start;
    padding: 6px 14px;
    border: 1px dashed color-mix(in oklab, var(--accent) 50%, transparent);
    border-radius: 999px;
    background: transparent;
    color: var(--accent);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .new-btn:disabled { opacity: 0.5; cursor: default; }
  .new-row {
    display: flex;
    gap: 8px;
  }
  .new-row input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid var(--accent);
    border-radius: 8px;
    background: var(--input-bg);
    color: var(--secondary);
    font-family: inherit;
    font-size: 13px;
    outline: none;
  }
  .confirm {
    padding: 8px 14px;
    border: 0;
    border-radius: 8px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .confirm:disabled { opacity: 0.5; cursor: default; }
  .list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 1px; }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 10px 12px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: var(--secondary);
    font-family: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .row:hover { background: color-mix(in oklab, var(--accent) 8%, transparent); }
  .row:disabled { opacity: 0.5; cursor: default; }
  .row-count {
    font-size: 11px;
    color: var(--tertiary);
    font-variant-numeric: tabular-nums;
  }
  .muted { color: var(--tertiary); font-size: 12px; padding: 8px 4px; }
</style>
