<script lang="ts">
  import { onMount } from "svelte";

  interface StreamFormat {
    id: string;
    label: string;
    ext: string;
    quality: string;
    size: string;
    type: "video" | "audio";
  }

  interface VideoResult {
    id: string;
    url: string;
    title: string;
    author: string;
    duration: string;
    thumbnail: string;
    views: string;
    formats: StreamFormat[];
  }

  interface DownloadTask {
    id: string;
    title: string;
    quality: string;
    progress: number;
    speed: string;
    status: "downloading" | "completed" | "paused" | "error";
  }

  // Reactive state
  let urlInput = $state("");
  let isAnalyzing = $state(false);
  let selectedTab = $state<"extractor" | "playlist" | "history">("extractor");
  let activeFormat = $state<string>("1080p");
  let currentVideo = $state<VideoResult | null>(null);

  // Queue of downloads
  let downloadQueue = $state<DownloadTask[]>([
    {
      id: "dl-1",
      title: "Rust & Tauri v2: Building High Performance Desktop Apps",
      quality: "1080p (MP4)",
      progress: 100,
      speed: "0 KB/s",
      status: "completed"
    },
    {
      id: "dl-2",
      title: "Svelte 5 Runes Deep Dive: $state and $derived Masterclass",
      quality: "Audio (320k MP3)",
      progress: 68,
      speed: "4.8 MB/s",
      status: "downloading"
    }
  ]);

  // Sample history
  let historyItems = $state([
    {
      id: "h-1",
      title: "How Operating Systems Work - 10 Hour Full Course",
      channel: "freeCodeCamp",
      date: "Today, 14:20",
      format: "1080p MP4",
      size: "1.8 GB"
    },
    {
      id: "h-2",
      title: "Lofi Hip Hop Radio - Beats to Relax/Study to",
      channel: "Lofi Girl",
      date: "Yesterday",
      format: "Audio MP3",
      size: "142 MB"
    }
  ]);

  function analyzeUrl() {
    if (!urlInput.trim()) return;
    isAnalyzing = true;

    // Simulated fast metadata resolution
    setTimeout(() => {
      currentVideo = {
        id: "yt-sample-1",
        url: urlInput.trim(),
        title: "OmniGet Architecture: Native In-Process IPC vs Worker Sidecars",
        author: "OpenSelena Engineering",
        duration: "14:28",
        thumbnail: "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=600&auto=format&fit=crop&q=80",
        views: "128,450 views",
        formats: [
          { id: "4k", label: "4K Ultra HD (2160p)", ext: "mp4", quality: "2160p60", size: "1.2 GB", type: "video" },
          { id: "1080p", label: "Full HD (1080p)", ext: "mp4", quality: "1080p60", size: "340 MB", type: "video" },
          { id: "720p", label: "HD (720p)", ext: "mp4", quality: "720p", size: "180 MB", type: "video" },
          { id: "mp3-320", label: "Audio Only (320kbps MP3)", ext: "mp3", quality: "320 kbps", size: "32 MB", type: "audio" },
          { id: "opus", label: "Audio Only (Lossless OPUS)", ext: "opus", quality: "160 kbps", size: "24 MB", type: "audio" }
        ]
      };
      isAnalyzing = false;
    }, 600);
  }

  function startDownload() {
    if (!currentVideo) return;
    const format = currentVideo.formats.find(f => f.id === activeFormat) || currentVideo.formats[0];

    const newTask: DownloadTask = {
      id: "dl-" + Date.now(),
      title: currentVideo.title,
      quality: `${format.quality} (${format.ext.toUpperCase()})`,
      progress: 0,
      speed: "6.2 MB/s",
      status: "downloading"
    };

    downloadQueue = [newTask, ...downloadQueue];

    // Simulate progress
    const interval = setInterval(() => {
      const task = downloadQueue.find(t => t.id === newTask.id);
      if (task) {
        if (task.progress < 100) {
          task.progress += 10;
        } else {
          task.status = "completed";
          task.speed = "0 KB/s";
          clearInterval(interval);
        }
      }
    }, 400);
  }

  async function pasteClipboard() {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        urlInput = text;
        analyzeUrl();
      }
    } catch {
      // ignore
    }
  }
</script>

<svelte:head>
  <title>OmniTube - High Speed Media Extractor</title>
</svelte:head>

