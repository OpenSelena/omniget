<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";
  import { fmtDuration, fmtDurationLong } from "$lib/study-music/format";
  import { musicPlayer, type MusicTrack } from "$lib/study-music/player-store.svelte";
  import { playlistsStore } from "$lib/study-music/playlists-store.svelte";
  import CoverImage from "$lib/study-music-components/CoverImage.svelte";

  type PlaylistDetail = {
    id: number;
    name: string;
    description: string | null;
    cover_path: string | null;
    track_count: number;
    total_duration_ms: number | null;
    resolved_cover: string | null;
    created_at: number;
    updated_at: number;
  };

  type PlaylistTrack = MusicTrack & { position: number };

  const playlistId = $derived(Number($page.params.id));

  let detail = $state<PlaylistDetail | null>(null);
  let tracks = $state<PlaylistTrack[]>([]);
  let loading = $state(true);
  let editingName = $state(false);
  let nameValue = $state("");
  let nameRef = $state<HTMLInputElement | null>(null);
  let confirmDeleteOpen = $state(false);
  let dragIndex = $state<number | null>(null);
  let dragOver = $state<number | null>(null);

  async function load() {
    if (!Number.isFinite(playlistId)) return;
    loading = true;
    try {
      const res = await pluginInvoke<{ playlist: PlaylistDetail; tracks: PlaylistTrack[] }>(
        "study",
        "study:music:playlists:get",
        { id: playlistId },
      );
      detail = res.playlist;
      tracks = res.tracks ?? [];
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });

  $effect(() => {
    void playlistId;
    void load();
  });

  function playAll() {
    if (tracks.length === 0) return;
    void musicPlayer.play(tracks[0], tracks);
  }

  function shufflePlay() {
    if (tracks.length === 0) return;
    musicPlayer.shuffle = true;
    const idx = Math.floor(Math.random() * tracks.length);
    void musicPlayer.play(tracks[idx], tracks);
  }

  async function removeTrack(track: PlaylistTrack) {
    try {
      await pluginInvoke("study", "study:music:playlists:remove_track", {
        playlistId,
        trackId: track.id,
      });
      tracks = tracks.filter((t) => t.id !== track.id);
      void playlistsStore.load(true);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    }
  }

  async function moveTrack(from: number, to: number) {
    if (from === to || from < 0 || to < 0 || from >= tracks.length || to >= tracks.length) return;
    const next = [...tracks];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    tracks = next;
    try {
      await pluginInvoke("study", "study:music:playlists:reorder", {
        playlistId,
        trackIds: next.map((t) => t.id),
      });
      void playlistsStore.load(true);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
      void load();
    }
  }

  function startEditName() {
    if (!detail) return;
    editingName = true;
    nameValue = detail.name;
    queueMicrotask(() => {
      nameRef?.focus();
      nameRef?.select();
    });
  }

  async function saveName() {
    const trimmed = nameValue.trim();
    if (!trimmed || !detail || trimmed === detail.name) {
      editingName = false;
      nameValue = "";
      return;
    }
    try {
      await playlistsStore.update(detail.id, trimmed);
      detail = { ...detail, name: trimmed };
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    } finally {
      editingName = false;
    }
  }

  function cancelEditName() {
    editingName = false;
    nameValue = "";
  }

  async function performDelete() {
    if (!detail) return;
    try {
      await playlistsStore.deleteOne(detail.id);
      showToast("success", $t("study.music.playlist_deleted") as string);
      goto("/study/music/playlists");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    } finally {
      confirmDeleteOpen = false;
    }
  }

  function onDragStart(e: DragEvent, idx: number) {
    dragIndex = idx;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(idx));
    }
  }

  function onDragOver(e: DragEvent, idx: number) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    dragOver = idx;
  }

  function onDrop(e: DragEvent, idx: number) {
    e.preventDefault();
    if (dragIndex === null) return;
    void moveTrack(dragIndex, idx);
    dragIndex = null;
    dragOver = null;
  }

  function onDragEnd() {
    dragIndex = null;
    dragOver = null;
  }
</script>

