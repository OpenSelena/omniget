<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import {
    soundcloudStore,
    type ScPlaylist,
    type ScTrack,
  } from "$lib/study-music/soundcloud-store.svelte";
  import { downloadStore } from "$lib/study-music/download-store.svelte";
  import {
    getFirstDownloadDone,
    getLastCodec,
    getLastDownloadDir,
    setLastCodec,
    setLastDownloadDir,
  } from "$lib/study-music/local-prefs";
  import { colorFromString } from "$lib/study-music/format";
  import SoundCloudDownloadDialog from "$lib/study-music-components/SoundCloudDownloadDialog.svelte";
  import SoundCloudDownloadButton from "$lib/study-music-components/SoundCloudDownloadButton.svelte";
  import SoundCloudError from "$lib/study-music-components/SoundCloudError.svelte";

  let playlist = $state<ScPlaylist | null>(null);
  let tracks = $state<ScTrack[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let playbackError = $state<string | null>(null);
  let playbackTrack = $state<ScTrack | null>(null);
  let playbackQueue = $state<ScTrack[] | null>(null);
  let downloadTrack = $state<ScTrack | null>(null);

  let bulkBusy = $state(false);

  const playlistId = $derived(parseInt($page.params.id ?? "0", 10));

  async function load() {
    if (!playlistId) {
      goto("/study/music/soundcloud");
      return;
    }
    loading = true;
    error = null;
    try {
      const p = await soundcloudStore.getPlaylist(playlistId);
      playlist = p;
      tracks = p?.tracks ?? [];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function playQueue(track: ScTrack, queue: ScTrack[]) {
    playbackError = null;
    playbackTrack = track;
    playbackQueue = queue;
    try {
      await soundcloudStore.playTrack(track, queue);
    } catch (e) {
      playbackError = e instanceof Error ? e.message : String(e);
    }
  }

  async function play(idx: number) {
    if (tracks.length === 0) return;
    const reordered = [...tracks.slice(idx), ...tracks.slice(0, idx)];
    await playQueue(reordered[0], reordered);
  }

  function retryPlayback() {
    if (!playbackTrack) return;
    void playQueue(playbackTrack, playbackQueue ?? [playbackTrack]);
  }

  function fmtDuration(ms: number): string {
    const total = Math.floor(ms / 1000);
    const m = Math.floor(total / 60);
    const s = total % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  async function downloadAll() {
    if (bulkBusy || tracks.length === 0 || !playlist) return;
    const codec = getLastCodec() ?? "mp3";
    let dir = getLastDownloadDir();
    if (!dir) {
      const dialog = await import("@tauri-apps/plugin-dialog");
      const picked = await dialog.open({ directory: true, multiple: false });
      if (typeof picked !== "string" || !picked) return;
      dir = picked;
      setLastDownloadDir(dir);
      setLastCodec(codec);
    }
    bulkBusy = true;
    downloadStore.addOptimisticBulkJob({
      playlistId,
      title: playlist.title,
      total: tracks.length,
      artwork: soundcloudStore.pickArtwork(playlist.artwork_url) ?? null,
      codec,
      outputDir: dir,
    });
    downloadStore.toggleDrawer(true);
    try {
      const res = await soundcloudStore.downloadPlaylist({
        playlistId,
        codec,
        outputDir: dir,
        quality: "progressive",
      });
      showToast(
        "success",
        $t("study.music.sc_download_bulk_done", {
          success: res.success,
          failed: res.failed,
        }) as string,
      );
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      bulkBusy = false;
    }
  }

  function folderName(dir: string): string {
    const parts = dir.split(/[\\/]/).filter(Boolean);
    return parts[parts.length - 1] ?? dir;
  }

  async function startInlineDownload(track: ScTrack) {
    const codec = getLastCodec() ?? "mp3";
    const dir = getLastDownloadDir();
    if (!dir) {
      downloadTrack = track;
      return;
    }
    const optId = downloadStore.addOptimisticSingleJob({
      trackId: track.id,
      title: track.title,
      artist: track.user.username,
      artwork:
        soundcloudStore.pickArtwork(track.artwork_url ?? track.user.avatar_url) ?? null,
    });
    try {
      await soundcloudStore.download({
        trackId: track.id,
        codec,
        outputDir: dir,
        quality: "progressive",
      });
      showToast("success", `Pronto — salvo na pasta ${folderName(dir)}`);
    } catch (e) {
      downloadStore.markJobError(optId, e instanceof Error ? e.message : String(e));
    }
  }

  function handleDownload(track: ScTrack, advanced: boolean) {
    if (advanced || !getFirstDownloadDone() || !getLastDownloadDir()) {
      downloadTrack = track;
      return;
    }
    void startInlineDownload(track);
  }
</script>

<section class="page">
  <button type="button" class="back" onclick={() => history.back()}>
    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <polyline points="15 18 9 12 15 6"/>
    </svg>
    Voltar
  </button>

  {#if playlist}
    <header class="hero">
      <div class="cover">
        {#if playlist.artwork_url}
          <img src={soundcloudStore.pickArtwork(playlist.artwork_url)} alt="" />
        {:else}
          <div class="cover-fb" style:background={colorFromString(playlist.title)}></div>
        {/if}
      </div>
      <div class="info">
        <span class="eyebrow">{playlist.is_album ? "Álbum" : "Playlist"}</span>
        <h1>{playlist.title}</h1>
        <p class="meta">{playlist.user.username} · {playlist.track_count} faixas · {fmtDuration(playlist.duration)}</p>
        {#if playlist.description}
          <p class="desc">{playlist.description}</p>
        {/if}
        <div class="actions">
          <button type="button" class="play-btn" onclick={() => play(0)} disabled={tracks.length === 0}>
            <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
            Tocar
          </button>
          <button type="button" class="bulk-btn" onclick={downloadAll} disabled={bulkBusy || tracks.length === 0}>
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
            {bulkBusy
              ? "Baixando playlist…"
              : $t('study.music.sc_download_all', { count: tracks.length })}
          </button>
        </div>
      </div>
    </header>
  {/if}

  {#if loading}
    <p class="muted">Carregando…</p>
  {:else if error}
    <SoundCloudError
      {error}
      trackUrl={playlist?.permalink_url}
      onRetry={load}
    />
  {:else}
    {#if playbackError}
      <SoundCloudError
        error={playbackError}
        trackUrl={playbackTrack?.permalink_url}
        onRetry={retryPlayback}
      />
    {/if}
    <div class="track-list">
      {#each tracks as track, i (track.id)}
        <div class="track-row" role="button" tabindex="0" onclick={() => play(i)} onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); play(i); } }}>
          <span class="num">{i + 1}</span>
          <div class="cov">
            {#if track.artwork_url}
              <img src={soundcloudStore.pickArtwork(track.artwork_url)} alt="" loading="lazy" />
            {/if}
          </div>
          <div class="m">
            <span class="t">{track.title}</span>
            <span class="a">{track.user.username}</span>
          </div>
          <span class="d">{fmtDuration(track.duration)}</span>
          <SoundCloudDownloadButton {track} onTrigger={handleDownload} />
        </div>
      {/each}
    </div>
  {/if}
</section>

{#if downloadTrack}
  <SoundCloudDownloadDialog track={downloadTrack} onClose={() => (downloadTrack = null)} />
{/if}

<style>
  .page { display: flex; flex-direction: column; gap: 24px; color: rgba(255,255,255,0.95); }
  .back { align-self: flex-start; display: inline-flex; align-items: center; gap: 8px; padding: 6px 12px 6px 8px; background: rgba(255,255,255,0.05); border: 0; border-radius: 999px; color: rgba(255,255,255,0.85); font-family: inherit; font-size: 12px; font-weight: 600; cursor: pointer; }
  .back:hover { background: rgba(255,255,255,0.1); }
  .hero { display: flex; gap: 32px; align-items: flex-end; padding: 24px 0; }
  .cover { width: 220px; height: 220px; border-radius: 8px; overflow: hidden; flex-shrink: 0; box-shadow: 0 4px 60px rgba(0,0,0,0.5); }
  .cover img { width: 100%; height: 100%; object-fit: cover; }
  .cover-fb { width: 100%; height: 100%; }
  .info { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .eyebrow { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em; color: rgba(255,255,255,0.6); }
  .info h1 { margin: 0; font-size: clamp(28px,4vw,56px); font-weight: 900; letter-spacing: -0.02em; line-height: 1.05; }
  .meta { margin: 0; color: rgba(255,255,255,0.5); font-size: 13px; }
  .desc { margin: 0; color: rgba(255,255,255,0.4); font-size: 13px; max-width: 60ch; }
  .actions { margin-top: 8px; display: flex; gap: 12px; }
  .play-btn { display: inline-flex; align-items: center; gap: 8px; padding: 12px 32px; background: #ff5500; color: white; border: 0; border-radius: 999px; font-family: inherit; font-size: 14px; font-weight: 700; cursor: pointer; transition: background 200ms ease, transform 200ms ease; }
  .play-btn:hover:not(:disabled) { background: #ff7733; transform: scale(1.04); }
  .play-btn:disabled { opacity: 0.5; cursor: default; }
  .bulk-btn { display: inline-flex; align-items: center; gap: 8px; padding: 12px 22px; background: rgba(255,255,255,0.06); color: rgba(255,255,255,0.92); border: 1px solid rgba(255,255,255,0.12); border-radius: 999px; font-family: inherit; font-size: 13px; font-weight: 600; cursor: pointer; transition: background 200ms ease; }
  .bulk-btn:hover:not(:disabled) { background: rgba(255,255,255,0.12); }
  .bulk-btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .actions { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; }
  .track-list { display: flex; flex-direction: column; gap: 2px; }
  .track-row { display: grid; grid-template-columns: 28px 56px 1fr 60px 32px; gap: 12px; align-items: center; padding: 8px 12px; border-radius: 6px; cursor: pointer; transition: background 200ms ease; }
  .track-row:hover { background: rgba(255,255,255,0.06); }
  .num { color: rgba(255,255,255,0.5); font-size: 12px; text-align: center; }
  .cov { width: 44px; height: 44px; border-radius: 4px; overflow: hidden; background: rgba(40,40,40,0.8); }
  .cov img { width: 100%; height: 100%; object-fit: cover; }
  .m { display: flex; flex-direction: column; min-width: 0; }
  .t { font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .a { font-size: 12px; color: rgba(255,255,255,0.55); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .d { font-size: 13px; color: rgba(255,255,255,0.55); text-align: right; font-variant-numeric: tabular-nums; }
  .muted { color: rgba(255,255,255,0.5); font-size: 13px; }
</style>
