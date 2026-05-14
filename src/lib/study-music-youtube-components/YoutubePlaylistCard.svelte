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
    gap: 8px;
    padding: 0;
    background: transparent;
    border: 0;
    text-align: left;
    cursor: pointer;
    color: var(--secondary);
  }
  .thumb-wrap {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: 8px;
    overflow: hidden;
    background: color-mix(in oklab, var(--button) 60%, transparent);
  }
  .thumb-wrap img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .stack-edge {
    position: absolute;
    right: -2px;
    top: 4px;
    bottom: 4px;
    width: 4px;
    border-radius: 0 8px 8px 0;
    background: color-mix(in oklab, var(--button) 80%, transparent);
  }
  .count {
    position: absolute;
    right: 6px;
    bottom: 6px;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.75);
    color: #fff;
    font-size: 11px;
  }
  .title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--secondary);
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .meta { margin: 0; font-size: 11px; color: var(--tertiary); }
</style>
