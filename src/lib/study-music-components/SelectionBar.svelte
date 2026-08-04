<script lang="ts">
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { musicPlayer, type MusicTrack } from "$lib/study-music/player-store.svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";

  function getSelectedTracks(): MusicTrack[] {
    const ids = musicUI.selectedIds;
    if (ids.size === 0 || !musicUI.selectionAnchor) return [];
    return musicUI.selectionAnchor.queue.filter((tr) => ids.has(tr.id));
  }

  async function playSelection() {
    const tracks = getSelectedTracks();
    if (tracks.length === 0) return;
    await musicPlayer.play(tracks[0], tracks);
    musicUI.clearSelection();
  }

  function queueSelection() {
    const tracks = getSelectedTracks();
    if (tracks.length === 0) return;
    musicPlayer.queue = [...musicPlayer.queue, ...tracks];
    showToast(
      "success",
      $t("study.music.queued_count", { count: tracks.length }) as string,
    );
  }

  async function favoriteSelection() {
    const tracks = getSelectedTracks();
    for (const tr of tracks) {
      if (!tr.favorite) {
        await musicPlayer.toggleFavorite(tr.id);
      }
    }
    showToast(
      "success",
      $t("study.music.favorited_count", { count: tracks.length }) as string,
    );
    musicUI.clearSelection();
  }

  function addToPlaylist() {
    const tracks = getSelectedTracks();
    if (tracks.length === 0) return;
    musicUI.openAddToPlaylist(tracks[0]);
  }

  $effect(() => {
    if (typeof document === "undefined") return;
    function onKey(e: KeyboardEvent) {
      if (musicUI.selectedCount() === 0) return;
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      ) {
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        musicUI.clearSelection();
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
        const queue = musicUI.selectionAnchor?.queue;
        if (queue && queue.length > 0) {
          e.preventDefault();
          const all = new Set(queue.map((t) => t.id));
          musicUI.selectedIds = all;
        }
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  });
</script>

{#if musicUI.selectedCount() > 0}
  <div class="selection-bar" role="toolbar" aria-label={$t("study.music.selection_aria") as string}>
    <span class="count">
      {$t("study.music.n_selected", { count: musicUI.selectedCount() })}
    </span>
    <div class="actions">
      <button type="button" class="action play" onclick={playSelection} title={$t("study.music.play") as string}>
        <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        <span>{$t("study.music.play")}</span>
      </button>
      <button type="button" class="action" onclick={queueSelection} title={$t("study.music.add_to_queue") as string}>
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="6" x2="16" y2="6"/><line x1="3" y1="12" x2="16" y2="12"/><line x1="3" y1="18" x2="11" y2="18"/><line x1="20" y1="14" x2="20" y2="20"/><line x1="17" y1="17" x2="23" y2="17"/></svg>
        <span>{$t("study.music.add_to_queue")}</span>
      </button>
      <button type="button" class="action" onclick={addToPlaylist} title={$t("study.music.add_to_playlist") as string}>
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        <span>{$t("study.music.add_to_playlist")}</span>
      </button>
      <button type="button" class="action" onclick={favoriteSelection} title={$t("study.music.favorite") as string}>
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/></svg>
        <span>{$t("study.music.favorite")}</span>
      </button>
    </div>
    <button type="button" class="dismiss" onclick={() => musicUI.clearSelection()} aria-label={$t("study.common.close") as string}>×</button>
  </div>
{/if}

<style>
  .selection-bar {
    position: fixed;
    bottom: 96px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 110;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 12px 8px 16px;
    background: rgb(28, 28, 30);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    animation: slideUp 180ms ease;
  }
  @keyframes slideUp {
    from { opacity: 0; transform: translateX(-50%) translateY(8px); }
    to { opacity: 1; transform: translateX(-50%) translateY(0); }
  }
  .count {
    font-size: 12px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.85);
    padding-right: 4px;
    border-right: 1px solid rgba(255, 255, 255, 0.1);
    margin-right: 4px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .action {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: transparent;
    border: 0;
    border-radius: 999px;
    color: rgba(255, 255, 255, 0.85);
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .action:hover {
    background: rgba(255, 255, 255, 0.08);
    color: white;
  }
  .action.play {
    background: var(--accent);
    color: var(--on-accent, white);
    font-weight: 700;
  }
  .action.play:hover {
    filter: brightness(1.1);
  }
  .dismiss {
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.5);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
  }
  .dismiss:hover {
    color: white;
    background: rgba(255, 255, 255, 0.08);
  }
</style>
