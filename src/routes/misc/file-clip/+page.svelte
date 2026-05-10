<script lang="ts">
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { emit } from "@tauri-apps/api/event";

  type ClipResult = {
    output_path: string;
    duration_secs: number;
    size_bytes: number;
    used_reencode: boolean;
  };

  let sourcePath = $state<string | null>(null);
  let sourceLabel = $state<string>("");
  let videoSrc = $state<string>("");
  let videoEl = $state<HTMLVideoElement | null>(null);
  let videoDuration = $state<number>(0);
  let videoCurrent = $state<number>(0);
  let startSecs = $state<number>(0);
  let endSecs = $state<number>(30);
  let durationSecs = $derived(Math.max(0.5, endSecs - startSecs));
  let busy = $state(false);
  let lastError = $state<string | null>(null);
  let lastResult = $state<ClipResult | null>(null);

  async function pickFile() {
    try {
      const picked = await openDialog({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Video",
            extensions: ["mp4", "mkv", "webm", "mov", "m4v", "avi"],
          },
        ],
      });
      if (typeof picked === "string" && picked) {
        sourcePath = picked;
        const parts = picked.replace(/\\/g, "/").split("/");
        sourceLabel = parts[parts.length - 1] ?? picked;
        try {
          videoSrc = convertFileSrc(picked);
        } catch {
          videoSrc = "";
        }
        lastResult = null;
        lastError = null;
      }
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    }
  }

  function onVideoLoaded() {
    if (!videoEl) return;
    videoDuration = videoEl.duration || 0;
    if (endSecs <= 0 || endSecs > videoDuration) {
      endSecs = Math.min(30, videoDuration);
    }
  }

  function onVideoTimeUpdate() {
    if (!videoEl) return;
    videoCurrent = videoEl.currentTime;
  }

  function setStartHere() {
    startSecs = Math.floor(videoCurrent);
    if (endSecs <= startSecs) endSecs = Math.min(startSecs + 30, videoDuration);
  }

  function setEndHere() {
    endSecs = Math.ceil(videoCurrent);
    if (startSecs >= endSecs) startSecs = Math.max(0, endSecs - 30);
  }

  function jumpToStart() {
    if (videoEl) videoEl.currentTime = startSecs;
  }

  function jumpToEnd() {
    if (videoEl) videoEl.currentTime = endSecs;
  }

  async function doClip() {
    if (!sourcePath || busy) return;
    busy = true;
    lastError = null;
    lastResult = null;
    try {
      const result = await invoke<ClipResult>("clip_video", {
        req: {
          source_path: sourcePath,
          start_secs: Number(startSecs),
          duration_secs: Number(durationSecs),
          dest_dir: null,
          label: "fileclip",
          reencode: null,
        },
      });
      lastResult = result;

      const queueId = Date.now() * 1000;
      const filename =
        result.output_path.replace(/\\/g, "/").split("/").pop() ??
        "clip.mp4";
      await emit("host:queue:external:enqueue", {
        queue_id: queueId,
        platform: "studio",
        title: filename,
        output_path: result.output_path,
        total_bytes: result.size_bytes,
        kind: "video",
      });
      await emit("host:queue:external:complete", {
        queue_id: queueId,
        success: true,
        file_path: result.output_path,
        file_size_bytes: result.size_bytes,
      });
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function revealResult() {
    if (!lastResult) return;
    try {
      await invoke("reveal_file", { path: lastResult.output_path });
    } catch (e) {
      console.warn("[file-clip] reveal failed:", e);
    }
  }

  function fmtTime(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }
</script>

<section class="clip-page">
  <header class="head">
    <h1>File clip</h1>
    <p class="subtitle">
      Pick a video file and extract a portion. Uses fast stream copy when possible (no quality loss).
    </p>
  </header>

  <section class="picker-card">
    <div class="picker-row">
      <button type="button" class="pick-btn" onclick={pickFile} disabled={busy}>
        {sourcePath ? "Change file" : "Pick video file"}
      </button>
      {#if sourceLabel}
        <span class="filename mono" title={sourcePath ?? ""}>{sourceLabel}</span>
      {:else}
        <span class="filename muted">No file selected</span>
      {/if}
    </div>

    {#if sourcePath && videoSrc}
      <div class="player-wrap">
        <video
          bind:this={videoEl}
          src={videoSrc}
          controls
          onloadedmetadata={onVideoLoaded}
          ontimeupdate={onVideoTimeUpdate}
          class="player"
        ></video>
        <div class="player-time mono">
          <span>Now: {fmtTime(videoCurrent)}</span>
          {#if videoDuration > 0}
            <span class="muted"> / {fmtTime(videoDuration)}</span>
          {/if}
        </div>
      </div>

      <div class="markers">
        <div class="marker">
          <span class="marker-label">In</span>
          <span class="marker-value mono">{fmtTime(startSecs)}</span>
          <button type="button" class="marker-btn" onclick={setStartHere} disabled={busy}>
            Set here
          </button>
          <button type="button" class="marker-btn" onclick={jumpToStart} disabled={busy}>
            ↩ Jump
          </button>
        </div>
        <div class="marker">
          <span class="marker-label">Out</span>
          <span class="marker-value mono">{fmtTime(endSecs)}</span>
          <button type="button" class="marker-btn" onclick={setEndHere} disabled={busy}>
            Set here
          </button>
          <button type="button" class="marker-btn" onclick={jumpToEnd} disabled={busy}>
            ↩ Jump
          </button>
        </div>
        <div class="marker dur">
          <span class="marker-label">Duration</span>
          <span class="marker-value mono">{fmtTime(durationSecs)}</span>
        </div>
      </div>

      <button type="button" class="clip-btn" onclick={doClip} disabled={busy || durationSecs < 0.5}>
        {busy ? "Clipping…" : `✂ Clip ${fmtTime(durationSecs)} (${fmtTime(startSecs)} → ${fmtTime(endSecs)})`}
      </button>
    {/if}
  </section>

  {#if lastError}
    <div class="error" role="alert"><strong>Error:</strong> {lastError}</div>
  {/if}

  {#if lastResult}
    <section class="result-card">
      <h2>Clip ready</h2>
      <div class="result-stats">
        <div class="stat">
          <span class="stat-label">Duration</span>
          <span class="stat-value mono">{fmtTime(lastResult.duration_secs)}</span>
        </div>
        <div class="stat">
          <span class="stat-label">Size</span>
          <span class="stat-value mono">{fmtSize(lastResult.size_bytes)}</span>
        </div>
        <div class="stat">
          <span class="stat-label">Method</span>
          <span class="stat-value mono">
            {lastResult.used_reencode ? "Re-encoded" : "Stream copy"}
          </span>
        </div>
      </div>
      <div class="result-path mono" title={lastResult.output_path}>
        {lastResult.output_path}
      </div>
      <div class="result-actions">
        <a href="/downloads" class="btn-link">View in queue</a>
        <button type="button" class="btn-reveal" onclick={revealResult}>Open folder</button>
      </div>
    </section>
  {/if}

  <p class="hint">
    Tip: stream copy is instant but starts at the nearest keyframe (might begin 1-2s before/after the
    requested start). For precise cuts, the system falls back to re-encoding automatically.
  </p>
</section>

<style>
  .clip-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    width: 100%;
    max-width: 760px;
    margin-inline: auto;
    padding: 16px 20px 80px;
  }
  .head h1 {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 500;
    letter-spacing: -0.5px;
    color: var(--secondary);
  }
  .subtitle {
    margin: 0;
    font-size: 13px;
    color: var(--tertiary);
  }
  .picker-card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
  }
  .picker-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .pick-btn {
    padding: 10px 18px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--secondary);
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
  }
  .pick-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .filename {
    font-size: 12px;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .muted {
    color: var(--tertiary);
  }
  .player-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .player {
    width: 100%;
    max-height: 480px;
    background: black;
    border-radius: var(--border-radius);
  }
  .player-time {
    font-size: 12px;
    color: var(--secondary);
    text-align: right;
  }
  .markers {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 10px;
  }
  .marker {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
  }
  .marker.dur {
    border-style: dashed;
    background: color-mix(in oklab, var(--accent) 4%, var(--bg));
  }
  .marker-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--tertiary);
  }
  .marker-value {
    font-size: 16px;
    color: var(--secondary);
  }
  .marker-btn {
    padding: 4px 10px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--secondary);
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
  }
  .marker-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .marker-btn:disabled {
    opacity: 0.5;
  }
  .clip-btn {
    padding: 12px 18px;
    border: 1px solid var(--accent);
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--accent) 14%, var(--bg));
    color: var(--accent);
    font-size: 14px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
  }
  .clip-btn:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 22%, var(--bg));
  }
  .clip-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .error {
    padding: 10px 14px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, oklch(60% 0.22 25) 14%, transparent);
    color: oklch(70% 0.22 25);
    font-size: 13px;
  }
  .result-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px;
    border: 1px solid color-mix(in oklab, oklch(70% 0.18 145) 60%, var(--input-border));
    border-radius: var(--border-radius);
    background: color-mix(in oklab, oklch(70% 0.18 145) 4%, var(--surface));
  }
  .result-card h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--secondary);
  }
  .result-stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .stat-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--tertiary);
  }
  .stat-value {
    font-size: 13px;
    color: var(--secondary);
  }
  .mono {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-variant-numeric: tabular-nums;
  }
  .result-path {
    font-size: 11px;
    color: var(--tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .result-actions {
    display: flex;
    gap: 10px;
  }
  .btn-link,
  .btn-reveal {
    padding: 6px 14px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--secondary);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    text-decoration: none;
  }
  .btn-link:hover,
  .btn-reveal:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--tertiary);
    line-height: 1.5;
  }
</style>