<div class="youtube-container">
  <!-- Top Header Navigation -->
  <header class="page-header">
    <div class="header-left">
      <a href="/" class="nav-back" title="Back to Dashboard">
        <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
          <path d="M19 12H5M12 19l-7-7 7-7" />
        </svg>
      </a>
      <div class="title-group">
        <h1 class="page-title">OmniTube</h1>
        <span class="page-subtitle">Zero-Throttle Video & Audio Extractor</span>
      </div>
    </div>

    <!-- Navigation Tabs -->
    <nav class="sub-tabs">
      <button
        type="button"
        class="tab-link"
        class:active={selectedTab === "extractor"}
        onclick={() => (selectedTab = "extractor")}
      >
        Extractor
      </button>
      <button
        type="button"
        class="tab-link"
        class:active={selectedTab === "playlist"}
        onclick={() => (selectedTab = "playlist")}
      >
        Batch & Playlists
      </button>
      <button
        type="button"
        class="tab-link"
        class:active={selectedTab === "history"}
        onclick={() => (selectedTab = "history")}
      >
        Archive & History ({historyItems.length})
      </button>
    </nav>
  </header>

  <!-- URL Input Hero Bar -->
  <section class="url-section">
    <div class="input-card">
      <div class="input-wrapper">
        <svg viewBox="0 0 24 24" width="20" height="20" class="input-icon" stroke="currentColor" fill="none" stroke-width="2">
          <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
          <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
        </svg>

        <input
          type="text"
          class="url-input"
          placeholder="Paste YouTube, Vimeo, TikTok, or Bilibili URL..."
          bind:value={urlInput}
          onkeydown={(e) => e.key === "Enter" && analyzeUrl()}
        />

        <button type="button" class="btn-paste" onclick={pasteClipboard} title="Paste from clipboard">
          Paste
        </button>

        <button
          type="button"
          class="btn-analyze"
          onclick={analyzeUrl}
          disabled={!urlInput.trim() || isAnalyzing}
        >
          {#if isAnalyzing}
            <span class="spinner"></span>
            Probing...
          {:else}
            Inspect
          {/if}
        </button>
      </div>
    </div>
  </section>

  <!-- Main View Grid -->
  <div class="content-grid">
    <!-- Left Column: Video Result & Format Picker -->
    <main class="primary-col">
      {#if currentVideo}
        <div class="result-card">
          <div class="thumb-container">
            <img src={currentVideo.thumbnail} alt={currentVideo.title} class="video-thumb" />
            <span class="duration-badge">{currentVideo.duration}</span>
          </div>

          <div class="video-details">
            <h2 class="result-title">{currentVideo.title}</h2>
            <div class="result-meta">
              <span class="author">{currentVideo.author}</span>
              <span class="bullet">·</span>
              <span class="views">{currentVideo.views}</span>
            </div>

            <!-- Format Selector -->
            <div class="formats-box">
              <h3 class="formats-heading">Select Output Quality:</h3>
              <div class="formats-list">
                {#each currentVideo.formats as fmt}
                  <button
                    type="button"
                    class="format-chip"
                    class:selected={activeFormat === fmt.id}
                    onclick={() => (activeFormat = fmt.id)}
                  >
                    <span class="fmt-badge" class:audio={fmt.type === "audio"}>{fmt.ext.toUpperCase()}</span>
                    <span class="fmt-label">{fmt.label}</span>
                    <span class="fmt-size">{fmt.size}</span>
                  </button>
                {/each}
              </div>
            </div>

            <!-- Download CTA -->
            <div class="cta-row">
              <button type="button" class="btn-download" onclick={startDownload}>
                <svg viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" fill="none" stroke-width="2">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3" />
                </svg>
                Download Now
              </button>

              <a href="/video" class="btn-player-link">
                Open in OmniPlayer
              </a>
            </div>
          </div>
        </div>
      {:else}
        <!-- Empty State Hero -->
        <div class="empty-hero">
          <div class="empty-icon-wrap">
            <svg viewBox="0 0 24 24" width="48" height="48" stroke="currentColor" fill="none" stroke-width="1.5">
              <polygon points="5 3 19 12 5 21 5 3" />
            </svg>
          </div>
          <h3>Ready for Media Extraction</h3>
          <p>Paste any video, playlist, or music URL above to fetch instant streams and formats.</p>
        </div>
      {/if}
    </main>

    <!-- Right Column: Active Download Queue -->
    <aside class="queue-col">
      <div class="queue-card">
        <div class="queue-header">
          <h3 class="queue-title">Active Downloads</h3>
          <span class="queue-count">{downloadQueue.length} items</span>
        </div>

        <div class="queue-list">
          {#each downloadQueue as task (task.id)}
            <div class="task-card">
              <div class="task-info">
                <span class="task-title">{task.title}</span>
                <span class="task-spec">{task.quality} · {task.speed}</span>
              </div>

              <!-- Progress Bar -->
              <div class="task-progress-bar">
                <div
                  class="progress-fill"
                  class:complete={task.status === "completed"}
                  style:width="{task.progress}%"
                ></div>
              </div>

              <div class="task-footer">
                <span class="task-pct">{task.progress}%</span>
                <span class="task-status" class:done={task.status === "completed"}>
                  {task.status}
                </span>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </aside>
  </div>
</div>

<style>
  .youtube-container {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: #090d16;
    color: #f3f4f6;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  }

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 28px;
    background: #0f172a;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .nav-back {
    all: unset;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
    color: #e5e7eb;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .nav-back:hover {
    background: rgba(255, 255, 255, 0.14);
  }

  .page-title {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
  }

  .page-subtitle {
    font-size: 12px;
    color: #94a3b8;
  }

  .sub-tabs {
    display: flex;
    gap: 6px;
    background: rgba(255, 255, 255, 0.05);
    padding: 4px;
    border-radius: 8px;
  }

  .tab-link {
    all: unset;
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    color: #94a3b8;
    cursor: pointer;
  }

  .tab-link.active {
    background: #0284c7;
    color: #fff;
  }

  /* URL Section */
  .url-section {
    padding: 24px 28px 12px;
  }

  .input-card {
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    padding: 6px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.3);
  }

  .input-wrapper {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 10px;
  }

  .input-icon {
    color: #64748b;
  }

  .url-input {
    flex: 1;
    background: transparent;
    border: none;
    color: #fff;
    font-size: 14px;
    outline: none;
  }

  .btn-paste {
    all: unset;
    padding: 6px 12px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-paste:hover {
    background: rgba(255, 255, 255, 0.14);
  }

  .btn-analyze {
    all: unset;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 20px;
    background: #0284c7;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-analyze:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid #fff;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Content Grid */
  .content-grid {
    display: grid;
    grid-template-columns: 1fr 340px;
    gap: 24px;
    padding: 16px 28px 32px;
    flex: 1;
  }

  .result-card {
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .thumb-container {
    position: relative;
    width: 100%;
    height: 280px;
    background: #000;
  }

  .video-thumb {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .duration-badge {
    position: absolute;
    bottom: 12px;
    right: 12px;
    background: rgba(0, 0, 0, 0.85);
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 700;
  }

  .video-details {
    padding: 20px;
  }

  .result-title {
    margin: 0 0 8px;
    font-size: 18px;
    font-weight: 600;
    line-height: 1.4;
  }

  .result-meta {
    font-size: 13px;
    color: #94a3b8;
    margin-bottom: 20px;
  }

  .bullet {
    margin: 0 6px;
  }

  .formats-box {
    margin-bottom: 24px;
  }

  .formats-heading {
    margin: 0 0 10px;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
  }

  .formats-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .format-chip {
    all: unset;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.07);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .format-chip:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .format-chip.selected {
    background: rgba(2, 132, 199, 0.15);
    border-color: #0284c7;
  }

  .fmt-badge {
    font-size: 11px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    background: #0284c7;
    color: #fff;
  }

  .fmt-badge.audio {
    background: #a855f7;
  }

  .fmt-label {
    flex: 1;
    font-size: 13px;
    font-weight: 500;
  }

  .fmt-size {
    font-size: 12px;
    color: #94a3b8;
  }

  .cta-row {
    display: flex;
    gap: 12px;
  }

  .btn-download {
    all: unset;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 24px;
    border-radius: 8px;
    background: #0284c7;
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-player-link {
    all: unset;
    display: flex;
    align-items: center;
    padding: 10px 18px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.06);
    color: #e2e8f0;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }

  /* Empty Hero */
  .empty-hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 380px;
    background: rgba(30, 41, 59, 0.5);
    border: 2px dashed rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    text-align: center;
    padding: 24px;
  }

  .empty-icon-wrap {
    color: #64748b;
    margin-bottom: 16px;
  }

  .empty-hero h3 {
    margin: 0 0 6px;
    font-size: 16px;
  }

  .empty-hero p {
    margin: 0;
    font-size: 13px;
    color: #94a3b8;
    max-width: 380px;
  }

  /* Queue Column */
  .queue-card {
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    padding: 18px;
    height: fit-content;
  }

  .queue-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .queue-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .queue-count {
    font-size: 12px;
    color: #94a3b8;
  }

  .queue-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .task-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    padding: 10px 12px;
  }

  .task-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 8px;
  }

  .task-title {
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .task-spec {
    font-size: 11px;
    color: #94a3b8;
  }

  .task-progress-bar {
    width: 100%;
    height: 4px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 9999px;
    overflow: hidden;
    margin-bottom: 6px;
  }

  .progress-fill {
    height: 100%;
    background: #0284c7;
    transition: width 0.3s ease;
  }

  .progress-fill.complete {
    background: #10b981;
  }

  .task-footer {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: #94a3b8;
  }

  .task-status.done {
    color: #10b981;
    font-weight: 600;
  }
</style>
