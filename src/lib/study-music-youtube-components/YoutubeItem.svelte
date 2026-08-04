<script lang="ts">
  import type { YoutubeSearchItem } from "$lib/study-bridge";
  import YoutubeVideoCard from "./YoutubeVideoCard.svelte";
  import YoutubeChannelCard from "./YoutubeChannelCard.svelte";
  import YoutubePlaylistCard from "./YoutubePlaylistCard.svelte";

  type Props = {
    item: YoutubeSearchItem;
    onPlay?: (videoId: string) => void;
    onPlaylistOpen?: (playlistId: string) => void;
  };
  let { item, onPlay, onPlaylistOpen }: Props = $props();
</script>

{#if item.kind === "video"}
  <YoutubeVideoCard item={item} {onPlay} />
{:else if item.kind === "channel"}
  <YoutubeChannelCard item={item} />
{:else if item.kind === "playlist" || item.kind === "mix"}
  <YoutubePlaylistCard item={item} onOpen={onPlaylistOpen} />
{/if}
