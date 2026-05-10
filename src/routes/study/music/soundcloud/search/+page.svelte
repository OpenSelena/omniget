<script lang="ts">
  import { onMount } from "svelte";
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

  type Tab = "tracks" | "playlists" | "users";

  let query = $state("");
  let tab = $state<Tab>("tracks");
  let inputRef = $state<HTMLInputElement | null>(null);

  let tracks = $state<ScTrack[]>([]);
  let playlists = $state<ScPlaylist[]>([]);
  let users = $state<ScUser[]>([]);
  let loading = $state(false);
  let downloadTrack = $state<ScTrack | null>(null);
  let timer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    if (!soundcloudStore.isLoggedIn) {
      goto("/study/music/soundcloud");
      return;
    }
    queueMicrotask(() => inputRef?.focus());
  });

  function onInput() {
    if (timer) clearTimeout(timer);
    if (!query.trim()) {
      tracks = [];
      playlists = [];
      users = [];
      return;
    }
    timer = setTimeout(() => doSearch(), 350);
  }

  async function doSearch() {
    const q = query.trim();
    if (!q) return;
    loading = true;
    try {
      const [t, p, u] = await Promise.allSettled([
        soundcloudStore.search("tracks", q, 30),
        soundcloudStore.search("playlists", q, 20),
        soundcloudStore.search("users", q, 20),
      ]);
      tracks = (t.status === "fulfilled" ? (t.value as ScTrack[]) : []) ?? [];
      playlists = (p.status === "fulfilled" ? (p.value as ScPlaylist[]) : []) ?? [];
      users = (u.status === "fulfilled" ? (u.value as ScUser[]) : []) ?? [];
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      loading = false;
    }
  }

  async function play(track: ScTrack, queueAll: ScTrack[]) {
    try {
      await soundcloudStore.playTrack(track, queueAll);
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
  <header class="head">
    <button type="button" class="back" onclick={() => goto("/study/music/soundcloud")}>
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="15 18 9 12 15 6"/></svg>
      SoundCloud
    </button>
  </header>

  <div class="search-bar">
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <circle cx="11" cy="11" r="8"/>
      <line x1="21" y1="21" x2="16.65" y2="16.65"/>
    </svg>
    <input
      bind:this={inputRef}
      bind:value={query}
      oninput={onInput}
      type="search"
      placeholder="Buscar no SoundCloud…"
      autocomplete="off"
    />
    {#if loading}<span class="spinner"></span>{/if}
  </div>

  {#if tracks.length || playlists.length || users.length}
    <div class="tabs" role="tablist">
      <button type="button" class="tab" class:on={tab === "tracks"} onclick={() => (tab = "tracks")} role="tab" aria-selected={tab === "tracks"}>Faixas {tracks.length > 0 ? `(${tracks.length})` : ""}</button>
      <button type="button" class="tab" class:on={tab === "playlists"} onclick={() => (tab = "playlists")} role="tab" aria-selected={tab === "playlists"}>Playlists {playlists.length > 0 ? `(${playlists.length})` : ""}</button>
      <button type="button" class="tab" class:on={tab === "users"} onclick={() => (tab = "users")} role="tab" aria-selected={tab === "users"}>Artistas {users.length > 0 ? `(${users.length})` : ""}</button>
    </div>
  {/if}

  {#if tab === "tracks" && tracks.length > 0}
    <div class="track-list">
      {#each tracks as t, i (t.id)}
        <div class="track-row" role="button" tabindex="0" onclick={() => play(t, tracks)} onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); play(t, tracks); } }}>
          <div class="cov">
            {#if t.artwork_url}<img src={soundcloudStore.pickArtwork(t.artwork_url)} alt="" loading="lazy" />{/if}
          </div>
          <div class="m">
            <span class="t">{t.title}</span>
            <span class="a">{t.user.username}</span>
          </div>
          <span class="d">{fmtDuration(t.duration)}</span>
          <button type="button" class="dl" onclick={(e) => { e.stopPropagation(); downloadTrack = t; }} aria-label="Baixar">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
          </button>
        </div>
      {/each}
    </div>
  {:else if tab === "playlists" && playlists.length > 0}
    <div class="grid">
      {#each playlists as p (p.id)}
        <a class="card" href={`/study/music/soundcloud/playlist/${p.id}`}>
          <div class="card-cover">
            {#if p.artwork_url}<img src={soundcloudStore.pickArtwork(p.artwork_url)} alt="" loading="lazy" />{:else}<div class="cover-fb" style:background={colorFromString(p.title)}></div>{/if}
          </div>
          <h3 class="ct">{p.title}</h3>
          <p class="cs">{p.track_count} faixas · {p.user.username}</p>
        </a>
      {/each}
    </div>
  {:else if tab === "users" && users.length > 0}
    <div class="grid">
      {#each users as u (u.id)}
        <a class="card user-card" href={`/study/music/soundcloud/user/${u.id}`}>
          <div class="card-circle">
            {#if u.avatar_url}<img src={soundcloudStore.pickArtwork(u.avatar_url)} alt="" loading="lazy" />{:else}<div class="cover-fb round" style:background={colorFromString(u.username)}>{u.username.slice(0,1).toUpperCase()}</div>{/if}
          </div>
          <h3 class="ct center">{u.username}</h3>
          {#if u.followers_count}<p class="cs center">{u.followers_count.toLocaleString("pt-BR")} seguidores</p>{/if}
        </a>
      {/each}
    </div>
  {:else if query.trim() && !loading}
    <p class="muted">Nada encontrado para "{query}"</p>
  {/if}
</section>

{#if downloadTrack}
  <SoundCloudDownloadDialog track={downloadTrack} onClose={() => (downloadTrack = null)} />
{/if}

<style>
  .page { display: flex; flex-direction: column; gap: 24px; color: rgba(255,255,255,0.95); }
  .head { display: flex; }
  .back { display: inline-flex; align-items: center; gap: 8px; padding: 6px 12px 6px 8px; background: rgba(255,255,255,0.05); border: 0; border-radius: 999px; color: rgba(255,255,255,0.85); font-family: inherit; font-size: 12px; font-weight: 600; cursor: pointer; }
  .back:hover { background: rgba(255,255,255,0.1); }
  .search-bar { position: relative; display: flex; align-items: center; gap: 12px; background: rgba(255,255,255,0.06); border-radius: 999px; padding: 12px 20px; color: rgba(255,255,255,0.6); }
  .search-bar input { flex: 1; background: transparent; border: 0; outline: 0; color: white; font-family: inherit; font-size: 15px; }
  .spinner { width: 14px; height: 14px; border: 2px solid rgba(255,255,255,0.2); border-top-color: #ff5500; border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .tabs { display: flex; gap: 8px; border-bottom: 1px solid rgba(255,255,255,0.08); }
  .tab { padding: 8px 16px; background: transparent; border: 0; border-bottom: 2px solid transparent; color: rgba(255,255,255,0.55); font-family: inherit; font-size: 13px; font-weight: 600; cursor: pointer; transition: color 200ms ease, border-color 200ms ease; }
  .tab:hover { color: white; }
  .tab.on { color: white; border-bottom-color: #ff5500; }
  .track-list { display: flex; flex-direction: column; gap: 2px; }
  .track-row { display: grid; grid-template-columns: 56px 1fr 60px 32px; gap: 12px; align-items: center; padding: 8px 12px; border-radius: 6px; cursor: pointer; transition: background 200ms ease; }
  .track-row:hover { background: rgba(255,255,255,0.06); }
  .cov { width: 44px; height: 44px; border-radius: 4px; overflow: hidden; background: rgba(40,40,40,0.8); }
  .cov img { width: 100%; height: 100%; object-fit: cover; }
  .m { display: flex; flex-direction: column; min-width: 0; }
  .t { font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .a { font-size: 12px; color: rgba(255,255,255,0.55); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .d { font-size: 13px; color: rgba(255,255,255,0.55); text-align: right; font-variant-numeric: tabular-nums; }
  .dl { width: 28px; height: 28px; padding: 0; background: transparent; border: 0; color: rgba(255,255,255,0.5); cursor: pointer; display: grid; place-items: center; border-radius: 4px; transition: background 200ms ease, color 200ms ease; }
  .dl:hover { background: rgba(255,255,255,0.1); color: #ff5500; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 24px; }
  .card { background: transparent; border: 0; padding: 0; cursor: pointer; color: inherit; text-align: left; text-decoration: none; }
  .card-cover, .card-circle { aspect-ratio: 1/1; border-radius: 12px; overflow: hidden; margin-bottom: 12px; background: rgba(40,40,40,0.8); }
  .card-circle { border-radius: 50%; }
  .card-cover img, .card-circle img { width: 100%; height: 100%; object-fit: cover; transition: transform 500ms ease; }
  .card:hover .card-cover img, .card:hover .card-circle img { transform: scale(1.05); }
  .cover-fb { width: 100%; height: 100%; }
  .cover-fb.round { border-radius: 50%; display: grid; place-items: center; color: white; font-size: 32px; font-weight: 800; }
  .ct { margin: 0; font-size: 14px; font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 0 4px; }
  .ct.center { text-align: center; }
  .cs { margin: 4px 0 0; font-size: 12px; color: rgba(255,255,255,0.5); padding: 0 4px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cs.center { text-align: center; }
  .user-card { text-align: center; }
  .muted { color: rgba(255,255,255,0.5); font-size: 13px; text-align: center; padding: 32px 0; }
</style>
