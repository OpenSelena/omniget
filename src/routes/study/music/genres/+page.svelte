<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import { fmtDurationLong } from "$lib/study-music/format";

  type Genre = {
    name: string;
    track_count: number;
    album_count: number;
    artist_count: number;
    total_duration_ms: number;
    cover_path: string | null;
  };

  let genres = $state<Genre[]>([]);
  let loading = $state(true);

  async function load() {
    loading = true;
    try {
      const res = await pluginInvoke<{ genres: Genre[] }>(
        "study",
        "study:music:genres:list",
      );
      genres = res.genres ?? [];
    } finally {
      loading = false;
    }
  }

  function open(g: Genre) {
    goto(`/study/music/genres/${encodeURIComponent(g.name)}`);
  }

  function genreColor(name: string): string {
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = (hash * 31 + name.charCodeAt(i)) >>> 0;
    }
    const hue = hash % 360;
    return `oklch(0.45 0.18 ${hue})`;
  }

  onMount(() => {
    void load();
  });
</script>

<section class="page">
  <header class="head">
    <h1>{$t("study.music.genres_title")}</h1>
    <p class="sub">{$t("study.music.genres_subtitle")}</p>
  </header>

  {#if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if genres.length === 0}
    <div class="empty">
      <p>{$t("study.music.no_genres")}</p>
    </div>
  {:else}
    <ul class="grid">
      {#each genres as g (g.name)}
        <li>
          <button
            type="button"
            class="card"
            style="--seed: {genreColor(g.name)};"
            onclick={() => open(g)}
          >
            <span class="title">{g.name}</span>
            <span class="meta">
              {$t("study.music.tracks_count", { count: g.track_count })} ·
              {fmtDurationLong(g.total_duration_ms)}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 24px;
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
  .muted {
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
  }
  .empty {
    padding: 80px 0;
    text-align: center;
    color: rgba(255, 255, 255, 0.5);
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 16px;
  }
  .card {
    width: 100%;
    aspect-ratio: 16 / 11;
    padding: 18px 16px;
    border: 0;
    border-radius: 12px;
    background: linear-gradient(135deg, var(--seed), color-mix(in oklab, var(--seed) 60%, black));
    color: rgba(255, 255, 255, 0.95);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 8px;
    transition: transform 120ms ease, filter 120ms ease;
    overflow: hidden;
    position: relative;
  }
  .card:hover {
    transform: translateY(-2px);
    filter: brightness(1.1);
  }
  .title {
    font-size: 18px;
    font-weight: 700;
    text-transform: capitalize;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .meta {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.75);
  }
</style>
