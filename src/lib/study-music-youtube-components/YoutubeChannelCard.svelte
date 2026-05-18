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
    padding: 16px 8px;
    background: transparent;
    border: 0;
    border-radius: 12px;
    color: #f1f1f1;
    cursor: pointer;
    text-align: center;
    transition: background 140ms ease;
  }
  .card:hover { background: rgba(255, 255, 255, 0.06); }
  .avatar {
    width: 88px;
    height: 88px;
    border-radius: 50%;
    overflow: hidden;
    background: #272727;
    display: block;
  }
  .avatar img { width: 100%; height: 100%; object-fit: cover; }
  .title {
    margin: 8px 0 0 0;
    font-size: 14px;
    font-weight: 500;
    color: #f1f1f1;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .handle, .subs {
    margin: 0;
    font-size: 12.5px;
    color: #aaa;
  }
</style>
