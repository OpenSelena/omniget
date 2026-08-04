<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { colorFromString } from "$lib/study-music/format";
  import {
    studyYoutubeMoodDetail,
    type YoutubeMoodPlaylistItem,
    type YoutubeMoodShelf,
  } from "$lib/study-bridge";
  import NavigationTitle from "$lib/study-music-components/NavigationTitle.svelte";
  import EmptyPlaceholder from "$lib/study-music-components/EmptyPlaceholder.svelte";
  import YoutubeSkeleton from "$lib/study-music-youtube-components/YoutubeSkeleton.svelte";

  let moodId = $derived($page.params.id ?? "");
  let params = $derived($page.url.searchParams.get("params"));

  let title = $state<string | null>(null);
  let shelves = $state<YoutubeMoodShelf[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    if (!moodId) return;
    loading = true;
    error = null;
    try {
      const res = await studyYoutubeMoodDetail({ moodId, params });
      title = res.title ?? null;
      shelves = res.shelves ?? [];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function openPlaylist(item: YoutubeMoodPlaylistItem) {
    const id = item.playlist_id;
    if (!id) return;
    const route = id.startsWith("OLAK") ? "album" : "playlist";
    void goto(`/study/music/youtube/${route}/${encodeURIComponent(id)}`);
  }

  $effect(() => {
    if (moodId) void load();
  });
</script>

<svelte:head>
  <title>{title ?? $t("study.music.youtube.explore_title")}</title>
</svelte:head>

<main class="mood-page">
  <header class="page-head">
    <button class="back" type="button" onclick={() => goto("/study/music/youtube/explore")}>
      <span aria-hidden="true">←</span>
      <span>{$t("study.music.youtube.back_to_explore")}</span>
    </button>
    {#if title}
      <h1>{title}</h1>
    {:else if loading}
      <div class="title-skeleton"></div>
    {/if}
  </header>

  {#if loading}
    <section class="block">
      <YoutubeSkeleton kind="card" count={6} />
    </section>
  {:else if error}
    <EmptyPlaceholder
      title={$t("study.music.youtube.mood_loading_error")}
      ctaLabel={$t("study.music.retry")}
      onCta={() => load()}
    />
  {:else if shelves.length === 0}
    <EmptyPlaceholder
      title={$t("study.music.youtube.mood_empty")}
      ctaLabel={$t("study.music.youtube.back_to_explore")}
      ctaHref="/study/music/youtube/explore"
    />
  {:else}
    {#each shelves as shelf, idx (shelf.title + idx)}
      <section class="block">
        {#if shelf.title}<NavigationTitle title={shelf.title} />{/if}
        <div class="h-scroll">
          {#each shelf.items as item (item.playlist_id + idx)}
            <button type="button" class="card" onclick={() => openPlaylist(item)}>
              <div class="cover">
                {#if item.thumbnail_url}
                  <img src={item.thumbnail_url} alt="" loading="lazy" />
                {:else}
                  <div class="cover-fallback" style:background={colorFromString(item.title)}></div>
                {/if}
              </div>
              <h3 class="card-title">{item.title}</h3>
              {#if item.subtitle}<p class="card-sub">{item.subtitle}</p>{/if}
            </button>
          {/each}
        </div>
      </section>
    {/each}
  {/if}
</main>

<style>
  .mood-page {
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
  h1 {
    margin: 0;
    font-size: 32px;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: white;
  }
  .title-skeleton {
    height: 36px;
    width: 280px;
    border-radius: 8px;
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
    .title-skeleton {
      animation: none;
    }
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
</style>
