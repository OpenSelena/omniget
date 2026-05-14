<script lang="ts">
  import { goto } from "$app/navigation";
  import type { YoutubeSearchChannelItem } from "$lib/study-bridge";

  type Props = { item: YoutubeSearchChannelItem };
  let { item }: Props = $props();

  function open() {
    goto(`/study/music/youtube/channel/${item.channel_id}`);
  }
</script>

<button class="card" onclick={open} type="button">
  <div class="avatar">
    {#if item.avatar_url}
      <img src={item.avatar_url} alt="" loading="lazy" />
    {/if}
  </div>
  <p class="title" title={item.title}>{item.title}</p>
  {#if item.handle}
    <p class="handle">{item.handle}</p>
  {/if}
  {#if item.subscribers_text}
    <p class="subs">{item.subscribers_text}</p>
  {/if}
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 12px 8px;
    background: transparent;
    border: 0;
    border-radius: 12px;
    color: var(--secondary);
    cursor: pointer;
    text-align: center;
  }
  .card:hover { background: color-mix(in oklab, var(--button) 30%, transparent); }
  .avatar {
    width: 96px;
    height: 96px;
    border-radius: 50%;
    overflow: hidden;
    background: color-mix(in oklab, var(--button) 60%, transparent);
    display: block;
  }
  .avatar img { width: 100%; height: 100%; object-fit: cover; }
  .title {
    margin: 6px 0 0 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--secondary);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .handle, .subs {
    margin: 0;
    font-size: 11px;
    color: var(--tertiary);
  }
</style>
