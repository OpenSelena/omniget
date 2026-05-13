<script lang="ts">
  import type { YoutubeSearchItem } from "$lib/study-bridge";
  import YoutubeItem from "./YoutubeItem.svelte";

  type Props = {
    title?: string | null;
    items: YoutubeSearchItem[];
    onPlay?: (videoId: string) => void;
    onPlaylistOpen?: (playlistId: string) => void;
    minCard?: number;
  };
  let { title, items, onPlay, onPlaylistOpen, minCard = 220 }: Props = $props();
</script>

<section class="shelf">
  {#if title}
    <h3 class="title">{title}</h3>
  {/if}
  <div class="grid" style="--min-card: {minCard}px">
    {#each items as item, idx (idx)}
      <YoutubeItem item={item} {onPlay} {onPlaylistOpen} />
    {/each}
  </div>
</section>

<style>
  .shelf { display: flex; flex-direction: column; gap: 12px; }
  .title {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
    color: var(--secondary);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--min-card, 220px), 1fr));
    gap: 18px 14px;
  }
</style>
