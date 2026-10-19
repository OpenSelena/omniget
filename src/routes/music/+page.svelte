<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  interface Track {
    id: string;
    title: string;
    artist: string;
    album: string;
    duration: string;
    durationSecs: number;
    cover: string;
    url: string;
    isFavorite: boolean;
  }

  // Reactive Svelte 5 state runes
  let audioEl = $state<HTMLAudioElement | null>(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let volume = $state(0.85);
  let isMuted = $state(false);
  let isShuffle = $state(false);
  let repeatMode = $state<"off" | "all" | "one">("all");
  let searchQuery = $state("");
  let selectedTab = $state<"tracks" | "playlists" | "lyrics">("tracks");

  // Track catalog
  let tracks = $state<Track[]>([
    {
      id: "tr-1",
      title: "Night Drive in Tokyo",
      artist: "Synthwave Collective",
      album: "Neon Horizon",
      duration: "03:42",
      durationSecs: 222,
      cover: "https://images.unsplash.com/photo-1514525253161-7a46d19cd819?w=300&auto=format&fit=crop&q=80",
      url: "https://actions.google.com/sounds/v1/weather/rain_heavy.ogg",
      isFavorite: true
    },
    {
      id: "tr-2",
      title: "Chill Lofi Beats & Code",
      artist: "Aesthetic Soundscape",
      album: "Midnight Sessions",
      duration: "02:58",
      durationSecs: 178,
      cover: "https://images.unsplash.com/photo-1508700115892-45ecd05ae2ad?w=300&auto=format&fit=crop&q=80",
      url: "https://actions.google.com/sounds/v1/water/lake_waves_gentle.ogg",
      isFavorite: false
    },
    {
      id: "tr-3",
      title: "Cyberpunk Glitch Engine",
      artist: "Circuit Breakers",
      album: "Neural Network Vol. 1",
      duration: "04:15",
      durationSecs: 255,
      cover: "https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=300&auto=format&fit=crop&q=80",
      url: "https://actions.google.com/sounds/v1/ambiences/coffee_shop.ogg",
      isFavorite: true
    }
  ]);

  let currentTrackIndex = $state(0);
  let currentTrack = $derived(tracks[currentTrackIndex] || tracks[0]);

  let filteredTracks = $derived(
    searchQuery.trim() === ""
      ? tracks
      : tracks.filter(
          t =>
            t.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
            t.artist.toLowerCase().includes(searchQuery.toLowerCase())
        )
  );

  function formatTime(secs: number): string {
    if (isNaN(secs) || secs < 0) return "00:00";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }

  function playTrack(idx: number) {
    currentTrackIndex = idx;
    if (audioEl) {
      audioEl.src = tracks[idx].url;
      audioEl.play().then(() => {
        isPlaying = true;
      }).catch(() => {});
    }
  }

  function togglePlay() {
    if (!audioEl) return;
    if (audioEl.paused) {
      audioEl.play().then(() => {
        isPlaying = true;
      }).catch(() => {});
    } else {
      audioEl.pause();
      isPlaying = false;
    }
  }

  function nextTrack() {
    if (isShuffle) {
      const rand = Math.floor(Math.random() * tracks.length);
      playTrack(rand);
    } else {
      const nextIdx = (currentTrackIndex + 1) % tracks.length;
      playTrack(nextIdx);
    }
  }

  function prevTrack() {
    if (currentTime > 3) {
      if (audioEl) audioEl.currentTime = 0;
    } else {
      const prevIdx = (currentTrackIndex - 1 + tracks.length) % tracks.length;
      playTrack(prevIdx);
    }
  }

  function seek(secs: number) {
    if (!audioEl) return;
    audioEl.currentTime = Math.max(0, Math.min(duration, secs));
    currentTime = audioEl.currentTime;
  }

  function toggleMute() {
    if (!audioEl) return;
    isMuted = !isMuted;
    audioEl.muted = isMuted;
  }

  function setVolume(v: number) {
    if (!audioEl) return;
    volume = Math.max(0, Math.min(1, v));
    audioEl.volume = volume;
    if (volume > 0 && isMuted) {
      isMuted = false;
      audioEl.muted = false;
    }
  }

  function toggleFavorite(id: string) {
    const t = tracks.find(item => item.id === id);
    if (t) t.isFavorite = !t.isFavorite;
  }
</script>

<svelte:head>
  <title>OmniMusic - {currentTrack ? currentTrack.title : "Library"}</title>
</svelte:head>

<!-- Audio Element -->
<audio
  bind:this={audioEl}
  src={currentTrack?.url}
  ontimeupdate={() => {
    if (audioEl) currentTime = audioEl.currentTime;
  }}
  onloadedmetadata={() => {
    if (audioEl) duration = audioEl.duration;
  }}
  onended={() => {
    if (repeatMode === "one") {
      if (audioEl) {
        audioEl.currentTime = 0;
        audioEl.play();
      }
    } else {
      nextTrack();
    }
  }}
></audio>

<div class="music-container">
  <!-- Top Bar -->
  <header class="music-header">
    <div class="header-left">
      <a href="/" class="btn-back" title="Back to Dashboard">
        <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
          <path d="M19 12H5M12 19l-7-7 7-7" />
        </svg>
      </a>
      <div>
        <h1 class="header-title">OmniMusic</h1>
        <span class="header-sub">High Fidelity Local & Stream Audio</span>
      </div>
    </div>

    <div class="search-bar">
      <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2">
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <input
        type="text"
        placeholder="Search tracks, artists, albums..."
        bind:value={searchQuery}
      />
    </div>

    <!-- Navigation Tabs -->
    <nav class="tabs-nav">
      <button
        type="button"
        class="tab-btn"
        class:active={selectedTab === "tracks"}
        onclick={() => (selectedTab = "tracks")}
      >
        Tracks
      </button>
      <button
        type="button"
        class="tab-btn"
        class:active={selectedTab === "playlists"}
        onclick={() => (selectedTab = "playlists")}
      >
        Playlists
      </button>
      <button
        type="button"
        class="tab-btn"
        class:active={selectedTab === "lyrics"}
        onclick={() => (selectedTab = "lyrics")}
      >
        Lyrics
      </button>
    </nav>
  </header>

  <!-- Content Body -->
  <main class="music-content">
    {#if selectedTab === "tracks"}
      <div class="tracks-table-wrap">
        <table class="tracks-table">
          <thead>
            <tr>
              <th class="th-num">#</th>
              <th class="th-title">Title</th>
              <th class="th-artist">Artist</th>
              <th class="th-album">Album</th>
              <th class="th-time">Duration</th>
              <th class="th-fav"></th>
            </tr>
          </thead>
          <tbody>
            {#each filteredTracks as track, idx}
              <tr
                class="track-row"
                class:active={currentTrack?.id === track.id}
                onclick={() => playTrack(idx)}
              >
                <td class="td-num">
                  {#if currentTrack?.id === track.id && isPlaying}
                    <span class="playing-bars">
                      <span class="bar"></span>
                      <span class="bar"></span>
                      <span class="bar"></span>
                    </span>
                  {:else}
                    {idx + 1}
                  {/if}
                </td>
                <td class="td-title">
                  <div class="title-cell">
                    <img src={track.cover} alt={track.title} class="track-thumb" />
                    <span class="track-name">{track.title}</span>
                  </div>
                </td>
                <td class="td-artist">{track.artist}</td>
                <td class="td-album">{track.album}</td>
                <td class="td-time">{track.duration}</td>
                <td class="td-fav">
                  <button
                    type="button"
                    class="btn-fav"
                    class:starred={track.isFavorite}
                    onclick={(e) => {
                      e.stopPropagation();
                      toggleFavorite(track.id);
                    }}
                    title="Favorite"
                  >
                    ★
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

    {:else if selectedTab === "playlists"}
      <div class="playlists-grid">
        <div class="playlist-card">
          <div class="pl-cover favorites-gradient">
            <span>★</span>
          </div>
          <h3 class="pl-title">Starred Favorites</h3>
          <span class="pl-meta">{tracks.filter(t => t.isFavorite).length} tracks</span>
        </div>

        <div class="playlist-card">
          <div class="pl-cover lofi-gradient">
            <span>♫</span>
          </div>
          <h3 class="pl-title">Deep Focus & Code</h3>
          <span class="pl-meta">12 tracks</span>
        </div>

        <div class="playlist-card">
          <div class="pl-cover synth-gradient">
            <span>⚡</span>
          </div>
          <h3 class="pl-title">Cyberpunk Synthwave</h3>
          <span class="pl-meta">8 tracks</span>
        </div>
      </div>

    {:else if selectedTab === "lyrics"}
      <div class="lyrics-view">
        <div class="lyrics-header">
          <img src={currentTrack.cover} alt={currentTrack.title} class="lyrics-art" />
          <div class="lyrics-info">
            <h2>{currentTrack.title}</h2>
            <p>{currentTrack.artist} — {currentTrack.album}</p>
          </div>
        </div>

        <div class="lyrics-scroll">
          <p class="lyric-line passed">Lights reflect across the midnight boulevard</p>
          <p class="lyric-line current">Zero-latency sounds echoing through the wires</p>
          <p class="lyric-line">Memory-safe code humming in the background</p>
          <p class="lyric-line">Autonomous agents orchestrating the rhythm</p>
          <p class="lyric-line">Building the future line by line</p>
        </div>
      </div>
    {/if}
  </main>

  <!-- Bottom Player Bar -->
  <footer class="player-bar">
    <!-- Track Info Column -->
    <div class="player-left">
      <img src={currentTrack.cover} alt={currentTrack.title} class="player-cover" />
      <div class="player-titles">
        <span class="player-track-name">{currentTrack.title}</span>
        <span class="player-artist-name">{currentTrack.artist}</span>
      </div>
      <button
        type="button"
        class="btn-fav small"
        class:starred={currentTrack.isFavorite}
        onclick={() => toggleFavorite(currentTrack.id)}
      >
        ★
      </button>
    </div>

    <!-- Center Playback Controls & Scrubber -->
    <div class="player-center">
      <div class="playback-buttons">
        <button
          type="button"
          class="ctrl-btn"
          class:active={isShuffle}
          onclick={() => (isShuffle = !isShuffle)}
          title="Shuffle"
        >
          ⇄
        </button>

        <button type="button" class="ctrl-btn" onclick={prevTrack} title="Previous">
          ⏮
        </button>

        <button type="button" class="ctrl-btn play-main" onclick={togglePlay} title={isPlaying ? "Pause" : "Play"}>
          {#if isPlaying}
            ⏸
          {:else}
            ▶
          {/if}
        </button>

        <button type="button" class="ctrl-btn" onclick={nextTrack} title="Next">
          ⏭
        </button>

        <button
          type="button"
          class="ctrl-btn"
          class:active={repeatMode !== "off"}
          onclick={() => {
            if (repeatMode === "off") repeatMode = "all";
            else if (repeatMode === "all") repeatMode = "one";
            else repeatMode = "off";
          }}
          title={`Repeat: ${repeatMode}`}
        >
          {repeatMode === "one" ? "🔂" : "🔁"}
        </button>
      </div>

      <!-- Scrubber Timeline -->
      <div class="timeline-row">
        <span class="time-label">{formatTime(currentTime)}</span>
        <div
          class="timeline-bar"
          role="slider"
          tabindex="0"
          aria-label="Seek Track"
          aria-valuenow={currentTime}
          aria-valuemin="0"
          aria-valuemax={duration || 100}
          onclick={(e) => {
            const rect = e.currentTarget.getBoundingClientRect();
            const pct = (e.clientX - rect.left) / rect.width;
            seek(pct * duration);
          }}
          onkeydown={(e) => {
            if (e.key === "ArrowLeft") seek(currentTime - 5);
            if (e.key === "ArrowRight") seek(currentTime + 5);
          }}
        >
          <div
            class="timeline-fill"
            style:width="{duration > 0 ? (currentTime / duration) * 100 : 0}%"
          ></div>
        </div>
        <span class="time-label">{formatTime(duration)}</span>
      </div>
    </div>

    <!-- Right Volume Controls -->
    <div class="player-right">
      <button type="button" class="btn-mute" onclick={toggleMute} title="Mute">
        {isMuted || volume === 0 ? "🔇" : volume < 0.5 ? "🔉" : "🔊"}
      </button>
      <input
        type="range"
        min="0"
        max="1"
        step="0.05"
        value={isMuted ? 0 : volume}
        oninput={(e) => setVolume(parseFloat(e.currentTarget.value))}
        class="vol-slider"
      />
    </div>
  </footer>
</div>

<style>
  .music-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #090d16;
    color: #f3f4f6;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  }

  .music-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 28px;
    background: #0f172a;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    gap: 20px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .btn-back {
    all: unset;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
    cursor: pointer;
  }

  .btn-back:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .header-title {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
  }

  .header-sub {
    font-size: 12px;
    color: #94a3b8;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 6px 12px;
    width: 280px;
  }

  .search-bar input {
    all: unset;
    flex: 1;
    font-size: 13px;
    color: #fff;
  }

  .tabs-nav {
    display: flex;
    gap: 4px;
    background: rgba(255, 255, 255, 0.05);
    padding: 3px;
    border-radius: 8px;
  }

  .tab-btn {
    all: unset;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    color: #94a3b8;
    cursor: pointer;
  }

  .tab-btn.active {
    background: #0284c7;
    color: #fff;
  }

  .music-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px 28px;
  }

  /* Tracks Table */
  .tracks-table-wrap {
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    overflow: hidden;
  }

  .tracks-table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    font-size: 13px;
  }

  th {
    padding: 12px 16px;
    font-weight: 600;
    color: #64748b;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    text-transform: uppercase;
    font-size: 11px;
    letter-spacing: 0.05em;
  }

  td {
    padding: 10px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }

  .track-row {
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .track-row:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .track-row.active {
    background: rgba(2, 132, 199, 0.15);
  }

  .title-cell {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .track-thumb {
    width: 36px;
    height: 36px;
    border-radius: 6px;
    object-fit: cover;
  }

  .track-name {
    font-weight: 600;
    color: #fff;
  }

  .track-row.active .track-name {
    color: #38bdf8;
  }

  .btn-fav {
    all: unset;
    font-size: 16px;
    color: #475569;
    cursor: pointer;
    transition: color 0.15s ease;
  }

  .btn-fav.starred {
    color: #eab308;
  }

  .playing-bars {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
    height: 14px;
  }

  .bar {
    width: 3px;
    background: #38bdf8;
    animation: bounce 0.8s ease-in-out infinite alternate;
  }

  .bar:nth-child(2) { animation-delay: 0.2s; height: 12px; }
  .bar:nth-child(3) { animation-delay: 0.4s; height: 8px; }

  @keyframes bounce {
    from { height: 4px; }
    to { height: 14px; }
  }

  /* Playlists */
  .playlists-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 20px;
  }

  .playlist-card {
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 16px;
    cursor: pointer;
    transition: transform 0.15s ease;
  }

  .playlist-card:hover {
    transform: translateY(-3px);
  }

  .pl-cover {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 8px;
    display: grid;
    place-items: center;
    font-size: 32px;
    margin-bottom: 12px;
  }

  .favorites-gradient { background: linear-gradient(135deg, #f59e0b, #ef4444); color: #fff; }
  .lofi-gradient { background: linear-gradient(135deg, #0284c7, #8b5cf6); color: #fff; }
  .synth-gradient { background: linear-gradient(135deg, #ec4899, #8b5cf6); color: #fff; }

  .pl-title {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 600;
  }

  .pl-meta {
    font-size: 12px;
    color: #94a3b8;
  }

  /* Lyrics */
  .lyrics-view {
    max-width: 680px;
    margin: 0 auto;
    text-align: center;
  }

  .lyrics-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    margin-bottom: 32px;
  }

  .lyrics-art {
    width: 120px;
    height: 120px;
    border-radius: 12px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
  }

  .lyrics-scroll {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .lyric-line {
    font-size: 18px;
    color: #64748b;
    margin: 0;
    transition: all 0.3s ease;
  }

  .lyric-line.current {
    font-size: 24px;
    font-weight: 700;
    color: #38bdf8;
    transform: scale(1.05);
  }

  .lyric-line.passed {
    color: #94a3b8;
  }

  /* Bottom Player Bar */
  .player-bar {
    height: 84px;
    background: #0f172a;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    display: grid;
    grid-template-columns: 280px 1fr 240px;
    align-items: center;
    padding: 0 24px;
    gap: 20px;
  }

  .player-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .player-cover {
    width: 48px;
    height: 48px;
    border-radius: 6px;
    object-fit: cover;
  }

  .player-titles {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .player-track-name {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .player-artist-name {
    font-size: 11px;
    color: #94a3b8;
  }

  .player-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .playback-buttons {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .ctrl-btn {
    all: unset;
    cursor: pointer;
    font-size: 16px;
    color: #94a3b8;
    transition: color 0.15s ease;
  }

  .ctrl-btn:hover {
    color: #fff;
  }

  .ctrl-btn.active {
    color: #38bdf8;
  }

  .ctrl-btn.play-main {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: #0284c7;
    color: #fff;
    font-size: 14px;
  }

  .ctrl-btn.play-main:hover {
    background: #0369a1;
  }

  .timeline-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    max-width: 520px;
  }

  .time-label {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: #64748b;
  }

  .timeline-bar {
    flex: 1;
    height: 4px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 9999px;
    cursor: pointer;
    position: relative;
  }

  .timeline-fill {
    height: 100%;
    background: #0284c7;
    border-radius: 9999px;
  }

  .player-right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
  }

  .btn-mute {
    all: unset;
    cursor: pointer;
    font-size: 16px;
  }

  .vol-slider {
    width: 80px;
    accent-color: #0284c7;
    cursor: pointer;
  }
</style>
