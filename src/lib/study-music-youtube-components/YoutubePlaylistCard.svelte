<script lang="ts">
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import type { YoutubeSearchPlaylistItem, YoutubeSearchMixItem } from "$lib/study-bridge";

  type Props = {
    item: YoutubeSearchPlaylistItem | YoutubeSearchMixItem;
    onOpen?: (playlistId: string) => void;
  };
  let { item, onOpen }: Props = $props();

  function open() {
    if (onOpen) {
      onOpen(item.playlist_id);
      return;
    }
    const route = item.playlist_id.startsWith("OLAK") ? "album" : "playlist";
    void goto(`/study/music/youtube/${route}/${encodeURIComponent(item.playlist_id)}`);
  }

  let videoCount = $derived(item.kind === "playlist" ? item.video_count : null);
  let channelTitle = $derived(item.kind === "playlist" ? item.channel_title : null);
</script>

<button class="card" onclick={open} type="button">
  <div class="thumb-wrap">
    {#if item.thumbnail_url}
      <img src={item.thumbnail_url} alt="" loading="lazy" />
    {:else}
      <div class="thumb-placeholder"></div>
    {/if}
    <div class="stack-edge"></div>
    {#if videoCount != null}
      <span class="count tabular-nums">{$t("study.music.tracks_count_n", { count: videoCount })}</span>
    {/if}
  </div>
  <p class="title" title={item.title}>{item.title}</p>
  {#if channelTitle}
    <p class="meta">{channelTitle}</p>
  {/if}
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 0;
    background: transparent;
    border: 0;
    text-align: left;
    cursor: pointer;
    color: #f1f1f1;
  }
  .thumb-wrap {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: 12px;
    overflow: hidden;
    background: #272727;
  }
  .thumb-wrap img { width: 100%; height: 100%; object-fit: cover; display: block; transition: transform 220ms ease; }
  .card:hover .thumb-wrap img { transform: scale(1.045); }
  .stack-edge {
    position: absolute;
    right: -3px;
    top: 5px;
    bottom: 5px;
    width: 6px;
    border-radius: 0 8px 8px 0;
    background: rgba(255, 255, 255, 0.18);
  }
  .count {
    position: absolute;
    right: 8px;
    bottom: 8px;
    padding: 3px 5px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.8);
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    line-height: 1;
  }
  .title {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: #f1f1f1;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .card:hover .title { color: #fff; }
  .meta { margin: 0; font-size: 12.5px; color: #aaa; }
</style>
