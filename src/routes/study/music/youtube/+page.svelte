<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import {
    musicPlayer,
    type MusicTrack,
  } from "$lib/study-music/player-store.svelte";
  import { fmtDuration } from "$lib/study-music/format";
  import { t } from "$lib/i18n";

  type Playlist = {
    id: string;
    url: string;
    title: string | null;
    uploader: string | null;
    thumbnail: string | null;
    track_count: number;
    kind: string;
    added_at: number;
    refreshed_at: number;
  };

  type RemoteTrack = {
    video_id: string;
    title: string;
    artist: string;
    album: string | null;
    duration_ms: number | null;
    thumbnail: string | null;
  };

  let playlists = $state<Playlist[]>([]);
  let loading = $state(true);
  let busyId = $state<string | null>(null);

  let searchQuery = $state("");
  let searchResults = $state<RemoteTrack[]>([]);
  let searchLoading = $state(false);
  let searchError = $state<string | null>(null);
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  let recs = $state<RemoteTrack[]>([]);
  let recsSeedTitle = $state<string | null>(null);
  let downloadingId = $state<string | null>(null);

  async function load() {
    loading = true;
    try {
      const res = await pluginInvoke<{ playlists: Playlist[] }>(
        "study",
        "study:music:youtube:playlists_list",
      );
      playlists = res.playlists ?? [];
    } finally {
      loading = false;
    }
  }

  async function loadRecommendations() {
    try {
      const recent = await pluginInvoke<{
        tracks: Array<{ video_id: string; title: string | null; artist: string | null }>;
      }>("study", "study:music:youtube:tracks_recent", { limit: 1 });
      const seed = recent.tracks?.[0];
      if (!seed) return;
      recsSeedTitle = seed.title ?? seed.video_id;
      const res = await pluginInvoke<{ tracks: RemoteTrack[] }>(
        "study",
        "study:music:youtube:recommendations",
        { video_id: seed.video_id, limit: 12 },
      );
      recs = res.tracks ?? [];
    } catch (e) {
      console.warn("[recommendations] failed", e);
    }
  }

  function handleSearchInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    searchQuery = value;
    if (searchTimer) clearTimeout(searchTimer);
    if (!value.trim()) {
      searchResults = [];
      searchError = null;
      return;
    }
    searchTimer = setTimeout(() => void runSearch(value.trim()), 300);
  }

  async function runSearch(q: string) {
    if (!q) return;
    searchLoading = true;
    searchError = null;
    try {
      const res = await pluginInvoke<{ tracks: RemoteTrack[] }>(
        "study",
        "study:music:youtube:search",
        { query: q },
      );
      searchResults = res.tracks ?? [];
    } catch (e) {
      searchError = e instanceof Error ? e.message : String(e);
      searchResults = [];
    } finally {
      searchLoading = false;
    }
  }

  function clearSearch() {
    searchQuery = "";
    searchResults = [];
    searchError = null;
  }

  async function remoteToMusicTrack(rt: RemoteTrack): Promise<MusicTrack> {
    const ensured = await pluginInvoke<{ id: number }>(
      "study",
      "study:music:youtube:ensure_external_track",
      {
        video_id: rt.video_id,
        title: rt.title,
        artist: rt.artist,
        album: rt.album,
        duration_ms: rt.duration_ms,
        thumbnail: rt.thumbnail,
      },
    );
    return {
      id: ensured.id,
      path: `external://youtube/${rt.video_id}`,
      title: rt.title,
      artist: rt.artist,
      album: rt.album,
      duration_ms: rt.duration_ms,
      cover_path: rt.thumbnail,
      source: "youtube",
      youtube_url: `https://www.youtube.com/watch?v=${rt.video_id}`,
      youtube_video_id: rt.video_id,
      youtube_thumbnail: rt.thumbnail ?? undefined,
    };
  }

  async function playRemote(rt: RemoteTrack, queue: RemoteTrack[]) {
    try {
      const tracks = await Promise.all(queue.map((q) => remoteToMusicTrack(q)));
      const target = tracks.find((t) => t.youtube_video_id === rt.video_id);
      if (target) await musicPlayer.play(target, tracks);
      void loadRecommendations();
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function downloadRemote(rt: RemoteTrack) {
    if (downloadingId) return;
    downloadingId = rt.video_id;
    try {
      await pluginInvoke("study", "study:music:ytdlp:download_shortcut", {
        url: `https://www.youtube.com/watch?v=${rt.video_id}`,
      });
      showToast("success", $t("study.music.yt_download_started") as string);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      downloadingId = null;
    }
  }

  async function refresh(p: Playlist) {
    if (busyId) return;
    busyId = p.id;
    try {
      const res = await pluginInvoke<{ imported: number }>(
        "study",
        "study:music:youtube:playlist_refresh",
        { id: p.id },
      );
      showToast(
        "success",
        $t("study.music.yt_refreshed", { count: res.imported }) as string,
      );
      await load();
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busyId = null;
    }
  }

  async function remove(p: Playlist) {
    if (busyId) return;
    if (!window.confirm($t("study.music.yt_remove_confirm") as string)) return;
    busyId = p.id;
    try {
      await pluginInvoke("study", "study:music:youtube:playlist_remove", { id: p.id });
      await load();
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busyId = null;
    }
  }

  function open(p: Playlist) {
    goto(`/study/music/youtube/playlist/${encodeURIComponent(p.id)}`);
  }

  onMount(() => {
    void load();
    void loadRecommendations();
  });
</script>

<section class="page">
  <header class="head">
    <div>
      <h1>{$t("study.music.yt_title")}</h1>
      <p class="sub">{$t("study.music.yt_subtitle")}</p>
    </div>
    <button type="button" class="cta" onclick={() => musicUI.openYoutube()}>
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <line x1="12" y1="5" x2="12" y2="19"/>
        <line x1="5" y1="12" x2="19" y2="12"/>
      </svg>
      {$t("study.music.yt_add_playlist")}
    </button>
  </header>

  <div class="search-block">
    <div class="search-input-wrap">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" class="search-ico">
        <circle cx="11" cy="11" r="8"/>
        <line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      <input
        type="search"
        class="search-input"
        placeholder={$t("study.music.yt_search_placeholder") as string}
        value={searchQuery}
        oninput={handleSearchInput}
        spellcheck="false"
        autocomplete="off"
      />
      {#if searchQuery}
        <button type="button" class="search-clear" onclick={clearSearch} aria-label="clear">×</button>
      {/if}
    </div>
    {#if searchLoading}
      <p class="muted small">{$t("study.common.loading")}</p>
    {/if}
    {#if searchError}
      <p class="error small">{searchError}</p>
    {/if}
    {#if searchResults.length > 0}
      <ul class="track-list">
        {#each searchResults as rt (rt.video_id)}
          <li class="track-row">
            <button class="track-cover" onclick={() => playRemote(rt, searchResults)} aria-label="play">
              {#if rt.thumbnail}
                <img src={rt.thumbnail} alt="" loading="lazy" />
              {:else}
                <div class="track-cover-fallback"></div>
              {/if}
              <span class="track-play-overlay">
                <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
              </span>
            </button>
            <div class="track-meta">
              <span class="track-title">{rt.title}</span>
              <span class="track-sub">{rt.artist}{rt.album ? ` • ${rt.album}` : ""}</span>
            </div>
            <span class="track-dur">{rt.duration_ms ? fmtDuration(rt.duration_ms) : ""}</span>
            <button
              type="button"
              class="track-action"
              onclick={() => downloadRemote(rt)}
              disabled={downloadingId === rt.video_id}
              title={$t("study.music.yt_download") as string}
              aria-label={$t("study.music.yt_download") as string}
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if recs.length > 0 && !searchQuery}
    <div class="recs-block">
      <h2 class="shelf-title">
        {$t("study.music.yt_recs_for", { name: recsSeedTitle ?? "" })}
      </h2>
      <ul class="track-list">
        {#each recs as rt (rt.video_id)}
          <li class="track-row">
            <button class="track-cover" onclick={() => playRemote(rt, recs)} aria-label="play">
              {#if rt.thumbnail}
                <img src={rt.thumbnail} alt="" loading="lazy" />
              {:else}
                <div class="track-cover-fallback"></div>
              {/if}
              <span class="track-play-overlay">
                <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
              </span>
            </button>
            <div class="track-meta">
              <span class="track-title">{rt.title}</span>
              <span class="track-sub">{rt.artist}{rt.album ? ` • ${rt.album}` : ""}</span>
            </div>
            <span class="track-dur">{rt.duration_ms ? fmtDuration(rt.duration_ms) : ""}</span>
            <button
              type="button"
              class="track-action"
              onclick={() => downloadRemote(rt)}
              disabled={downloadingId === rt.video_id}
              title={$t("study.music.yt_download") as string}
              aria-label={$t("study.music.yt_download") as string}
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if playlists.length === 0}
    <div class="empty">
      <h2>{$t("study.music.yt_empty_title")}</h2>
      <p>{$t("study.music.yt_empty_body")}</p>
      <button type="button" class="cta-big" onclick={() => musicUI.openYoutube()}>
        {$t("study.music.yt_setup")}
      </button>
    </div>
  {:else}
    <ul class="grid">
      {#each playlists as p (p.id)}
        <li>
          <article class="card">
            <button type="button" class="card-cover" onclick={() => open(p)}>
              {#if p.thumbnail}
                <img src={p.thumbnail} alt="" loading="lazy" />
              {:else}
                <div class="fallback">
                  <svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <path d="M22.54 6.42a2.78 2.78 0 0 0-1.94-2C18.88 4 12 4 12 4s-6.88 0-8.6.46a2.78 2.78 0 0 0-1.94 2A29 29 0 0 0 1 11.75a29 29 0 0 0 .46 5.33A2.78 2.78 0 0 0 3.4 19c1.72.46 8.6.46 8.6.46s6.88 0 8.6-.46a2.78 2.78 0 0 0 1.94-2 29 29 0 0 0 .46-5.25 29 29 0 0 0-.46-5.33z"/>
                    <polygon points="9.75 15.02 15.5 11.75 9.75 8.48 9.75 15.02" fill="currentColor"/>
                  </svg>
                </div>
              {/if}
              <span class="play-overlay" aria-hidden="true">
                <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
              </span>
            </button>
            <div class="card-body">
              <h3>{p.title ?? p.id}</h3>
              {#if p.uploader}<span class="card-sub">{p.uploader}</span>{/if}
              <span class="card-meta">{p.track_count} faixas</span>
            </div>
            <div class="card-actions">
              <button
                type="button"
                class="ico"
                onclick={() => refresh(p)}
                disabled={busyId === p.id}
                title={$t("study.music.yt_refresh") as string}
                aria-label={$t("study.music.yt_refresh") as string}
              >
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="23 4 23 10 17 10"/>
                  <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                </svg>
              </button>
              <button
                type="button"
                class="ico danger"
                onclick={() => remove(p)}
                disabled={busyId === p.id}
                title={$t("study.music.yt_remove") as string}
                aria-label={$t("study.music.yt_remove") as string}
              >×</button>
            </div>
          </article>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding-bottom: 60px;
  }
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
  }
  .head h1 {
    margin: 0 0 4px;
    font-size: 28px;
    font-weight: 800;
    letter-spacing: -0.02em;
  }
  .sub {
    margin: 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
  }
  .cta {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    flex-shrink: 0;
  }
  .muted { color: rgba(255, 255, 255, 0.5); font-size: 13px; }
  .empty {
    padding: 80px 24px;
    text-align: center;
    color: rgba(255, 255, 255, 0.6);
    border: 1px dashed rgba(255, 255, 255, 0.08);
    border-radius: 12px;
  }
  .empty h2 {
    margin: 0 0 8px;
    font-size: 18px;
    color: rgba(255, 255, 255, 0.95);
  }
  .empty p {
    margin: 0 0 16px;
    font-size: 14px;
    line-height: 1.5;
  }
  .cta-big {
    padding: 10px 22px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 18px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 10px;
    position: relative;
    transition: background 120ms ease;
  }
  .card:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .card-cover {
    position: relative;
    width: 100%;
    aspect-ratio: 1 / 1;
    border: 0;
    padding: 0;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 6px;
    overflow: hidden;
    cursor: pointer;
  }
  .card-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .fallback {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    color: rgba(255, 255, 255, 0.3);
  }
  .play-overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: linear-gradient(to bottom, transparent 50%, rgba(0, 0, 0, 0.4));
    color: white;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .card-cover:hover .play-overlay { opacity: 1; }
  .card-body { display: flex; flex-direction: column; gap: 2px; padding: 0 4px; }
  .card-body h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.95);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-sub {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-meta {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.4);
    margin-top: 2px;
  }
  .card-actions {
    position: absolute;
    top: 14px;
    right: 14px;
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .card:hover .card-actions { opacity: 1; }
  .ico {
    width: 28px;
    height: 28px;
    padding: 0;
    background: rgba(0, 0, 0, 0.6);
    border: 0;
    border-radius: 50%;
    color: white;
    font-size: 14px;
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .ico:hover { background: rgba(0, 0, 0, 0.85); }
  .ico.danger:hover { background: rgb(220, 38, 38); }

  .search-block { display: flex; flex-direction: column; gap: 12px; }
  .search-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    padding: 0 14px 0 40px;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .search-input-wrap:focus-within {
    border-color: var(--accent);
    background: rgba(255, 255, 255, 0.07);
  }
  .search-ico {
    position: absolute;
    left: 14px;
    color: rgba(255, 255, 255, 0.5);
    pointer-events: none;
  }
  .search-input {
    flex: 1;
    background: transparent;
    border: 0;
    outline: 0;
    padding: 12px 0;
    color: white;
    font-family: inherit;
    font-size: 14px;
  }
  .search-clear {
    background: transparent;
    border: 0;
    color: rgba(255, 255, 255, 0.6);
    cursor: pointer;
    font-size: 18px;
    line-height: 1;
    padding: 4px 8px;
  }
  .small { font-size: 12px; }
  .error { color: rgb(248, 113, 113); }
  .recs-block { display: flex; flex-direction: column; gap: 12px; }
  .shelf-title {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.92);
  }
  .track-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .track-row {
    display: grid;
    grid-template-columns: 48px 1fr auto auto;
    gap: 12px;
    align-items: center;
    padding: 6px 8px;
    border-radius: 8px;
    transition: background 120ms ease;
  }
  .track-row:hover { background: rgba(255, 255, 255, 0.04); }
  .track-cover {
    position: relative;
    width: 48px;
    height: 48px;
    border: 0;
    padding: 0;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 6px;
    overflow: hidden;
    cursor: pointer;
  }
  .track-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .track-cover-fallback {
    width: 100%;
    height: 100%;
    background: rgba(255, 255, 255, 0.04);
  }
  .track-play-overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.5);
    color: white;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .track-cover:hover .track-play-overlay { opacity: 1; }
  .track-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
  }
  .track-title {
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.95);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .track-sub {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .track-dur {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    font-variant-numeric: tabular-nums;
  }
  .track-action {
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 0;
    color: rgba(255, 255, 255, 0.6);
    border-radius: 6px;
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .track-action:hover {
    background: rgba(255, 255, 255, 0.08);
    color: white;
  }
  .track-action:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
