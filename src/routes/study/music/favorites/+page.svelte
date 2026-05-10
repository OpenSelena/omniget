<script lang="ts">
  import { onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import TrackRow from "$lib/study-music-components/TrackRow.svelte";
  import type { MusicTrack } from "$lib/study-music/player-store.svelte";
  import {
    spotifyStore,
    spotifyTrackToMusicTrack,
  } from "$lib/study-music/spotify-store.svelte";
  import {
    soundcloudStore,
    scTrackToMusicTrack,
  } from "$lib/study-music/soundcloud-store.svelte";

  let tracks = $state<MusicTrack[]>([]);
  let loading = $state(true);

  async function load() {
    loading = true;
    try {
      const [localRes] = await Promise.allSettled([
        pluginInvoke<{ tracks: MusicTrack[] }>(
          "study",
          "study:music:favorites:list",
          { limit: 500 },
        ),
        spotifyStore.status.logged_in && spotifyStore.savedTracks.length === 0
          ? spotifyStore.loadAll()
          : Promise.resolve(),
        soundcloudStore.isLoggedIn && soundcloudStore.likedTracks.length === 0
          ? soundcloudStore.loadAll()
          : Promise.resolve(),
      ]);

      const local: MusicTrack[] =
        localRes.status === "fulfilled"
          ? (localRes.value.tracks ?? []).map((t) => ({ ...t, favorite: true }))
          : [];

      const spotifyTracks: MusicTrack[] = spotifyStore.savedTracks
        .filter((sp) => !spotifyStore.localMatches.has(sp.id))
        .map((t) => ({
          ...spotifyTrackToMusicTrack(t),
          favorite: true,
        }));

      const scTracks: MusicTrack[] = soundcloudStore.likedTracks.map((t) => ({
        ...scTrackToMusicTrack(t),
        favorite: true,
      }));

      tracks = [...local, ...spotifyTracks, ...scTracks];
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<section class="favorites-page">
  <header class="head">
    <h1>{$t("study.music.favorites_title")}</h1>
    <p class="sub">{$t("study.music.favorites_subtitle")}</p>
  </header>
  {#if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if tracks.length === 0}
    <div class="empty">
      <p>{$t("study.music.favorites_empty")}</p>
    </div>
  {:else}
    <ul class="track-list">
      {#each tracks as track (track.id)}
        <TrackRow {track} queue={tracks} showCover showAlbum />
      {/each}
    </ul>
  {/if}
</section>

<style>
  .favorites-page { display: flex; flex-direction: column; gap: 14px; }
  .head h1 { margin: 0; font-size: 28px; font-weight: 800; color: var(--secondary); letter-spacing: -0.01em; }
  .head .sub { margin: 4px 0 0; color: var(--tertiary); font-size: 13px; }
  .track-list { list-style: none; margin: 8px 0 0; padding: 0; display: flex; flex-direction: column; gap: 1px; }
  .empty { padding: 48px 24px; text-align: center; color: var(--tertiary); }
  .muted { color: var(--tertiary); font-size: 13px; }
</style>
