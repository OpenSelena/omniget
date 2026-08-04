<script lang="ts">
  import { onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import { fmtDuration, fmtDurationLong } from "$lib/study-music/format";
  import {
    musicPlayer,
    type MusicTrack,
  } from "$lib/study-music/player-store.svelte";
  import CoverImage from "$lib/study-music-components/CoverImage.svelte";

  type Period = "7d" | "30d" | "90d" | "1y" | "all";

  type Summary = {
    period: string;
    total_plays: number;
    total_ms: number;
    unique_tracks: number;
  };

  type TopTrack = {
    id: number;
    title: string | null;
    artist: string | null;
    album: string | null;
    cover_path: string | null;
    duration_ms: number | null;
    plays: number;
    played_ms: number;
  };

  type TopArtist = {
    name: string;
    plays: number;
    played_ms: number;
    track_count: number;
  };

  type TopAlbum = {
    name: string;
    artist: string | null;
    plays: number;
    played_ms: number;
    cover_path: string | null;
  };

  type TopGenre = {
    name: string;
    plays: number;
    played_ms: number;
  };

  let period = $state<Period>("30d");
  let loading = $state(true);
  let summary = $state<Summary | null>(null);
  let tracks = $state<TopTrack[]>([]);
  let artists = $state<TopArtist[]>([]);
  let albums = $state<TopAlbum[]>([]);
  let genres = $state<TopGenre[]>([]);

  const PERIODS: { id: Period; labelKey: string }[] = [
    { id: "7d", labelKey: "study.music.period_7d" },
    { id: "30d", labelKey: "study.music.period_30d" },
    { id: "90d", labelKey: "study.music.period_90d" },
    { id: "1y", labelKey: "study.music.period_1y" },
    { id: "all", labelKey: "study.music.period_all" },
  ];

  async function load() {
    loading = true;
    try {
      const [s, tr, ar, al, ge] = await Promise.all([
        pluginInvoke<Summary>("study", "study:music:activity:summary", { period }),
        pluginInvoke<{ tracks: TopTrack[] }>("study", "study:music:activity:top_tracks", { period, limit: 10 }),
        pluginInvoke<{ artists: TopArtist[] }>("study", "study:music:activity:top_artists", { period, limit: 10 }),
        pluginInvoke<{ albums: TopAlbum[] }>("study", "study:music:activity:top_albums", { period, limit: 10 }),
        pluginInvoke<{ genres: TopGenre[] }>("study", "study:music:activity:top_genres", { period, limit: 10 }),
      ]);
      summary = s;
      tracks = tr.tracks ?? [];
      artists = ar.artists ?? [];
      albums = al.albums ?? [];
      genres = ge.genres ?? [];
    } finally {
      loading = false;
    }
  }

  async function playTrack(t: TopTrack) {
    try {
      const res = await pluginInvoke<{ track: MusicTrack }>(
        "study",
        "study:music:tracks:get",
        { id: t.id },
      );
      if (res.track) {
        await musicPlayer.play(res.track, [res.track]);
      }
    } catch {
      /* ignore */
    }
  }

  $effect(() => {
    void period;
    void load();
  });

  onMount(() => {
    void load();
  });
</script>

<section class="page">
  <header class="head">
    <h1>{$t("study.music.activity_title")}</h1>
    <p class="sub">{$t("study.music.activity_subtitle")}</p>
  </header>

  <div class="period-tabs" role="tablist">
    {#each PERIODS as p (p.id)}
      <button
        type="button"
        role="tab"
        class="period-tab"
        class:active={period === p.id}
        aria-selected={period === p.id}
        onclick={() => (period = p.id)}
      >{$t(p.labelKey)}</button>
    {/each}
  </div>

  {#if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if summary && summary.total_plays === 0}
    <div class="empty">
      <p>{$t("study.music.no_activity")}</p>
    </div>
  {:else if summary}
    <div class="stats">
      <div class="stat">
        <span class="stat-label">{$t("study.music.total_plays")}</span>
        <span class="stat-value">{summary.total_plays}</span>
      </div>
      <div class="stat">
        <span class="stat-label">{$t("study.music.total_listened")}</span>
        <span class="stat-value">{fmtDurationLong(summary.total_ms)}</span>
      </div>
      <div class="stat">
        <span class="stat-label">{$t("study.music.unique_tracks")}</span>
        <span class="stat-value">{summary.unique_tracks}</span>
      </div>
    </div>

    <section class="block">
      <h2>{$t("study.music.top_tracks")}</h2>
      {#if tracks.length === 0}
        <p class="muted">{$t("study.music.empty")}</p>
      {:else}
        <ol class="track-rank">
          {#each tracks as tr, i (tr.id)}
            <li>
              <button type="button" class="rank-row" onclick={() => playTrack(tr)}>
                <span class="rank">{i + 1}</span>
                <CoverImage src={tr.cover_path} alt={tr.title ?? ""} size={36} rounded="sm" trackId={tr.id} />
                <span class="info">
                  <span class="title">{tr.title ?? "—"}</span>
                  {#if tr.artist}<span class="artist">{tr.artist}</span>{/if}
                </span>
                <span class="metric">{tr.plays} · {fmtDuration(tr.played_ms)}</span>
              </button>
            </li>
          {/each}
        </ol>
      {/if}
    </section>

    <section class="block">
      <h2>{$t("study.music.top_artists")}</h2>
      {#if artists.length === 0}
        <p class="muted">{$t("study.music.empty")}</p>
      {:else}
        <ol class="rank-list">
          {#each artists as a, i (a.name + ":" + i)}
            <li class="rank-row plain">
              <span class="rank">{i + 1}</span>
              <span class="info">
                <span class="title">{a.name}</span>
                <span class="artist">{$t("study.music.tracks_count", { count: a.track_count })}</span>
              </span>
              <span class="metric">{a.plays} · {fmtDuration(a.played_ms)}</span>
            </li>
          {/each}
        </ol>
      {/if}
    </section>

    <section class="block">
      <h2>{$t("study.music.top_albums")}</h2>
      {#if albums.length === 0}
        <p class="muted">{$t("study.music.empty")}</p>
      {:else}
        <ol class="rank-list">
          {#each albums as a, i (a.name + ":" + i)}
            <li class="rank-row plain">
              <span class="rank">{i + 1}</span>
              <CoverImage src={a.cover_path} alt={a.name} size={36} rounded="sm" />
              <span class="info">
                <span class="title">{a.name}</span>
                {#if a.artist}<span class="artist">{a.artist}</span>{/if}
              </span>
              <span class="metric">{a.plays} · {fmtDuration(a.played_ms)}</span>
            </li>
          {/each}
        </ol>
      {/if}
    </section>

    <section class="block">
      <h2>{$t("study.music.top_genres")}</h2>
      {#if genres.length === 0}
        <p class="muted">{$t("study.music.empty")}</p>
      {:else}
        <ol class="rank-list">
          {#each genres as g, i (g.name + ":" + i)}
            <li class="rank-row plain">
              <span class="rank">{i + 1}</span>
              <span class="info">
                <span class="title">{g.name}</span>
              </span>
              <span class="metric">{g.plays} · {fmtDuration(g.played_ms)}</span>
            </li>
          {/each}
        </ol>
      {/if}
    </section>
  {/if}
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding-bottom: 40px;
  }
  .head h1 {
    margin: 0 0 4px;
    font-size: 28px;
    font-weight: 800;
    letter-spacing: -0.02em;
  }
  .sub {
    margin: 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
  }
  .period-tabs {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .period-tab {
    padding: 6px 14px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.65);
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }
  .period-tab:hover {
    color: rgba(255, 255, 255, 0.95);
    border-color: rgba(255, 255, 255, 0.25);
  }
  .period-tab.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent, white);
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 12px;
  }
  .stat {
    padding: 16px 18px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .stat-label {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .stat-value {
    font-size: 22px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.95);
  }
  .block h2 {
    margin: 0 0 12px;
    font-size: 16px;
    font-weight: 700;
  }
  .rank-list,
  .track-rank {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .rank-row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: rgba(255, 255, 255, 0.95);
    text-align: left;
    cursor: pointer;
    font-family: inherit;
  }
  .rank-row.plain { cursor: default; }
  .rank-row:hover:not(.plain) {
    background: rgba(255, 255, 255, 0.05);
  }
  .rank {
    flex-shrink: 0;
    width: 28px;
    text-align: center;
    color: rgba(255, 255, 255, 0.4);
    font-size: 14px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .artist {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .metric {
    flex-shrink: 0;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.5);
    font-variant-numeric: tabular-nums;
  }
  .muted { color: rgba(255, 255, 255, 0.5); font-size: 13px; }
  .empty { padding: 80px 0; text-align: center; color: rgba(255, 255, 255, 0.5); }
</style>
