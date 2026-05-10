<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import {
    soundcloudStore,
    type ScPlaylist,
    type ScTrack,
    type ScUser,
  } from "$lib/study-music/soundcloud-store.svelte";
  import { colorFromString } from "$lib/study-music/format";
  import SoundCloudDownloadDialog from "$lib/study-music-components/SoundCloudDownloadDialog.svelte";

  let user = $state<ScUser | null>(null);
  let tracks = $state<ScTrack[]>([]);
  let playlists = $state<ScPlaylist[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let downloadTrack = $state<ScTrack | null>(null);

  const userId = $derived(parseInt($page.params.id ?? "0", 10));

  onMount(async () => {
    if (!userId) {
      goto("/study/music/soundcloud");
      return;
    }
    try {
      const [u, t, p] = await Promise.allSettled([
        soundcloudStore.getUser(userId),
        soundcloudStore.getUserTracks(userId, 50),
        soundcloudStore.getUserPlaylists(userId, 30),
      ]);
      if (u.status === "fulfilled") user = u.value;
      if (t.status === "fulfilled") tracks = t.value;
      if (p.status === "fulfilled") playlists = p.value;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });

  async function play(idx: number) {
    if (tracks.length === 0) return;
    const reordered = [...tracks.slice(idx), ...tracks.slice(0, idx)];
    try {
      await soundcloudStore.playTrack(reordered[0], reordered);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  function fmtDuration(ms: number): string {
    const total = Math.floor(ms / 1000);
    const m = Math.floor(total / 60);
    const s = total % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  }
</script>

<section class="page">
  <button type="button" class="back" onclick={() => history.back()}>
    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <polyline points="15 18 9 12 15 6"/>
    </svg>
    Voltar
  </button>

  {#if user}
    <header class="hero">
      <div class="circle">
        {#if user.avatar_url}
          <img src={soundcloudStore.pickArtwork(user.avatar_url)} alt="" />
        {:else}
          <div class="circle-fb" style:background={colorFromString(user.username)}>{user.username.slice(0,1).toUpperCase()}</div>
        {/if}
      </div>
      <div class="info">
        <span class="eyebrow">Artista</span>
        <h1>{user.username}</h1>
        <p class="meta">
          {#if user.followers_count}{user.followers_count.toLocaleString("pt-BR")} seguidores{/if}
          {#if user.city}{user.city}{user.country_code ? `, ${user.country_code}` : ""}{/if}
        </p>
        {#if user.description}
          <p class="desc">{user.description}</p>
        {/if}
        <div class="actions">
          <button type="button" class="play-btn" onclick={() => play(0)} disabled={tracks.length === 0}>
            <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>
            Tocar
          </button>
        </div>
      </div>
    </header>
  {/if}

  {#if loading}
    <p class="muted">Carregando…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else}
    {#if tracks.length > 0}
      <section class="block">
        <header class="block-head"><h2>Faixas ({tracks.length})</h2></header>
        <div class="track-list">
          {#each tracks.slice(0, 20) as track, i (track.id)}
            <div class="track-row" role="button" tabindex="0" onclick={() => play(i)} onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); play(i); } }}>
              <span class="num">{i + 1}</span>
              <div class="cov">
                {#if track.artwork_url}
                  <img src={soundcloudStore.pickArtwork(track.artwork_url)} alt="" loading="lazy" />
                {/if}
              </div>
              <div class="m">
                <span class="t">{track.title}</span>
                {#if track.likes_count}<span class="a">♥ {track.likes_count.toLocaleString("pt-BR")}</span>{/if}
              </div>
              <span class="d">{fmtDuration(track.duration)}</span>
              <button type="button" class="dl" onclick={(e) => { e.stopPropagation(); downloadTrack = track; }} aria-label="Baixar">
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
              </button>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    {#if playlists.length > 0}
      <section class="block">
        <header class="block-head"><h2>Playlists</h2></header>
        <div class="grid">
          {#each playlists as p (p.id)}
            <a class="card" href={`/study/music/soundcloud/playlist/${p.id}`}>
              <div class="card-cover">
                {#if p.artwork_url}
                  <img src={soundcloudStore.pickArtwork(p.artwork_url)} alt="" loading="lazy" />
                {:else}
                  <div class="cover-fb" style:background={colorFromString(p.title)}></div>
                {/if}
              </div>
              <h3 class="card-title">{p.title}</h3>
              <p class="card-sub">{p.track_count} faixas</p>
            </a>
          {/each}
        </div>
      </section>
    {/if}
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
  .circle { width: 220px; height: 220px; border-radius: 50%; overflow: hidden; flex-shrink: 0; box-shadow: 0 4px 60px rgba(0,0,0,0.5); }
  .circle img { width: 100%; height: 100%; object-fit: cover; }
  .circle-fb { width: 100%; height: 100%; display: grid; place-items: center; color: white; font-size: 64px; font-weight: 800; }
  .info { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .eyebrow { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em; color: rgba(255,255,255,0.6); }
  .info h1 { margin: 0; font-size: clamp(28px,4vw,56px); font-weight: 900; letter-spacing: -0.02em; line-height: 1.05; }
  .meta { margin: 0; color: rgba(255,255,255,0.5); font-size: 13px; }
  .desc { margin: 0; color: rgba(255,255,255,0.4); font-size: 13px; max-width: 60ch; }
  .actions { margin-top: 8px; display: flex; gap: 12px; }
  .play-btn { display: inline-flex; align-items: center; gap: 8px; padding: 12px 32px; background: #ff5500; color: white; border: 0; border-radius: 999px; font-family: inherit; font-size: 14px; font-weight: 700; cursor: pointer; transition: background 200ms ease, transform 200ms ease; }
  .play-btn:hover:not(:disabled) { background: #ff7733; transform: scale(1.04); }
  .play-btn:disabled { opacity: 0.5; cursor: default; }
  .block { display: flex; flex-direction: column; gap: 16px; }
  .block-head h2 { margin: 0; font-size: 22px; font-weight: 800; }
  .track-list { display: flex; flex-direction: column; gap: 2px; }
  .track-row { display: grid; grid-template-columns: 28px 56px 1fr 60px 32px; gap: 12px; align-items: center; padding: 8px 12px; border-radius: 6px; cursor: pointer; transition: background 200ms ease; }
  .track-row:hover { background: rgba(255,255,255,0.06); }
  .num { color: rgba(255,255,255,0.5); font-size: 12px; text-align: center; }
  .cov { width: 44px; height: 44px; border-radius: 4px; overflow: hidden; background: rgba(40,40,40,0.8); }
  .cov img { width: 100%; height: 100%; object-fit: cover; }
  .m { display: flex; flex-direction: column; min-width: 0; }
  .t { font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .a { font-size: 12px; color: rgba(255,255,255,0.55); }
  .d { font-size: 13px; color: rgba(255,255,255,0.55); text-align: right; font-variant-numeric: tabular-nums; }
  .dl { width: 28px; height: 28px; padding: 0; background: transparent; border: 0; color: rgba(255,255,255,0.5); cursor: pointer; display: grid; place-items: center; border-radius: 4px; transition: background 200ms ease, color 200ms ease; }
  .dl:hover { background: rgba(255,255,255,0.1); color: #ff5500; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 24px; }
  .card { background: transparent; border: 0; padding: 0; cursor: pointer; color: inherit; text-align: left; text-decoration: none; }
  .card-cover { aspect-ratio: 1/1; border-radius: 12px; overflow: hidden; margin-bottom: 12px; background: rgba(40,40,40,0.8); }
  .card-cover img { width: 100%; height: 100%; object-fit: cover; transition: transform 500ms ease; }
  .card:hover .card-cover img { transform: scale(1.05); }
  .cover-fb { width: 100%; height: 100%; }
  .card-title { margin: 0; font-size: 14px; font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 0 4px; }
  .card-sub { margin: 4px 0 0; font-size: 12px; color: rgba(255,255,255,0.5); padding: 0 4px; }
  .muted { color: rgba(255,255,255,0.5); font-size: 13px; }
  .error { color: #e22134; font-size: 13px; }
</style>
