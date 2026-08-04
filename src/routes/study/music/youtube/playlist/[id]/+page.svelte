<script lang="ts">
  import { page } from "$app/stores";
  import { t } from "$lib/i18n";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import {
    studyYoutubePlaylist,
    type YoutubePlaylist,
    type YoutubePlaylistVideo,
    type YoutubeSearchVideoItem,
  } from "$lib/study-bridge";
  import MediaHero from "$lib/study-music-components/MediaHero.svelte";
  import TrackListRow from "$lib/study-music-components/TrackListRow.svelte";
  import YoutubeSkeleton from "$lib/study-music-youtube-components/YoutubeSkeleton.svelte";
  import YoutubeError from "$lib/study-music-youtube-components/YoutubeError.svelte";
  import EmptyPlaceholder from "$lib/study-music-components/EmptyPlaceholder.svelte";
  import ExpandableText from "$lib/study-music-components/ExpandableText.svelte";
  import StickyHeader from "$lib/study-music-components/StickyHeader.svelte";
  import { playYoutubeVideoItem } from "$lib/study-music/youtube-play-helper";
  import { musicPlayer } from "$lib/study-music/player-store.svelte";

  let playlistId = $derived($page.params.id ?? "");
  let playlist = $state<YoutubePlaylist | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let pendingShuffle = $state(false);

  async function load() {
    loading = true;
    error = null;
    try {
      playlist = await studyYoutubePlaylist({ playlistId });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function totalDurationText(videos: YoutubePlaylistVideo[]): string | null {
    const totalSeconds = videos.reduce((acc, v) => acc + (v.duration_seconds ?? 0), 0);
    if (totalSeconds <= 0) return null;
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    if (hours > 0) return `${hours}h ${minutes}min`;
    return `${minutes}min`;
  }

  function asSearchVideo(v: YoutubePlaylistVideo, fallbackThumb: string | null): YoutubeSearchVideoItem {
    return {
      kind: "video",
      video_id: v.video_id,
      title: v.title,
      channel_id: v.channel_id,
      channel_title: v.channel_title,
      thumbnail_url: v.thumbnail_url ?? fallbackThumb,
      duration_text: v.duration_text,
      published_time_text: null,
      view_count_text: null,
      short_description: null,
      is_live: false,
    };
  }

  async function playFromIndex(idx: number, shuffle = false) {
    if (!playlist || playlist.videos.length === 0) return;
    const fallbackThumb = playlist.info.thumbnail_url;
    let queue = playlist.videos.map((v) => asSearchVideo(v, fallbackThumb));
    let target = queue[idx];
    if (shuffle) {
      const rest = queue.slice();
      for (let i = rest.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [rest[i], rest[j]] = [rest[j], rest[i]];
      }
      queue = rest;
      target = queue[0];
    }
    try {
      await playYoutubeVideoItem(target, queue);
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function shareLink() {
    if (!playlist) return;
    const url = `https://www.youtube.com/playlist?list=${encodeURIComponent(playlist.info.playlist_id)}`;
    try {
      await navigator.clipboard.writeText(url);
      showToast("success", $t("study.music.share_copied"));
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function shuffleAll() {
    if (pendingShuffle) return;
    pendingShuffle = true;
    try {
      await playFromIndex(0, true);
    } finally {
      pendingShuffle = false;
    }
  }

  $effect(() => {
    if (playlistId) void load();
  });
</script>

<section class="page">
  <StickyHeader>
    <div class="sticky-row">
      <button class="back" type="button" onclick={() => history.back()}>← {$t("study.music.back")}</button>
      {#if playlist}
        <span class="sticky-title">{playlist.info.title || playlistId}</span>
      {/if}
    </div>
  </StickyHeader>

  {#if loading && !playlist}
    <div class="hero-skel"><YoutubeSkeleton kind="card" count={1} /></div>
    <div class="list-skel"><YoutubeSkeleton kind="row" count={6} /></div>
  {:else if error && !playlist}
    <YoutubeError error={error} onRetry={() => { error = null; void load(); }} />
  {:else if playlist}
    {@const total = totalDurationText(playlist.videos)}
    {@const trackCount = playlist.info.video_count ?? playlist.videos.length}
    {@const ownerLine = playlist.info.owner_title
      ? $t("study.music.youtube.creator_by", { name: playlist.info.owner_title })
      : null}
    <MediaHero
      coverUrl={playlist.info.thumbnail_url}
      eyebrow={$t("study.music.eyebrow_playlist")}
      title={playlist.info.title || playlistId}
      subtitle={ownerLine}
      stats={[
        { label: "", value: trackCount === 1 ? $t("study.music.track_count_one") : $t("study.music.tracks_count_n", { count: trackCount }) },
        { label: "", value: total },
        { label: "", value: playlist.info.view_count_text },
      ]}
      actions={[
        {
          id: "play",
          label: $t("study.music.play_all"),
          primary: true,
          onClick: () => void playFromIndex(0, false),
        },
        {
          id: "shuffle",
          label: $t("study.music.shuffle_play"),
          busy: pendingShuffle,
          onClick: () => void shuffleAll(),
        },
        {
          id: "share",
          label: $t("study.music.share"),
          onClick: () => void shareLink(),
        },
      ]}
    >
      {#snippet description()}
        <ExpandableText text={playlist?.info.description ?? null} maxLines={3} />
      {/snippet}
    </MediaHero>

    {#if playlist.videos.length === 0}
      <EmptyPlaceholder title={$t("study.music.playlist_empty")} />
    {:else}
      <div class="tracklist">
        {#each playlist.videos as v, idx (v.video_id)}
          <TrackListRow
            index={idx}
            coverUrl={v.thumbnail_url}
            title={v.title}
            subtitle={v.channel_title}
            durationText={v.duration_text}
            isPlaying={musicPlayer.currentTrack?.youtube_video_id === v.video_id}
            onPlay={() => void playFromIndex(idx, false)}
          />
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 0 20px 32px;
  }
  .sticky-row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 32px;
  }
  .sticky-title {
    font-size: 14px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.9);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0;
    transition: opacity 200ms ease;
  }
  :global(.sticky-header.opaque) .sticky-title {
    opacity: 1;
  }
  @media (prefers-reduced-motion: reduce) {
    .sticky-title {
      transition: none;
    }
  }
  .back {
    align-self: flex-start;
    padding: 6px 10px;
    background: transparent;
    border: 0;
    color: var(--tertiary);
    cursor: pointer;
    font-size: 13px;
  }
  .back:hover { color: var(--secondary); }
  .tracklist {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
  }
  .hero-skel,
  .list-skel { display: flex; flex-direction: column; gap: 8px; }
</style>
