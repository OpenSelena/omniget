<script lang="ts">
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { colorFromString } from "$lib/study-music/format";
  import type { YoutubeSearchVideoItem } from "$lib/study-bridge";

  type Props = {
    item: YoutubeSearchVideoItem;
    onPlay?: (videoId: string) => void;
  };
  let { item, onPlay }: Props = $props();

  const channelInitial = $derived(
    (item.channel_title ?? "?").trim().charAt(0).toUpperCase() || "?",
  );
  const channelColor = $derived(colorFromString(item.channel_title ?? item.video_id));

  function play() {
    if (onPlay) onPlay(item.video_id);
  }

  function openChannel(e: MouseEvent) {
    e.stopPropagation();
    if (item.channel_id) goto(`/study/music/youtube/channel/${item.channel_id}`);
  }
</script>

<div class="card">
  <button type="button" class="thumb-wrap" onclick={play} aria-label={item.title}>
    {#if item.thumbnail_url}
      <img src={item.thumbnail_url} alt="" loading="lazy" />
    {:else}
      <div class="thumb-placeholder"></div>
    {/if}
    <span class="thumb-hover" aria-hidden="true"></span>
    {#if item.is_live}
      <span class="live">{$t("study.music.youtube_live")}</span>
    {:else if item.duration_text}
      <span class="duration tabular-nums">{item.duration_text}</span>
    {/if}
  </button>

  <div class="row">
    <button
      type="button"
      class="avatar"
      onclick={openChannel}
      style:background={channelColor}
      title={item.channel_title ?? ""}
      aria-label={item.channel_title ?? ""}
    >{channelInitial}</button>
    <div class="meta">
      <button type="button" class="title-btn" onclick={play} title={item.title}>
        <span class="title">{item.title}</span>
      </button>
      {#if item.channel_title}
        <button class="channel" onclick={openChannel} type="button" title={item.channel_title}>
          {item.channel_title}
        </button>
      {/if}
      <p class="stats">
        {#if item.view_count_text}<span>{item.view_count_text}</span>{/if}
        {#if item.view_count_text && item.published_time_text}<span aria-hidden="true"> • </span>{/if}
        {#if item.published_time_text}<span>{item.published_time_text}</span>{/if}
      </p>
    </div>
  </div>
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 0;
    color: var(--secondary);
  }
  .thumb-wrap {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: 12px;
    overflow: hidden;
    background: color-mix(in oklab, var(--button) 60%, transparent);
    padding: 0;
    border: 0;
    cursor: pointer;
    display: block;
    width: 100%;
  }
  .thumb-wrap img,
  .thumb-placeholder {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: transform 220ms ease;
  }
  .thumb-placeholder {
    background: color-mix(in oklab, var(--button) 50%, transparent);
  }
  .thumb-wrap:hover img {
    transform: scale(1.045);
  }
  .thumb-hover {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0);
    transition: background 160ms ease;
    pointer-events: none;
  }
  .thumb-wrap:hover .thumb-hover {
    background: rgba(0, 0, 0, 0.12);
  }
  .duration,
  .live {
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
    letter-spacing: 0.01em;
  }
  .live {
    left: 8px;
    right: auto;
    background: #cc0000;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .row {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    min-width: 0;
  }
  .avatar {
    flex: 0 0 auto;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: 0;
    cursor: pointer;
    color: #fff;
    font-size: 15px;
    font-weight: 700;
    display: grid;
    place-items: center;
    line-height: 1;
    user-select: none;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    flex: 1 1 auto;
  }
  .title-btn {
    padding: 0;
    background: transparent;
    border: 0;
    text-align: left;
    cursor: pointer;
    color: inherit;
    width: 100%;
  }
  .title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--secondary);
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .title-btn:hover .title {
    color: var(--accent);
  }
  .channel {
    margin: 0;
    padding: 0;
    background: transparent;
    border: 0;
    color: var(--tertiary);
    font-size: 12.5px;
    cursor: pointer;
    text-align: left;
    align-self: flex-start;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .channel:hover {
    color: var(--secondary);
  }
  .stats {
    margin: 0;
    font-size: 12.5px;
    color: var(--tertiary);
  }
</style>
