<script lang="ts">
  import { onMount } from "svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";
  import { colorFromString } from "$lib/study-music/format";
  import { playYoutubeVideoId, playYoutubeVideoItem } from "$lib/study-music/youtube-play-helper";
  import {
    studyMusicQuickPicks,
    studyMusicDailyDiscover,
    studyMusicContinueListening,
    studyYoutubeTrending,
    studyYoutubeSubsFeed,
    studyMusicLoopCookieStatus,
    type MusicQuickPickEntry,
    type MusicDiscoverEntry,
    type MusicContinueEntry,
    type YoutubeSearchVideoItem,
    type LoopCookieStatus,
  } from "$lib/study-bridge";
  import NavigationTitle from "$lib/study-music-components/NavigationTitle.svelte";
  import EmptyPlaceholder from "$lib/study-music-components/EmptyPlaceholder.svelte";
  import YoutubeSkeleton from "$lib/study-music-youtube-components/YoutubeSkeleton.svelte";
  import YoutubeConnectHero from "$lib/study-music-youtube-components/YoutubeConnectHero.svelte";

  let authStatus = $state<LoopCookieStatus | null>(null);
  let bootingAuth = $state(true);
  let authDerived = $derived(
    authStatus === null
      ? "loading"
      : authStatus.available && authStatus.has_youtube
        ? "ready"
        : authStatus.available
          ? "no_youtube"
          : "missing",
  );

  let quickPicks = $state<MusicQuickPickEntry[]>([]);
  let discover = $state<MusicDiscoverEntry[]>([]);
  let discoverSeedTitle = $state<string | null>(null);
  let continueWatching = $state<MusicContinueEntry[]>([]);
  let trending = $state<YoutubeSearchVideoItem[]>([]);
  let subsFeed = $state<YoutubeSearchVideoItem[]>([]);

  let loadingPicks = $state(true);
  let loadingDiscover = $state(true);
  let loadingContinue = $state(true);
  let loadingTrending = $state(true);
  let loadingSubs = $state(true);

  async function loadAuth() {
    bootingAuth = true;
    try {
      authStatus = await studyMusicLoopCookieStatus();
    } catch {
      authStatus = { available: false, file_path: "", has_youtube: false };
    } finally {
      bootingAuth = false;
    }
  }

  async function loadQuickPicks() {
    loadingPicks = true;
    try {
      const res = await studyMusicQuickPicks();
      quickPicks = (res.entries ?? []).filter((e) => e.source === "youtube");
    } catch {
      quickPicks = [];
    } finally {
      loadingPicks = false;
    }
  }

  async function loadDiscover() {
    loadingDiscover = true;
    try {
      const res = await studyMusicDailyDiscover();
      discover = res.entries ?? [];
      discoverSeedTitle = res.seed_title ?? null;
    } catch {
      discover = [];
    } finally {
      loadingDiscover = false;
    }
  }

  async function loadContinueWatching() {
    loadingContinue = true;
    try {
      const res = await studyMusicContinueListening({ limit: 16 });
      continueWatching = (res.entries ?? [])
        .filter((e) => e.source === "youtube")
        .slice(0, 8);
    } catch {
      continueWatching = [];
    } finally {
      loadingContinue = false;
    }
  }

  async function loadTrending() {
    loadingTrending = true;
    try {
      const res = await studyYoutubeTrending({ category: "now" });
      const flat: YoutubeSearchVideoItem[] = [];
      for (const it of res.items ?? []) {
        if (it.kind === "shelf") {
          for (const s of it.items) if (s.kind === "video") flat.push(s);
        } else if (it.kind === "video") {
          flat.push(it);
        }
      }
      trending = flat.slice(0, 16);
    } catch {
      trending = [];
    } finally {
      loadingTrending = false;
    }
  }

  async function loadSubs() {
    loadingSubs = true;
    try {
      const res = await studyYoutubeSubsFeed();
      const items: YoutubeSearchVideoItem[] = [];
      for (const e of res.entries ?? []) {
        for (const it of e.items.slice(0, 4)) {
          if (it.kind === "video") items.push(it);
        }
      }
      subsFeed = items.slice(0, 12);
    } catch {
      subsFeed = [];
    } finally {
      loadingSubs = false;
    }
  }

  async function playQuickPick(entry: MusicQuickPickEntry) {
    if (!entry.external_id) return;
    try {
      await playYoutubeVideoId(
        entry.external_id,
        entry.title,
        entry.artist,
        entry.cover_url,
      );
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function playDiscover(entry: MusicDiscoverEntry) {
    try {
      await playYoutubeVideoId(
        entry.external_id,
        entry.title,
        entry.artist,
        entry.cover_url,
      );
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function playContinue(entry: MusicContinueEntry) {
    if (!entry.external_id) return;
    try {
      await playYoutubeVideoId(
        entry.external_id,
        entry.title,
        entry.artist,
        entry.cover_url,
      );
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function playVideo(item: YoutubeSearchVideoItem, queue: YoutubeSearchVideoItem[]) {
    try {
      await playYoutubeVideoItem(item, queue);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  onMount(async () => {
    await loadAuth();
    void loadTrending();
    if (authDerived === "ready") {
      void loadContinueWatching();
      void loadQuickPicks();
      void loadDiscover();
      void loadSubs();
    }
  });
</script>

<section class="yt-hub">
  <header class="yt-topbar">
    <h1 class="yt-brand">{$t("study.music.youtube.hub_title")}</h1>
    <a
      class="yt-search"
      href="/study/music/youtube/search"
      aria-label={$t("study.music.speed_dial_search") as string}
    >
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
      <span>{$t("study.music.speed_dial_search")}</span>
    </a>
  </header>

  {#if authDerived === "loading"}
    <div class="boot-skel">
      <YoutubeSkeleton kind="card" count={6} />
    </div>
  {:else if authDerived !== "ready"}
    <nav class="yt-chips" aria-label={$t("study.music.youtube.hub_title") as string}>
      <a class="yt-chip" href="/study/music/youtube/search">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
        <span>{$t("study.music.speed_dial_search")}</span>
      </a>
      <a class="yt-chip" href="/study/music/youtube/explore">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76"/></svg>
        <span>{$t("study.music.youtube.explore_title")}</span>
      </a>
    </nav>

    <YoutubeConnectHero status={authDerived as "missing" | "no_youtube"} filePath={authStatus?.file_path ?? ""} />
  {:else}
    <nav class="yt-chips" aria-label={$t("study.music.youtube.hub_title") as string}>
      <a class="yt-chip" href="/study/music/youtube/search">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
        <span>{$t("study.music.speed_dial_search")}</span>
      </a>
      <a class="yt-chip" href="/study/music/youtube/explore">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76"/></svg>
        <span>{$t("study.music.youtube.explore_title")}</span>
      </a>
      <a class="yt-chip" href="/study/music/youtube/history">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 1 0 9-9 9.74 9.74 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/></svg>
        <span>{$t("study.music.speed_dial_history")}</span>
      </a>
      <a class="yt-chip" href="/study/music/youtube/explore">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 19h16"/><path d="M6 15h12"/><rect x="3" y="3" width="18" height="9" rx="2"/></svg>
        <span>{$t("study.music.youtube.shelf_subscriptions")}</span>
      </a>
      <a class="yt-chip" href="/study/music/youtube/playlists">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 15V6"/><path d="M3 18V9"/><path d="M3 18a3 3 0 1 0 6 0V9a3 3 0 1 0-6 0"/><path d="M15 6a3 3 0 1 0 6 0 3 3 0 1 0-6 0"/></svg>
        <span>{$t("study.music.my_playlists_title")}</span>
      </a>
      <a class="yt-chip" href="/study/music/youtube/explore">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m23 6-9.5 9.5-5-5L1 18"/><path d="M17 6h6v6"/></svg>
        <span>{$t("study.music.speed_dial_trending")}</span>
      </a>
    </nav>

  <section class="block">
    <NavigationTitle title={$t("study.music.youtube.shelf_continue_watching") as string} seeAllHref="/study/music/youtube/history" />
    {#if loadingContinue}
      <YoutubeSkeleton kind="row" count={4} />
    {:else if continueWatching.length === 0}
      <EmptyPlaceholder title={$t("study.music.youtube.shelf_continue_watching_empty") as string} compact />
    {:else}
      <div class="h-scroll">
        {#each continueWatching as entry (entry.external_id ?? entry.title)}
          <button
            type="button"
            class="continue-card"
            onclick={() => playContinue(entry)}
          >
            <div class="cover">
              {#if entry.cover_url}
                <img src={entry.cover_url} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(entry.title)}></div>
              {/if}
              <div class="progress-track">
                <div class="progress-fill" style:width={`${Math.round((entry.progress ?? 0) * 100)}%`}></div>
              </div>
            </div>
            <h3 class="card-title">{entry.title}</h3>
            {#if entry.artist}<p class="card-sub">{entry.artist}</p>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="block">
    <NavigationTitle title={$t("study.music.youtube.shelf_quick_picks") as string} />
    {#if loadingPicks}
      <YoutubeSkeleton kind="card" count={6} />
    {:else if quickPicks.length === 0}
      <EmptyPlaceholder title={$t("study.music.youtube.shelf_quick_picks_empty") as string} compact />
    {:else}
      <div class="h-scroll">
        {#each quickPicks as entry, idx (entry.external_id ?? entry.title + idx)}
          <button
            type="button"
            class="discover-card"
            onclick={() => playQuickPick(entry)}
          >
            <div class="cover">
              {#if entry.cover_url}
                <img src={entry.cover_url} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(entry.title)}></div>
              {/if}
            </div>
            <h3 class="card-title">{entry.title}</h3>
            {#if entry.artist}<p class="card-sub">{entry.artist}</p>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="block">
    <NavigationTitle title={$t("study.music.youtube.shelf_daily_discover") as string} />
    {#if discoverSeedTitle}
      <p class="seed-hint">{discoverSeedTitle}</p>
    {/if}
    {#if loadingDiscover}
      <YoutubeSkeleton kind="card" count={6} />
    {:else if discover.length === 0}
      <EmptyPlaceholder title={$t("study.music.youtube.shelf_daily_discover_empty") as string} compact />
    {:else}
      <div class="h-scroll">
        {#each discover as entry, idx (entry.external_id + idx)}
          <button
            type="button"
            class="discover-card"
            onclick={() => playDiscover(entry)}
          >
            <div class="cover">
              {#if entry.cover_url}
                <img src={entry.cover_url} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(entry.title)}></div>
              {/if}
            </div>
            <h3 class="card-title">{entry.title}</h3>
            {#if entry.artist}<p class="card-sub">{entry.artist}</p>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="block">
    <NavigationTitle title={$t("study.music.youtube.shelf_trending") as string} seeAllHref="/study/music/youtube/explore" />
    {#if loadingTrending}
      <YoutubeSkeleton kind="card" count={6} />
    {:else if trending.length === 0}
      <EmptyPlaceholder title={$t("study.music.youtube.shelf_trending_error") as string} compact />
    {:else}
      <div class="h-scroll">
        {#each trending as item (item.video_id)}
          <button
            type="button"
            class="discover-card"
            onclick={() => playVideo(item, trending)}
          >
            <div class="cover">
              {#if item.thumbnail_url}
                <img src={item.thumbnail_url} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(item.title)}></div>
              {/if}
            </div>
            <h3 class="card-title">{item.title}</h3>
            {#if item.channel_title}<p class="card-sub">{item.channel_title}</p>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="block">
    <NavigationTitle title={$t("study.music.youtube.shelf_subscriptions") as string} />
    {#if loadingSubs}
      <YoutubeSkeleton kind="card" count={6} />
    {:else if subsFeed.length === 0}
      <EmptyPlaceholder title={$t("study.music.youtube.shelf_subscriptions_empty") as string} compact />
    {:else}
      <div class="h-scroll">
        {#each subsFeed as item (item.video_id)}
          <button
            type="button"
            class="discover-card"
            onclick={() => playVideo(item, subsFeed)}
          >
            <div class="cover">
              {#if item.thumbnail_url}
                <img src={item.thumbnail_url} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(item.title)}></div>
              {/if}
            </div>
            <h3 class="card-title">{item.title}</h3>
            {#if item.channel_title}<p class="card-sub">{item.channel_title}</p>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </section>
  {/if}
</section>

<style>
  .yt-hub {
    display: flex;
    flex-direction: column;
    gap: 22px;
    color: #f1f1f1;
    background: #0f0f0f;
    min-height: 100vh;
    width: 100vw;
    margin-left: calc(50% - 50vw);
    margin-top: -16px;
    margin-bottom: -16px;
    padding: 0 0 56px;
    font-family:
      "Roboto", "YouTube Sans", system-ui, -apple-system, "Segoe UI", sans-serif;
  }
  .yt-topbar {
    position: sticky;
    top: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 12px 24px;
    background: rgba(15, 15, 15, 0.92);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }
  .yt-brand {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: #fff;
    white-space: nowrap;
  }
  .yt-search {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    max-width: 640px;
    padding: 9px 18px;
    border-radius: 999px;
    background: #121212;
    border: 1px solid rgba(255, 255, 255, 0.14);
    color: #aaa;
    font-size: 14px;
    text-decoration: none;
    transition: border-color 140ms ease, background 140ms ease;
  }
  .yt-search:hover {
    border-color: rgba(255, 255, 255, 0.3);
    background: #1a1a1a;
  }
  .yt-search svg { color: #aaa; flex: 0 0 auto; }
  .yt-chips {
    display: flex;
    gap: 10px;
    overflow-x: auto;
    padding: 4px 24px 6px;
    scrollbar-width: none;
  }
  .yt-chips::-webkit-scrollbar { display: none; }
  .yt-chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    padding: 8px 14px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.08);
    color: #f1f1f1;
    font-size: 13px;
    font-weight: 500;
    text-decoration: none;
    white-space: nowrap;
    transition: background 140ms ease;
  }
  .yt-chip svg { color: #f1f1f1; }
  .yt-chip:hover { background: rgba(255, 255, 255, 0.16); }
  .yt-chip:active { background: rgba(255, 255, 255, 0.22); }
  .boot-skel {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 16px;
    padding: 8px 24px 0;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 0 24px;
  }
  .seed-hint {
    margin: -4px 0 0;
    color: rgba(255, 255, 255, 0.55);
    font-size: 13px;
  }
  .h-scroll {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(260px, 1fr);
    grid-template-rows: 1fr;
    gap: 16px;
    overflow-x: auto;
    overflow-y: visible;
    padding: 2px 2px 14px;
    scrollbar-width: thin;
    scrollbar-color: #5a5a5a transparent;
    scroll-behavior: smooth;
    scroll-snap-type: x proximity;
  }
  .h-scroll::-webkit-scrollbar { height: 8px; }
  .h-scroll::-webkit-scrollbar-thumb {
    background: #5a5a5a;
    border-radius: 4px;
  }
  .h-scroll::-webkit-scrollbar-track { background: transparent; }

  .continue-card,
  .discover-card {
    flex: 0 0 auto;
    width: 100%;
    background: transparent;
    border: 0;
    padding: 0;
    text-align: left;
    color: inherit;
    cursor: pointer;
    font: inherit;
    scroll-snap-align: start;
  }

  .cover {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: 12px;
    overflow: hidden;
    background: #272727;
    margin-bottom: 10px;
  }
  .cover img,
  .cover-fallback {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: transform 220ms ease;
  }
  .continue-card:hover .cover img,
  .discover-card:hover .cover img {
    transform: scale(1.045);
  }
  .cover::after {
    content: "";
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0);
    transition: background 160ms ease;
  }
  .continue-card:hover .cover::after,
  .discover-card:hover .cover::after {
    background: rgba(0, 0, 0, 0.12);
  }
  .progress-track {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 4px;
    background: rgba(255, 255, 255, 0.25);
    overflow: hidden;
    z-index: 1;
  }
  .progress-fill {
    height: 100%;
    background: #f00;
    border-radius: 0;
  }
  .card-title {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: #f1f1f1;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    padding: 0;
  }
  .continue-card:hover .card-title,
  .discover-card:hover .card-title {
    color: #fff;
  }
  .card-sub {
    margin: 4px 0 0;
    font-size: 12.5px;
    font-weight: 400;
    color: #aaa;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .cover img,
    .continue-card:hover .cover img,
    .discover-card:hover .cover img {
      transition: none;
      transform: none;
    }
  }
</style>
