<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { pluginInvoke } from "$lib/plugin-invoke";

  type RecordingState = "idle" | "recording" | "stopping";

  type SessionStats = {
    video_frames: number;
    video_packets: number;
    audio_chunks: number;
    audio_samples: number;
    audio_packets: number;
    duration_secs: number;
    file_size_bytes: number;
    output_path: string;
  };

  type ClipResult = {
    queue_id: number;
    title: string;
    output_path: string;
    stats: SessionStats;
    saved_at: number;
  };

  type ReplayStatus = {
    running: boolean;
    buffer_secs: number;
    packet_count: number;
    bytes_used: number;
    max_secs: number;
  };

  let recState = $state<RecordingState>("idle");
  let elapsedSecs = $state(0);
  let outputPath = $state<string | null>(null);
  let lastError = $state<string | null>(null);
  let recentClips = $state<ClipResult[]>([]);

  let replayStatus = $state<ReplayStatus>({
    running: false,
    buffer_secs: 0,
    packet_count: 0,
    bytes_used: 0,
    max_secs: 30,
  });
  let replayBusy = $state(false);

  let timer: number | null = null;
  let replayPoller: number | null = null;
  let unlistenSaved: UnlistenFn | null = null;
  let unlistenStarted: UnlistenFn | null = null;
  let unlistenTranscribe: UnlistenFn | null = null;
  let unlistenTranscribeProgress: UnlistenFn | null = null;
  let startedAt = 0;

  type TranscribeState = "idle" | "running" | "done" | "error";
  let transcribeStates = $state<Record<number, TranscribeState>>({});
  let transcribeMessages = $state<Record<number, string>>({});
  let transcribeSrtPaths = $state<Record<number, string>>({});
  let transcribeProgress = $state<Record<number, number>>({});

  type TranslateState = "idle" | "running" | "done" | "error";
  let translateStates = $state<Record<number, TranslateState>>({});
  let translateMessages = $state<Record<number, string>>({});
  let translateTarget = $state<Record<number, string>>({});
  let translateSource = $state<Record<number, string>>({});
  let detectedLanguage = $state<Record<number, string>>({});

  onMount(() => {
    void initListeners();
    void refreshStatus();
    void refreshReplayStatus();
    replayPoller = window.setInterval(() => void refreshReplayStatus(), 1000);
  });

  onDestroy(() => {
    stopTimer();
    if (replayPoller != null) clearInterval(replayPoller);
    unlistenSaved?.();
    unlistenStarted?.();
    unlistenTranscribe?.();
    unlistenTranscribeProgress?.();
  });

  async function initListeners() {
    unlistenStarted = await listen("misc:studio:recording-started", (e) => {
      const payload = e.payload as { output_path: string };
      outputPath = payload.output_path;
    });
    unlistenSaved = await listen("misc:studio:clip-saved", (e) => {
      const payload = e.payload as Omit<ClipResult, "saved_at">;
      const clip: ClipResult = { ...payload, saved_at: Date.now() };
      recentClips = [clip, ...recentClips].slice(0, 8);
    });
    unlistenTranscribeProgress = await listen("misc:transcribe:progress", (e) => {
      const p = e.payload as { queue_id: number; percent: number };
      transcribeProgress = { ...transcribeProgress, [p.queue_id]: p.percent };
    });
    unlistenTranscribe = await listen("misc:transcribe:done", (e) => {
      const p = e.payload as {
        queue_id: number;
        srt_path: string;
        elapsed_s: number;
        duration_s: number;
        segment_count: number;
        language: string;
      };
      transcribeStates = { ...transcribeStates, [p.queue_id]: "done" };
      transcribeSrtPaths = { ...transcribeSrtPaths, [p.queue_id]: p.srt_path };
      detectedLanguage = { ...detectedLanguage, [p.queue_id]: p.language };
      translateSource = { ...translateSource, [p.queue_id]: p.language };
      transcribeMessages = {
        ...transcribeMessages,
        [p.queue_id]: `${p.segment_count} segmentos · ${p.elapsed_s.toFixed(1)}s · idioma: ${p.language} → ${p.srt_path}`,
      };
    });
  }

  async function translateClip(clip: ClipResult, target: string, source: string) {
    const srtPath = transcribeSrtPaths[clip.queue_id];
    if (!srtPath) {
      lastError = "Transcribe first to generate the SRT file.";
      return;
    }
    if (translateStates[clip.queue_id] === "running") return;
    if (source === target) {
      lastError = `Source and target are both '${source}'. Pick a different target.`;
      return;
    }
    translateStates = { ...translateStates, [clip.queue_id]: "running" };
    translateMessages = {
      ...translateMessages,
      [clip.queue_id]: `Translating ${source} → ${target}…`,
    };
    try {
      const res = await pluginInvoke<{ output_path: string; elapsed_s: number; block_count: number }>(
        "misc",
        "misc:translate:srt",
        {
          srtPath,
          targetLanguage: target,
          sourceLanguage: source,
        },
      );
      translateStates = { ...translateStates, [clip.queue_id]: "done" };
      translateMessages = {
        ...translateMessages,
        [clip.queue_id]: `${res.block_count} blocos em ${res.elapsed_s.toFixed(1)}s → ${res.output_path}`,
      };
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      translateStates = { ...translateStates, [clip.queue_id]: "error" };
      translateMessages = { ...translateMessages, [clip.queue_id]: msg };
      if (msg.includes("model not installed")) {
        lastError = "Translation model not installed. Go to /misc/models and install 'M2M-100 418M (ONNX)' (~870 MB).";
      } else if (msg.includes("not found as token")) {
        lastError = `Language code '${source}' not supported by M2M-100. Use ISO codes like en, pt, es, fr, de, it, ja, zh, ko, ru, ar, hi.`;
      } else {
        lastError = msg;
      }
    }
  }

  const languageOptions = [
    { value: "en", label: "English (en)" },
    { value: "pt", label: "Portuguese (pt)" },
    { value: "es", label: "Spanish (es)" },
    { value: "fr", label: "French (fr)" },
    { value: "de", label: "German (de)" },
    { value: "it", label: "Italian (it)" },
    { value: "ja", label: "Japanese (ja)" },
    { value: "zh", label: "Chinese (zh)" },
    { value: "ko", label: "Korean (ko)" },
    { value: "ru", label: "Russian (ru)" },
    { value: "ar", label: "Arabic (ar)" },
    { value: "hi", label: "Hindi (hi)" },
  ];

  async function transcribeClip(clip: ClipResult) {
    if (transcribeStates[clip.queue_id] === "running") return;
    transcribeStates = { ...transcribeStates, [clip.queue_id]: "running" };
    transcribeMessages = {
      ...transcribeMessages,
      [clip.queue_id]: "Extracting audio…",
    };
    transcribeProgress = { ...transcribeProgress, [clip.queue_id]: 0 };
    try {
      await pluginInvoke("misc", "misc:transcribe:run", {
        path: clip.output_path,
        wordTimestamps: true,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      transcribeStates = { ...transcribeStates, [clip.queue_id]: "error" };
      transcribeMessages = { ...transcribeMessages, [clip.queue_id]: msg };
      if (msg.includes("model not installed")) {
        lastError = `Whisper model not installed. Go to /misc/models and install 'Whisper Tiny' (~75 MB) or 'Large v3 Turbo' (~800 MB).`;
      } else if (msg.includes("aborted")) {
        transcribeMessages = { ...transcribeMessages, [clip.queue_id]: "Cancelled" };
      } else {
        lastError = msg;
      }
    }
  }

  async function cancelTranscribe(clip: ClipResult) {
    try {
      await pluginInvoke("misc", "misc:transcribe:cancel", {});
    } catch (e) {
      console.warn("[studio] cancel failed:", e);
    }
  }

  async function refreshStatus() {
    try {
      const res = await pluginInvoke<{
        recording: boolean;
        elapsed_secs: number;
        output_path: string | null;
      }>("misc", "misc:studio:status", {});
      if (res.recording) {
        recState = "recording";
        elapsedSecs = res.elapsed_secs;
        outputPath = res.output_path;
        startedAt = Date.now() - res.elapsed_secs * 1000;
        startTimer();
      }
    } catch (e) {
      console.warn("[studio] status refresh failed:", e);
    }
  }

  async function refreshReplayStatus() {
    try {
      replayStatus = await pluginInvoke<ReplayStatus>(
        "misc",
        "misc:studio:replay_buffer:status",
        {},
      );
    } catch (e) {
      console.warn("[studio] replay status failed:", e);
    }
  }

  function startTimer() {
    stopTimer();
    timer = window.setInterval(() => {
      elapsedSecs = (Date.now() - startedAt) / 1000;
    }, 200);
  }

  function stopTimer() {
    if (timer != null) {
      clearInterval(timer);
      timer = null;
    }
  }

  async function startRecording() {
    if (recState !== "idle") return;
    lastError = null;
    recState = "recording";
    elapsedSecs = 0;
    startedAt = Date.now();
    try {
      const res = await pluginInvoke<{ ok: boolean; output_path: string }>(
        "misc",
        "misc:studio:start_recording",
        {},
      );
      outputPath = res.output_path;
      startTimer();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
      recState = "idle";
      stopTimer();
    }
  }

  async function stopRecording() {
    if (recState !== "recording") return;
    recState = "stopping";
    stopTimer();
    try {
      await pluginInvoke("misc", "misc:studio:stop_recording", {});
      recState = "idle";
      elapsedSecs = 0;
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
      recState = "idle";
    }
  }

  async function toggleReplayBuffer() {
    if (replayBusy) return;
    replayBusy = true;
    lastError = null;
    try {
      if (replayStatus.running) {
        await pluginInvoke("misc", "misc:studio:replay_buffer:stop", {});
      } else {
        await pluginInvoke("misc", "misc:studio:replay_buffer:start", {
          maxSecs: 35,
          maxMb: 120,
        });
      }
      await refreshReplayStatus();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    } finally {
      replayBusy = false;
    }
  }

  async function saveReplay() {
    if (replayBusy || !replayStatus.running) return;
    replayBusy = true;
    lastError = null;
    try {
      await pluginInvoke("misc", "misc:studio:replay_buffer:save", {});
      await refreshReplayStatus();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    } finally {
      replayBusy = false;
    }
  }

  async function revealFile(path: string) {
    try {
      await invoke("reveal_file", { path });
    } catch (e) {
      console.warn("[studio] reveal_file failed:", e);
    }
  }

  function fmtTime(secs: number): string {
    const total = Math.floor(secs);
    const m = Math.floor(total / 60);
    const s = total % 60;
    return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function fmtRel(ts: number): string {
    const diff = Math.max(0, Date.now() - ts);
    if (diff < 60_000) return `${Math.floor(diff / 1000)}s ago`;
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    return new Date(ts).toLocaleTimeString();
  }
</script>

<section class="studio-page">
  <header class="head">
    <h1>Studio</h1>
    <p class="subtitle">
      Record screen + system audio (what you hear). Replay buffer keeps the last 30s in RAM.
    </p>
  </header>

  <section class="mode-card replay-card" class:armed={replayStatus.running}>
    <div class="mode-head">
      <div class="mode-title">
        <span class="dot" class:on={replayStatus.running}></span>
        Replay buffer
      </div>
      <button
        type="button"
        class="toggle-btn"
        class:active={replayStatus.running}
        onclick={toggleReplayBuffer}
        disabled={replayBusy}
      >
        {replayStatus.running ? "Turn off" : "Turn on"}
      </button>
    </div>
    <p class="mode-desc">
      Always recording in background. Press "Save last 30s" any time to clip what just happened — like ShadowPlay.
    </p>
    <div class="replay-stats">
      <div class="stat">
        <span class="stat-label">Buffered</span>
        <span class="stat-value mono">
          {fmtTime(replayStatus.buffer_secs)} / {fmtTime(replayStatus.max_secs)}
        </span>
      </div>
      <div class="stat">
        <span class="stat-label">RAM</span>
        <span class="stat-value mono">{fmtSize(replayStatus.bytes_used)}</span>
      </div>
      <div class="stat">
        <span class="stat-label">Packets</span>
        <span class="stat-value mono">{replayStatus.packet_count}</span>
      </div>
    </div>
    <button
      type="button"
      class="save-btn"
      class:disabled={!replayStatus.running || replayBusy}
      onclick={saveReplay}
      disabled={!replayStatus.running || replayBusy}
    >
      ✂ Save last {Math.floor(replayStatus.buffer_secs)}s
    </button>
  </section>

  <section class="mode-card">
    <div class="mode-head">
      <div class="mode-title">Manual recording</div>
    </div>
    <p class="mode-desc">
      Start/stop recording. Use this when you know in advance what you want to capture.
    </p>
    <div class="rec-stage">
      <button
        type="button"
        class="rec-btn"
        class:active={recState === "recording"}
        onclick={recState === "recording" ? stopRecording : startRecording}
        disabled={recState === "stopping"}
      >
        <span class="rec-dot"></span>
        <span class="rec-label">
          {#if recState === "idle"}
            Start recording
          {:else if recState === "recording"}
            Stop recording
          {:else}
            Finalizing…
          {/if}
        </span>
      </button>
      <div class="elapsed">
        {#if recState === "recording" || recState === "stopping"}
          <span class="elapsed-time mono">{fmtTime(elapsedSecs)}</span>
        {:else}
          <span class="elapsed-idle">Ready</span>
        {/if}
      </div>
    </div>
  </section>

  {#if lastError}
    <div class="error" role="alert"><strong>Error:</strong> {lastError}</div>
  {/if}

  <section class="recent">
    <h2>Recent clips</h2>
    {#if recentClips.length === 0}
      <p class="empty">No clips yet. Turn on the replay buffer or start a manual recording.</p>
    {:else}
      <ul class="clip-list">
        {#each recentClips as clip (clip.queue_id)}
          <li class="clip-row">
            <div class="clip-meta">
              <span class="clip-title">{clip.title}</span>
              <span class="clip-stats">
                {fmtTime(clip.stats.duration_secs)} · {fmtSize(clip.stats.file_size_bytes)}
                {#if clip.stats.video_frames > 0}
                  · {clip.stats.video_frames} frames
                {/if}
              </span>
              <span class="clip-path mono" title={clip.output_path}>{clip.output_path}</span>
            </div>
            <div class="clip-actions">
              <span class="clip-when">{fmtRel(clip.saved_at)}</span>
              {#if transcribeStates[clip.queue_id] === "running"}
                <button
                  type="button"
                  class="btn-reveal"
                  onclick={() => cancelTranscribe(clip)}
                  title="Cancel running transcription"
                >
                  Cancel
                </button>
              {:else}
                <button
                  type="button"
                  class="btn-reveal"
                  onclick={() => transcribeClip(clip)}
                  title="Generate SRT + VTT next to the clip"
                >
                  {#if transcribeStates[clip.queue_id] === "done"}
                    ✓ Transcribed
                  {:else}
                    Transcribe
                  {/if}
                </button>
              {/if}
              <button type="button" class="btn-reveal" onclick={() => revealFile(clip.output_path)}>
                Open folder
              </button>
            </div>
            {#if transcribeMessages[clip.queue_id]}
              <div class="transcribe-msg" class:error={transcribeStates[clip.queue_id] === "error"}>
                {#if transcribeStates[clip.queue_id] === "running"}
                  <div class="progress-line">
                    <div class="progress-bar-mini">
                      <div
                        class="progress-fill-mini"
                        style="width: {transcribeProgress[clip.queue_id] ?? 0}%"
                      ></div>
                    </div>
                    <span class="progress-pct">{transcribeProgress[clip.queue_id] ?? 0}%</span>
                  </div>
                {/if}
                {transcribeMessages[clip.queue_id]}
              </div>
            {/if}
            {#if transcribeStates[clip.queue_id] === "done"}
              <div class="translate-row">
                {#if detectedLanguage[clip.queue_id]}
                  <span class="detected-lang">
                    Detected: <strong>{detectedLanguage[clip.queue_id]}</strong>
                  </span>
                {/if}
                <select
                  class="translate-select"
                  bind:value={translateSource[clip.queue_id]}
                  title="Source language (override detected)"
                >
                  {#each languageOptions as opt (opt.value)}
                    <option value={opt.value}>{opt.label}</option>
                  {/each}
                </select>
                <span class="arrow">→</span>
                <select
                  class="translate-select"
                  bind:value={translateTarget[clip.queue_id]}
                  title="Target language"
                >
                  {#each languageOptions as opt (opt.value)}
                    <option value={opt.value}>{opt.label}</option>
                  {/each}
                </select>
                <button
                  type="button"
                  class="btn-reveal"
                  onclick={() =>
                    translateClip(
                      clip,
                      translateTarget[clip.queue_id] || "pt",
                      translateSource[clip.queue_id] || detectedLanguage[clip.queue_id] || "en",
                    )}
                  disabled={translateStates[clip.queue_id] === "running"}
                >
                  {#if translateStates[clip.queue_id] === "running"}
                    Translating…
                  {:else if translateStates[clip.queue_id] === "done"}
                    ✓ Translated
                  {:else}
                    Translate
                  {/if}
                </button>
              </div>
            {/if}
            {#if translateMessages[clip.queue_id]}
              <div class="transcribe-msg" class:error={translateStates[clip.queue_id] === "error"}>
                {translateMessages[clip.queue_id]}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</section>

<style>
  .studio-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    width: 100%;
    max-width: 880px;
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
  .mode-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 20px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
  }
  .replay-card.armed {
    border-color: oklch(60% 0.22 25);
    background: color-mix(in oklab, oklch(60% 0.22 25) 5%, var(--surface));
  }
  .mode-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .mode-title {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 15px;
    font-weight: 500;
    color: var(--secondary);
  }
  .dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: color-mix(in oklab, var(--input-border) 60%, transparent);
  }
  .dot.on {
    background: oklch(60% 0.22 25);
    animation: pulse 1.4s ease-in-out infinite;
    box-shadow: 0 0 6px oklch(60% 0.22 25 / 0.6);
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
  .mode-desc {
    margin: 0;
    font-size: 12px;
    color: var(--tertiary);
    line-height: 1.4;
  }
  .toggle-btn {
    padding: 6px 14px;
    border-radius: 999px;
    border: 1px solid var(--input-border);
    background: var(--bg);
    color: var(--secondary);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    transition: border-color 120ms ease, color 120ms ease;
  }
  .toggle-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .toggle-btn.active {
    border-color: oklch(60% 0.22 25);
    color: oklch(70% 0.22 25);
  }
  .toggle-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .replay-stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    background: var(--bg);
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
    font-size: 14px;
    font-weight: 500;
    color: var(--secondary);
  }
  .mono {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-variant-numeric: tabular-nums;
  }
  .save-btn {
    padding: 12px 18px;
    border: 1px solid oklch(60% 0.22 25);
    border-radius: var(--border-radius);
    background: color-mix(in oklab, oklch(60% 0.22 25) 14%, var(--bg));
    color: oklch(70% 0.22 25);
    font-size: 14px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: background 150ms ease;
  }
  .save-btn:hover:not(.disabled) {
    background: color-mix(in oklab, oklch(60% 0.22 25) 22%, var(--bg));
  }
  .save-btn.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .rec-stage {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 0;
  }
  .rec-btn {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    padding: 12px 24px;
    border: 1px solid var(--input-border);
    border-radius: 999px;
    background: var(--bg);
    color: var(--secondary);
    font-size: 14px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
  }
  .rec-btn:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .rec-btn.active {
    border-color: oklch(60% 0.22 25);
    color: oklch(70% 0.22 25);
    background: color-mix(in oklab, oklch(60% 0.22 25) 8%, var(--bg));
  }
  .rec-btn:disabled {
    opacity: 0.6;
  }
  .rec-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: oklch(60% 0.22 25);
  }
  .elapsed-time {
    font-size: 18px;
    color: var(--secondary);
  }
  .elapsed-idle {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--tertiary);
    letter-spacing: 0.5px;
  }
  .error {
    padding: 10px 14px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, oklch(60% 0.22 25) 14%, transparent);
    color: oklch(70% 0.22 25);
    font-size: 13px;
  }
  .recent h2 {
    margin: 0 0 12px;
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--tertiary);
  }
  .empty {
    color: var(--tertiary);
    font-size: 13px;
    margin: 0;
  }
  .clip-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .clip-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
  }
  .clip-meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
    min-width: 0;
  }
  .clip-title {
    font-size: 13px;
    color: var(--secondary);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .clip-stats {
    font-size: 11px;
    color: var(--tertiary);
  }
  .clip-path {
    font-size: 10px;
    color: var(--tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.7;
  }
  .clip-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }
  .clip-when {
    font-size: 11px;
    color: var(--tertiary);
  }
  .btn-reveal {
    padding: 6px 12px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--secondary);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
  }
  .btn-reveal:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn-reveal:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .transcribe-msg {
    flex-basis: 100%;
    margin-top: 6px;
    padding: 6px 10px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--secondary);
    font-size: 11px;
    line-height: 1.4;
  }
  .transcribe-msg.error {
    background: color-mix(in oklab, oklch(60% 0.22 25) 14%, transparent);
    color: oklch(70% 0.22 25);
  }
  .translate-row {
    flex-basis: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
    flex-wrap: wrap;
  }
  .translate-select {
    padding: 4px 10px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
    color: var(--secondary);
    font-size: 11px;
    font-family: inherit;
  }
  .translate-select:focus {
    outline: none;
    border-color: var(--accent);
  }
  .detected-lang {
    font-size: 11px;
    color: var(--tertiary);
  }
  .detected-lang strong {
    color: var(--secondary);
    font-weight: 600;
  }
  .arrow {
    font-size: 12px;
    color: var(--tertiary);
  }
  .progress-line {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .progress-bar-mini {
    flex: 1;
    height: 4px;
    background: color-mix(in oklab, var(--input-border) 50%, transparent);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill-mini {
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease;
  }
  .progress-pct {
    font-size: 10px;
    color: var(--tertiary);
    font-variant-numeric: tabular-nums;
    min-width: 32px;
    text-align: right;
  }
  @media (prefers-reduced-motion: reduce) {
    .dot.on {
      animation: none;
    }
  }
</style>
