<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { colorFromString } from "$lib/study-music/format";
  import { playYoutubeVideoId } from "$lib/study-music/youtube-play-helper";
  import {
    studyYoutubeTrending,
    studyYoutubeNewReleases,
    studyYoutubeMoodsAndGenres,
    type YoutubeSearchItem,
    type YoutubeNewReleaseAlbum,
    type YoutubeMoodCategory,
    type YoutubeMoodSection,
  } from "$lib/study-bridge";
  import NavigationTitle from "$lib/study-music-components/NavigationTitle.svelte";
  import EmptyPlaceholder from "$lib/study-music-components/EmptyPlaceholder.svelte";
  import YoutubeSkeleton from "$lib/study-music-youtube-components/YoutubeSkeleton.svelte";

  let chartsItems = $state<YoutubeSearchItem[]>([]);
  let chartsLoading = $state(true);
  let chartsError = $state<string | null>(null);

  let trendingItems = $state<YoutubeSearchItem[]>([]);
  let trendingLoading = $state(true);
  let trendingError = $state<string | null>(null);

  let newReleases = $state<YoutubeNewReleaseAlbum[]>([]);
  let newReleasesLoading = $state(true);
  let newReleasesError = $state<string | null>(null);

  let moodSections = $state<YoutubeMoodSection[]>([]);
  let moodsLoading = $state(true);
  let moodsError = $state<string | null>(null);

  function flattenItems(items: YoutubeSearchItem[]): YoutubeSearchItem[] {
    const out: YoutubeSearchItem[] = [];
    for (const it of items) {
      if (it.kind === "shelf") {
        for (const s of it.items) out.push(s);
      } else {
        out.push(it);
      }
    }
    return out;
  }

  async function loadAll() {
    void loadCharts();
    void loadTrending();
    void loadNewReleases();
    void loadMoods();
  }

  async function loadCharts() {
    chartsLoading = true;
    chartsError = null;
    try {
      const res = await studyYoutubeTrending({ category: "music" });
      chartsItems = flattenItems(res.items ?? []).filter((i) => i.kind === "video").slice(0, 16);
    } catch (e) {
      chartsError = e instanceof Error ? e.message : String(e);
    } finally {
      chartsLoading = false;
    }
  }

  async function loadTrending() {
    trendingLoading = true;
    trendingError = null;
    try {
      const res = await studyYoutubeTrending({ category: "now" });
      trendingItems = flattenItems(res.items ?? []).filter((i) => i.kind === "video").slice(0, 16);
    } catch (e) {
      trendingError = e instanceof Error ? e.message : String(e);
    } finally {
      trendingLoading = false;
    }
  }

  async function loadNewReleases() {
    newReleasesLoading = true;
    newReleasesError = null;
    try {
      const res = await studyYoutubeNewReleases();
      newReleases = (res.albums ?? []).slice(0, 16);
    } catch (e) {
      newReleasesError = e instanceof Error ? e.message : String(e);
    } finally {
      newReleasesLoading = false;
    }
  }

  async function loadMoods() {
    moodsLoading = true;
    moodsError = null;
    try {
      const res = await studyYoutubeMoodsAndGenres();
      moodSections = res.sections ?? [];
    } catch (e) {
      moodsError = e instanceof Error ? e.message : String(e);
    } finally {
      moodsLoading = false;
    }
  }

  async function playVideoItem(item: YoutubeSearchItem) {
    if (item.kind !== "video") return;
    try {
      await playYoutubeVideoId(item.video_id, item.title, item.channel_title, item.thumbnail_url);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  function openAlbum(album: YoutubeNewReleaseAlbum) {
    void goto(`/study/music/youtube/album/${encodeURIComponent(album.browse_id)}`);
  }

  function openMood(mood: YoutubeMoodCategory) {
    const params = mood.params ? `?params=${encodeURIComponent(mood.params)}` : "";
    void goto(`/study/music/youtube/mood/${encodeURIComponent(mood.mood_id)}${params}`);
  }

  function moodSurface(color: string | null): string {
    if (!color) return "rgba(255,255,255,0.06)";
    return `linear-gradient(135deg, ${color}cc 0%, ${color}66 100%)`;
  }

  onMount(() => {
    void loadAll();
  });
</script>

<svelte:head>
  <title>{$t("study.music.youtube.explore_title")}</title>
</svelte:head>

<main class="explore">
  <header class="page-head">
    <button class="back" type="button" onclick={() => goto("/study/music/youtube")}>
      <span aria-hidden="true">←</span>
      <span>{$t("study.music.back")}</span>
    </button>
    <div class="head-text">
      <h1>{$t("study.music.youtube.explore_title")}</h1>
      <p>{$t("study.music.youtube.explore_tagline")}</p>
    </div>
  </header>

  <section class="block">
    <NavigationTitle title={$t("study.music.youtube.shelf_charts")} />
    {#if chartsLoading}
      <YoutubeSkeleton kind="card" count={6} />
    {:else if chartsError || chartsItems.length === 0}
      <EmptyPlaceholder
        title={$t("study.music.youtube.shelf_charts_empty")}
        compact
      />
    {:else}
      <div class="h-scroll">
        {#each chartsItems as item, idx (idx)}
          {#if item.kind === "video"}
            <button type="button" class="card" onclick={() => playVideoItem(item)}>
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
          {/if}
        {/each}
      </div>
    {/if}
  </section>

  <section class="block">
    <NavigationTitle title={$t("study.music.youtube.shelf_new_releases")} />
    {#if newReleasesLoading}
      <YoutubeSkeleton kind="card" count={6} />
    {:else if newReleasesError || newReleases.length === 0}
      <EmptyPlaceholder
        title={$t("study.music.youtube.shelf_new_releases_empty")}
        compact
      />
    {:else}
      <div class="h-scroll">
        {#each newReleases as album (album.browse_id)}
          <button type="button" class="card" onclick={() => openAlbum(album)}>
            <div class="cover">
              {#if album.thumbnail_url}
                <img src={album.thumbnail_url} alt="" loading="lazy" />
              {:else}
                <div class="cover-fallback" style:background={colorFromString(album.title)}></div>
              {/if}
            </div>
            <h3 class="card-title">{album.title}</h3>
            {#if album.artist}<p class="card-sub">{album.artist}</p>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="block">
    <NavigationTitle title={$t("study.music.youtube.shelf_trending")} />
    {#if trendingLoading}
      <YoutubeSkeleton kind="card" count={6} />
    {:else if trendingError || trendingItems.length === 0}
      <EmptyPlaceholder
        title={$t("study.music.youtube.shelf_trending_error")}
        compact
      />
    {:else}
      <div class="h-scroll">
        {#each trendingItems as item, idx (idx)}
          {#if item.kind === "video"}
            <button type="button" class="card" onclick={() => playVideoItem(item)}>
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
          {/if}
        {/each}
      </div>
    {/if}
  </section>

  <section class="block">
    <NavigationTitle title={$t("study.music.youtube.shelf_moods_and_genres")} />
    {#if moodsLoading}
      <div class="moods-skeleton">
        {#each Array(8) as _, i (i)}
          <div class="mood-card mood-skeleton"></div>
        {/each}
      </div>
    {:else if moodsError || moodSections.length === 0}
      <EmptyPlaceholder
        title={$t("study.music.youtube.moods_empty")}
        compact
      />
    {:else}
      <div class="moods-stack">
        {#each moodSections as section, i (section.title + i)}
          {#if section.title}
            <h3 class="moods-section-title">{section.title}</h3>
          {/if}
          <div class="moods-grid">
            {#each section.items as mood (mood.mood_id)}
              <button
                type="button"
                class="mood-card"
                style:background={moodSurface(mood.color)}
                onclick={() => openMood(mood)}
              >
                <span class="mood-title">{mood.title}</span>
              </button>
            {/each}
          </div>
        {/each}
      </div>
    {/if}
  </section>
</main>

<style>
  .explore {
    padding: 24px 24px 96px;
    max-width: 1200px;
    margin: 0 auto;
    display: grid;
    gap: 28px;
  }
  .page-head {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .back {
    align-self: start;
    background: transparent;
    border: 0;
    color: rgba(255, 255, 255, 0.7);
    cursor: pointer;
    padding: 6px 10px;
    border-radius: 8px;
    font: inherit;
    font-size: 13px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .back:hover {
    color: white;
    background: rgba(255, 255, 255, 0.06);
  }
  .head-text h1 {
    margin: 0;
    font-size: 32px;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: white;
  }
  .head-text p {
    margin: 4px 0 0;
    color: rgba(255, 255, 255, 0.55);
    font-size: 14px;
  }
  .block {
    display: grid;
    gap: 12px;
  }
  .h-scroll {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(160px, 180px);
    gap: 12px;
    overflow-x: auto;
    padding-bottom: 6px;
    scroll-snap-type: x mandatory;
    -ms-overflow-style: none;
    scrollbar-width: thin;
  }
  .h-scroll::-webkit-scrollbar {
    height: 6px;
  }
  .h-scroll::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.08);
    border-radius: 999px;
  }
  .card {
    scroll-snap-align: start;
    background: transparent;
    border: 0;
    padding: 0;
    cursor: pointer;
    text-align: left;
    color: inherit;
    display: grid;
    gap: 6px;
  }
  .cover {
    aspect-ratio: 1 / 1;
    border-radius: 12px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.04);
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
  .card-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.92);
    line-height: 1.3;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .card-sub {
    margin: 0;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .moods-stack {
    display: grid;
    gap: 18px;
  }
  .moods-section-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.55);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .moods-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  @media (min-width: 760px) {
    .moods-grid {
      grid-template-columns: repeat(4, 1fr);
    }
  }
  .mood-card {
    min-height: 84px;
    border-radius: 14px;
    border: 0;
    cursor: pointer;
    padding: 14px;
    display: flex;
    align-items: flex-end;
    text-align: left;
    font: inherit;
    color: white;
    overflow: hidden;
    position: relative;
    transition: transform 200ms ease, filter 200ms ease;
  }
  .mood-card:hover {
    transform: translateY(-2px);
    filter: brightness(1.1);
  }
  .mood-title {
    font-size: 14px;
    font-weight: 700;
    letter-spacing: -0.005em;
    line-height: 1.3;
  }
  .moods-skeleton {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  @media (min-width: 760px) {
    .moods-skeleton {
      grid-template-columns: repeat(4, 1fr);
    }
  }
  .mood-skeleton {
    background: linear-gradient(
      90deg,
      rgba(255, 255, 255, 0.04) 0%,
      rgba(255, 255, 255, 0.08) 50%,
      rgba(255, 255, 255, 0.04) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.6s linear infinite;
  }
  @keyframes shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .mood-skeleton {
      animation: none;
    }
    .mood-card {
      transition: none;
    }
  }
</style>
