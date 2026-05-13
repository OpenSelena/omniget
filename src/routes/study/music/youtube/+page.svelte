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
  import SpeedDialGridItem from "$lib/study-music-components/SpeedDialGridItem.svelte";
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
  <header class="page-head">
    <h1>{$t("study.music.youtube.hub_title")}</h1>
    <p class="tagline">{$t("study.music.youtube.hub_tagline")}</p>
  </header>

  {#if authDerived === "loading"}
    <div class="boot-skel">
      <YoutubeSkeleton kind="card" count={3} />
    </div>
  {:else if authDerived !== "ready"}
    <div class="speed-dial-row anon">
      <SpeedDialGridItem label={$t("study.music.speed_dial_search") as string} href="/study/music/youtube/search">
        {#snippet icon()}
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
        {/snippet}
      </SpeedDialGridItem>
      <SpeedDialGridItem label={$t("study.music.youtube.explore_title") as string} href="/study/music/youtube/explore">
        {#snippet icon()}
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76"/></svg>
        {/snippet}
      </SpeedDialGridItem>
    </div>

    <YoutubeConnectHero status={authDerived as "missing" | "no_youtube"} filePath={authStatus?.file_path ?? ""} />
  {:else}
    <div class="speed-dial-row">
      <SpeedDialGridItem label={$t("study.music.speed_dial_search") as string} href="/study/music/youtube/search">
        {#snippet icon()}
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
        {/snippet}
      </SpeedDialGridItem>
      <SpeedDialGridItem label={$t("study.music.youtube.explore_title") as string} href="/study/music/youtube/explore">
        {#snippet icon()}
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76"/></svg>
        {/snippet}
      </SpeedDialGridItem>
      <SpeedDialGridItem label={$t("study.music.speed_dial_history") as string} href="/study/music/youtube/history">
        {#snippet icon()}
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 1 0 9-9 9.74 9.74 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/></svg>
        {/snippet}
      </SpeedDialGridItem>
      <SpeedDialGridItem label={$t("study.music.youtube.shelf_subscriptions") as string} href="/study/music/youtube/explore">
        {#snippet icon()}
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 19h16"/><path d="M6 15h12"/><rect x="3" y="3" width="18" height="9" rx="2"/></svg>
        {/snippet}
      </SpeedDialGridItem>
      <SpeedDialGridItem label={$t("study.music.my_playlists_title") as string} href="/study/music/youtube/playlists">
        {#snippet icon()}
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 15V6"/><path d="M3 18V9"/><path d="M3 18a3 3 0 1 0 6 0V9a3 3 0 1 0-6 0"/><path d="M15 6a3 3 0 1 0 6 0 3 3 0 1 0-6 0"/></svg>
        {/snippet}
      </SpeedDialGridItem>
      <SpeedDialGridItem label={$t("study.music.speed_dial_trending") as string} href="/study/music/youtube/explore">
        {#snippet icon()}
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m23 6-9.5 9.5-5-5L1 18"/><path d="M17 6h6v6"/></svg>
        {/snippet}
      </SpeedDialGridItem>
    </div>

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
    gap: 28px;
    color: rgba(255, 255, 255, 0.95);
  }
  .page-head h1 {
    margin: 0;
    font-size: clamp(28px, 3.5vw, 40px);
    font-weight: 900;
    letter-spacing: -0.02em;
    color: white;
  }
  .tagline {
    margin: 4px 0 0;
    color: rgba(255, 255, 255, 0.55);
    font-size: 14px;
  }
  .speed-dial-row {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 10px;
  }
  .speed-dial-row.anon {
    grid-template-columns: repeat(2, 1fr);
    max-width: 480px;
  }
  @media (max-width: 760px) {
    .speed-dial-row {
      grid-template-columns: repeat(3, 1fr);
    }
    .speed-dial-row.anon {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  .boot-skel {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 14px;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .seed-hint {
    margin: -4px 0 0;
    color: rgba(255, 255, 255, 0.55);
    font-size: 13px;
  }
  .h-scroll {
    display: flex;
    gap: 18px;
    overflow-x: auto;
    overflow-y: visible;
    padding: 4px 2px 16px;
    scrollbar-width: none;
    scroll-behavior: smooth;
  }
  .h-scroll::-webkit-scrollbar { display: none; }

  .continue-card,
  .discover-card {
    flex: 0 0 auto;
    background: transparent;
    border: 0;
    padding: 0;
    text-align: left;
    color: inherit;
    cursor: pointer;
    font: inherit;
  }
  .continue-card { width: 160px; }
  .discover-card { width: 176px; }

  .cover {
    position: relative;
    aspect-ratio: 1 / 1;
    border-radius: 12px;
    overflow: hidden;
    background: rgba(40, 40, 40, 0.6);
    margin-bottom: 10px;
    transition: transform 200ms ease;
  }
  .continue-card:hover .cover,
  .discover-card:hover .cover {
    transform: scale(1.02);
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .cover-fallback {
    width: 100%;
    height: 100%;
  }
  .progress-track {
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: 6px;
    height: 3px;
    background: rgba(0, 0, 0, 0.5);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: inherit;
  }
  .card-title {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: white;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 2px;
  }
  .card-sub {
    margin: 2px 0 0;
    font-size: 11px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 2px;
  }
  @media (prefers-reduced-motion: reduce) {
    .cover, .continue-card:hover .cover, .discover-card:hover .cover {
      transition: none;
      transform: none;
    }
  }
</style>