<section class="playlist-page">
  {#if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if !detail}
    <p class="error">{$t("study.music.playlist_not_found")}</p>
  {:else}
    <header class="hero">
      <div class="cover-block">
        <CoverImage
          src={detail.resolved_cover}
          alt={detail.name}
          fallbackSeed={detail.name}
          rounded="lg"
        />
      </div>
      <div class="meta">
        <span class="eyebrow">{$t("study.music.eyebrow_playlist")}</span>
        {#if editingName}
          <input
            bind:this={nameRef}
            bind:value={nameValue}
            class="title-edit"
            type="text"
            onkeydown={(e) => {
              if (e.key === "Enter") saveName();
              else if (e.key === "Escape") cancelEditName();
            }}
            onblur={saveName}
          />
        {:else}
          <h1 onclick={startEditName} title={$t("study.music.click_to_rename") as string}>{detail.name}</h1>
        {/if}
        <div class="info">
          <span>{detail.track_count} faixa(s)</span>
          {#if detail.total_duration_ms}
            <span class="dot" aria-hidden="true">·</span>
            <span>{fmtDurationLong(detail.total_duration_ms)}</span>
          {/if}
        </div>
        <div class="actions">
          <button type="button" class="play-big" onclick={playAll} disabled={tracks.length === 0}>
            <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
            <span>{$t("study.music.play")}</span>
          </button>
          <button
            type="button"
            class="ghost-big"
            onclick={shufflePlay}
            disabled={tracks.length === 0}
            title={$t("study.music.shuffle") as string}
            aria-label={$t("study.music.shuffle") as string}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <polyline points="16 3 21 3 21 8"/>
              <line x1="4" y1="20" x2="21" y2="3"/>
              <polyline points="21 16 21 21 16 21"/>
              <line x1="15" y1="15" x2="21" y2="21"/>
              <line x1="4" y1="4" x2="9" y2="9"/>
            </svg>
          </button>
          <button
            type="button"
            class="ghost-big danger"
            onclick={() => (confirmDeleteOpen = true)}
            title={$t("study.music.delete_playlist") as string}
            aria-label={$t("study.music.delete_playlist") as string}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <polyline points="3 6 5 6 21 6"/>
              <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/>
              <path d="M10 11v6"/>
              <path d="M14 11v6"/>
              <path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/>
            </svg>
          </button>
        </div>
      </div>
    </header>

    {#if tracks.length === 0}
      <div class="empty">
        <p>{$t("study.music.playlist_empty_title")}</p>
        <p class="sub">{$t("study.music.playlist_empty_body")}</p>
      </div>
    {:else}
      <ol class="track-list">
        {#each tracks as track, i (track.id)}
          <li
            class="track-li"
            class:dragover={dragOver === i}
            draggable="true"
            ondragstart={(e) => onDragStart(e, i)}
            ondragover={(e) => onDragOver(e, i)}
            ondrop={(e) => onDrop(e, i)}
            ondragend={onDragEnd}
          >
            <button
              type="button"
              class="play-cell"
              onclick={() => musicPlayer.play(track, tracks)}
            >
              <span class="num">{i + 1}</span>
              <CoverImage src={track.cover_path} alt={track.title ?? track.path} size={36} fallbackSeed={track.album ?? track.title} rounded="sm" />
              <span class="info">
                <span class="title">{track.title ?? track.path}</span>
                {#if track.artist}
                  <span class="artist">{track.artist}</span>
                {/if}
              </span>
              {#if track.album}
                <span class="album">{track.album}</span>
              {/if}
              <span class="dur">{fmtDuration(track.duration_ms)}</span>
            </button>
            <div class="row-actions">
              <button
                type="button"
                class="ico"
                onclick={() => moveTrack(i, i - 1)}
                disabled={i === 0}
                aria-label="Subir"
                title="Subir"
              >↑</button>
              <button
                type="button"
                class="ico"
                onclick={() => moveTrack(i, i + 1)}
                disabled={i === tracks.length - 1}
                aria-label="Descer"
                title="Descer"
              >↓</button>
              <button
                type="button"
                class="ico danger"
                onclick={() => removeTrack(track)}
                aria-label={$t("study.music.remove_from_playlist") as string}
                title={$t("study.music.remove_from_playlist") as string}
              >×</button>
            </div>
          </li>
        {/each}
      </ol>
    {/if}

    {#if confirmDeleteOpen}
      <div class="confirm-overlay" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) confirmDeleteOpen = false; }}>
        <div class="confirm-dialog" role="dialog" aria-modal="true">
          <h3>{$t("study.music.delete_confirm_title")}</h3>
          <p>{$t("study.music.delete_confirm_msg", { name: detail.name })}</p>
          <div class="confirm-actions">
            <button type="button" class="ghost" onclick={() => (confirmDeleteOpen = false)}>
              {$t("study.common.cancel")}
            </button>
            <button type="button" class="danger" onclick={performDelete}>
              {$t("study.music.delete_confirm")}
            </button>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</section>

<style>
  .playlist-page { display: flex; flex-direction: column; gap: 18px; }
  .hero {
    display: flex;
    align-items: flex-end;
    gap: 24px;
    padding: 24px 4px 18px;
  }
  .cover-block {
    flex-shrink: 0;
    width: clamp(160px, 22vw, 220px);
    aspect-ratio: 1 / 1;
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 12px 36px color-mix(in oklab, black 32%, transparent);
  }
  .meta { display: flex; flex-direction: column; gap: 10px; min-width: 0; flex: 1; }
  .eyebrow { font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--tertiary); }
  .meta h1 {
    margin: 0;
    font-size: clamp(28px, 4.5vw, 56px);
    font-weight: 800;
    letter-spacing: -0.02em;
    color: var(--secondary);
    cursor: text;
    overflow-wrap: anywhere;
  }
  .meta h1:hover { color: var(--accent); }
  .title-edit {
    margin: 0;
    font-size: clamp(28px, 4.5vw, 56px);
    font-weight: 800;
    letter-spacing: -0.02em;
    color: var(--secondary);
    background: transparent;
    border: 0;
    border-bottom: 2px solid var(--accent);
    outline: none;
    font-family: inherit;
    width: 100%;
  }
  .info { display: flex; gap: 6px; align-items: center; color: var(--tertiary); font-size: 13px; }
  .info .dot { opacity: 0.6; }
  .actions { display: flex; align-items: center; gap: 10px; margin-top: 6px; }
  .play-big {
    display: inline-flex; align-items: center; gap: 8px;
    padding: 10px 22px; background: var(--accent); color: var(--on-accent, white);
    border: 0; border-radius: 999px; font-family: inherit; font-size: 14px; font-weight: 700; cursor: pointer;
    box-shadow: 0 4px 14px color-mix(in oklab, var(--accent) 35%, transparent);
    transition: transform 120ms ease;
  }
  .play-big:hover:not(:disabled) { transform: scale(1.04); }
  .play-big:disabled { opacity: 0.4; cursor: default; }
  .ghost-big {
    width: 40px; height: 40px; padding: 0;
    background: transparent; border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 50%; color: var(--tertiary); cursor: pointer;
    display: grid; place-items: center;
    transition: color 120ms ease, border-color 120ms ease;
  }
  .ghost-big:hover:not(:disabled) { color: var(--accent); border-color: var(--accent); }
  .ghost-big.danger:hover { color: var(--error, #dc2626); border-color: var(--error, #dc2626); }
  .ghost-big:disabled { opacity: 0.4; cursor: default; }

  .track-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 1px; }
  .track-li {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
    align-items: center;
    padding: 0 4px;
    border-radius: 6px;
    transition: background 120ms ease;
  }
  .track-li.dragover {
    background: color-mix(in oklab, var(--accent) 16%, transparent);
    outline: 1px dashed var(--accent);
  }
  .play-cell {
    display: grid;
    grid-template-columns: 32px 36px 1fr minmax(120px, 200px) auto;
    gap: 12px;
    align-items: center;
    padding: 6px 8px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: var(--secondary);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 120ms ease;
    min-width: 0;
  }
  .play-cell:hover { background: color-mix(in oklab, var(--accent) 6%, transparent); }
  .num { color: var(--tertiary); font-size: 12px; font-variant-numeric: tabular-nums; text-align: center; }
  .info { display: flex; flex-direction: column; min-width: 0; overflow: hidden; }
  .title { font-size: 13px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .artist { font-size: 11px; color: var(--tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .album { font-size: 12px; color: var(--tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dur { font-size: 12px; color: var(--tertiary); font-variant-numeric: tabular-nums; min-width: 48px; text-align: right; }
  .row-actions {
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .track-li:hover .row-actions { opacity: 1; }
  .ico {
    width: 26px; height: 26px; padding: 0;
    background: transparent;
    border: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
    border-radius: 4px;
    color: var(--tertiary);
    cursor: pointer;
    font-family: inherit;
    font-size: 12px;
    line-height: 1;
    transition: color 120ms ease, border-color 120ms ease, background 120ms ease;
  }
  .ico:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .ico.danger:hover {
    color: var(--error, #dc2626);
    border-color: var(--error, #dc2626);
    background: color-mix(in oklab, var(--error, #dc2626) 10%, transparent);
  }
  .ico:disabled { opacity: 0.3; cursor: default; }

  .empty {
    padding: 48px 24px;
    text-align: center;
    color: var(--tertiary);
  }
  .empty p { margin: 0; }
  .empty .sub { margin-top: 4px; font-size: 13px; }

  .confirm-overlay {
    position: fixed;
    inset: 0;
    background: color-mix(in oklab, black 50%, transparent);
    display: grid;
    place-items: center;
    z-index: 250;
  }
  .confirm-dialog {
    background: var(--surface, var(--button-elevated));
    border: 1px solid var(--content-border);
    border-radius: 12px;
    padding: 20px;
    max-width: 380px;
    box-shadow: 0 16px 40px color-mix(in oklab, black 38%, transparent);
  }
  .confirm-dialog h3 { margin: 0 0 8px; font-size: 16px; color: var(--secondary); }
  .confirm-dialog p { margin: 0 0 16px; color: var(--tertiary); font-size: 13px; }
  .confirm-actions { display: flex; justify-content: flex-end; gap: 8px; }
  .confirm-actions .ghost {
    padding: 8px 14px; background: transparent;
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 8px; color: var(--secondary); font-family: inherit; font-size: 13px; cursor: pointer;
  }
  .confirm-actions .danger {
    padding: 8px 14px; background: var(--error, #dc2626); color: white;
    border: 0; border-radius: 8px; font-family: inherit; font-size: 13px; font-weight: 600; cursor: pointer;
  }
  .muted { color: var(--tertiary); font-size: 13px; }
  .error { color: var(--error, #dc2626); font-size: 13px; }
  @media (max-width: 720px) {
    .hero { flex-direction: column; align-items: flex-start; }
    .play-cell { grid-template-columns: 28px 36px 1fr auto; }
    .album { display: none; }
  }
</style>
