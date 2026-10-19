<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  interface Chapter {
    id: string;
    title: string;
    startTime: number;
    duration: string;
  }

  interface TranscriptLine {
    id: string;
    start: number;
    text: string;
  }

  interface VideoNote {
    id: string;
    timestamp: number;
    text: string;
    createdAt: string;
  }

  // Reactive Svelte 5 state runes
  let videoEl = $state<HTMLVideoElement | null>(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let volume = $state(1.0);
  let isMuted = $state(false);
  let playbackRate = $state(1.0);
  let isFullscreen = $state(false);
  let isPip = $state(false);
  let showControls = $state(true);
  let controlsTimeout = $state<number | null>(null);
  let osdMessage = $state<string | null>(null);
  let osdTimeout = $state<number | null>(null);

  // A-B Repeat
  let repeatA = $state<number | null>(null);
  let repeatB = $state<number | null>(null);

  // Side Panel
  let activeTab = $state<"chapters" | "transcript" | "notes" | "ai">("chapters");
  let isPanelOpen = $state(true);

  // Scrubber Hover
  let hoverTime = $state<number | null>(null);
  let hoverPct = $state(0);

  // Notes & Transcript
  let noteInput = $state("");
  let notes = $state<VideoNote[]>([]);
  let transcriptQuery = $state("");
  let aiQuery = $state("");
  let aiMessages = $state<Array<{ role: "user" | "ai"; text: string }>>([
    { role: "ai", text: "Hello! Ask me anything about this video, topics discussed, or seek to key timestamps." }
  ]);
  let isAiThinking = $state(false);

  // Sample data
  let videoTitle = $state("Mastering Svelte 5 & Tauri Architecture");
  let videoSource = $state("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4");

  let chapters = $state<Chapter[]>([
    { id: "1", title: "Introduction & Architecture Overview", startTime: 0, duration: "02:15" },
    { id: "2", title: "Reactive Runes ($state, $derived)", startTime: 135, duration: "03:40" },
    { id: "3", title: "In-Process Rust Plugin Engine", startTime: 355, duration: "04:10" },
    { id: "4", title: "Zero Sidecar Optimization & IPC", startTime: 605, duration: "03:20" }
  ]);

  let transcript = $state<TranscriptLine[]>([
    { id: "t1", start: 0, text: "Welcome to this deep-dive into the architectural evolution of OmniGet." },
    { id: "t2", start: 15, text: "In this session, we examine how we eliminate black-box sidecars in favor of native Rust plugins." },
    { id: "t3", start: 45, text: "Notice the responsive UI powered entirely by native Svelte 5 runes." },
    { id: "t4", start: 80, text: "By using in-process IPC dispatching, memory overhead and latency drop to near zero." },
    { id: "t5", start: 135, text: "Let's explore the Svelte 5 state model and fine-grained reactive tracking." },
    { id: "t6", start: 210, text: "Everything from timeline scrubbing to A-B repeat runs smoothly at 60 FPS." },
    { id: "t7", start: 355, text: "Now turning to the backend plugin SDK and asynchronous command dispatching in Rust." }
  ]);

  // Derived calculations
  let filteredTranscript = $derived(
    transcriptQuery.trim() === ""
      ? transcript
      : transcript.filter(t => t.text.toLowerCase().includes(transcriptQuery.toLowerCase()))
  );

  let currentChapter = $derived.by(() => {
    let curr = chapters[0];
    for (const c of chapters) {
      if (currentTime >= c.startTime) curr = c;
    }
    return curr;
  });

  function formatTime(secs: number): string {
    if (isNaN(secs) || secs < 0) return "00:00";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    const h = Math.floor(m / 60);
    const remM = m % 60;
    if (h > 0) {
      return `${h}:${remM.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
    }
    return `${remM.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }

  function triggerOsd(msg: string) {
    osdMessage = msg;
    if (osdTimeout) clearTimeout(osdTimeout);
    osdTimeout = window.setTimeout(() => {
      osdMessage = null;
    }, 1500);
  }

  function togglePlay() {
    if (!videoEl) return;
    if (videoEl.paused) {
      videoEl.play();
      isPlaying = true;
      triggerOsd("Play");
    } else {
      videoEl.pause();
      isPlaying = false;
      triggerOsd("Pause");
    }
  }

  function seek(time: number) {
    if (!videoEl) return;
    const target = Math.max(0, Math.min(duration, time));
    videoEl.currentTime = target;
    currentTime = target;
    triggerOsd(formatTime(target));
  }

  function seekOffset(offset: number) {
    if (!videoEl) return;
    seek(videoEl.currentTime + offset);
  }

  function handleTimeUpdate() {
    if (!videoEl) return;
    currentTime = videoEl.currentTime;

    // A-B Repeat loop check
    if (repeatA !== null && repeatB !== null && repeatB > repeatA) {
      if (currentTime >= repeatB) {
        videoEl.currentTime = repeatA;
      }
    }
  }

  function toggleMute() {
    if (!videoEl) return;
    isMuted = !isMuted;
    videoEl.muted = isMuted;
    triggerOsd(isMuted ? "Muted" : `Volume ${Math.round(volume * 100)}%`);
  }

  function setVolume(v: number) {
    if (!videoEl) return;
    volume = Math.max(0, Math.min(1, v));
    videoEl.volume = volume;
    if (volume > 0 && isMuted) {
      isMuted = false;
      videoEl.muted = false;
    }
    triggerOsd(`Volume ${Math.round(volume * 100)}%`);
  }

  function setSpeed(rate: number) {
    if (!videoEl) return;
    playbackRate = rate;
    videoEl.playbackRate = rate;
    triggerOsd(`${rate}x Speed`);
  }

  function toggleFullscreen() {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen().then(() => {
        isFullscreen = true;
      }).catch(() => {});
    } else {
      document.exitFullscreen().then(() => {
        isFullscreen = false;
      }).catch(() => {});
    }
  }

  async function togglePip() {
    if (!videoEl) return;
    try {
      if (document.pictureInPictureElement) {
        await document.exitPictureInPicture();
        isPip = false;
      } else {
        await videoEl.requestPictureInPicture();
        isPip = true;
      }
    } catch {
      // ignore
    }
  }

  function toggleAbRepeat() {
    if (repeatA === null) {
      repeatA = currentTime;
      triggerOsd(`Set A: ${formatTime(currentTime)}`);
    } else if (repeatB === null) {
      if (currentTime > repeatA) {
        repeatB = currentTime;
        triggerOsd(`Set B: ${formatTime(currentTime)} (Loop Active)`);
      } else {
        repeatA = currentTime;
        triggerOsd(`Reset A: ${formatTime(currentTime)}`);
      }
    } else {
      repeatA = null;
      repeatB = null;
      triggerOsd("Cleared A-B Repeat");
    }
  }

  function addNote() {
    if (!noteInput.trim()) return;
    const newNote: VideoNote = {
      id: "note-" + Date.now(),
      timestamp: currentTime,
      text: noteInput.trim(),
      createdAt: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    };
    notes = [newNote, ...notes];
    noteInput = "";
    triggerOsd("Note added");
  }

  function deleteNote(id: string) {
    notes = notes.filter(n => n.id !== id);
  }

  function sendAiQuestion() {
    if (!aiQuery.trim() || isAiThinking) return;
    const q = aiQuery.trim();
    aiMessages = [...aiMessages, { role: "user", text: q }];
    aiQuery = "";
    isAiThinking = true;

    setTimeout(() => {
      let reply = `Based on the transcript around ${formatTime(currentTime)}: The video highlights the performance advantages of native in-process plugins and Svelte 5 state reactivity over external worker binaries.`;
      aiMessages = [...aiMessages, { role: "ai", text: reply }];
      isAiThinking = false;
    }, 800);
  }

  function handleMouseMove() {
    showControls = true;
    if (controlsTimeout) clearTimeout(controlsTimeout);
    controlsTimeout = window.setTimeout(() => {
      if (isPlaying) showControls = false;
    }, 2800);
  }

  function handleKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;

    switch (e.code) {
      case "Space":
      case "KeyK":
        e.preventDefault();
        togglePlay();
        break;
      case "ArrowLeft":
      case "KeyJ":
        e.preventDefault();
        seekOffset(-10);
        break;
      case "ArrowRight":
      case "KeyL":
        e.preventDefault();
        seekOffset(10);
        break;
      case "ArrowUp":
        e.preventDefault();
        setVolume(volume + 0.05);
        break;
      case "ArrowDown":
        e.preventDefault();
        setVolume(volume - 0.05);
        break;
      case "KeyM":
        e.preventDefault();
        toggleMute();
        break;
      case "KeyF":
        e.preventDefault();
        toggleFullscreen();
        break;
      case "BracketLeft":
        e.preventDefault();
        setSpeed(Math.max(0.25, Number((playbackRate - 0.25).toFixed(2))));
        break;
      case "BracketRight":
        e.preventDefault();
        setSpeed(Math.min(3.0, Number((playbackRate + 0.25).toFixed(2))));
        break;
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    if (typeof window !== "undefined") {
      window.removeEventListener("keydown", handleKeydown);
      if (controlsTimeout) clearTimeout(controlsTimeout);
      if (osdTimeout) clearTimeout(osdTimeout);
    }
  });
</script>

<svelte:head>
  <title>OmniPlayer - {videoTitle}</title>
</svelte:head>

<div
  class="player-wrapper"
  class:fullscreen={isFullscreen}
  class:with-panel={isPanelOpen}
  onmousemove={handleMouseMove}
  role="region"
  aria-label="Video Player"
>
  <!-- Stage & Ambient Background -->
  <div class="video-stage">
    <!-- Ambient Blur Glow -->
    <div
      class="ambient-glow"
      style:background={isPlaying ? "radial-gradient(circle, rgba(14,165,233,0.2) 0%, transparent 70%)" : "none"}
    ></div>

    <!-- Video Element -->
    <!-- svelte-ignore a11y_media_has_caption -->
    <video
      bind:this={videoEl}
      src={videoSource}
      playsinline
      onclick={togglePlay}
      ontimeupdate={handleTimeUpdate}
      onloadedmetadata={() => {
        if (videoEl) duration = videoEl.duration;
      }}
      onplay={() => (isPlaying = true)}
      onpause={() => (isPlaying = false)}
    ></video>

    <!-- OSD Banner -->
    {#if osdMessage}
      <div class="osd-banner">{osdMessage}</div>
    {/if}

    <!-- Top Overlay Bar -->
    <div class="overlay-top" class:hidden={!showControls && isPlaying}>
      <a href="/" class="icon-btn" title="Back to Dashboard">
        <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
          <path d="M19 12H5M12 19l-7-7 7-7" />
        </svg>
      </a>

      <div class="video-meta">
        <h1 class="video-heading">{videoTitle}</h1>
        <span class="video-sub">{currentChapter?.title || "Standalone Media"}</span>
      </div>

      <div class="top-badges">
        <span class="badge hd">1080p 60fps</span>
        <span class="badge codec">H.264 / AAC</span>
        <button
          type="button"
          class="badge toggle-btn"
          class:active={isPanelOpen}
          onclick={() => (isPanelOpen = !isPanelOpen)}
          title="Toggle Side Panel"
        >
          <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
            <line x1="15" y1="3" x2="15" y2="21" />
          </svg>
          Side Panel
        </button>
      </div>
    </div>

    <!-- Bottom Controls Bar -->
    <div class="overlay-bottom" class:hidden={!showControls && isPlaying}>
      <!-- Timeline Scrubber -->
      <div
        class="scrub-container"
        role="slider"
        tabindex="0"
        aria-label="Video Timeline"
        aria-valuenow={currentTime}
        aria-valuemin="0"
        aria-valuemax={duration || 100}
        onclick={(e) => {
          const rect = e.currentTarget.getBoundingClientRect();
          const clickPct = (e.clientX - rect.left) / rect.width;
          seek(clickPct * duration);
        }}
        onmousemove={(e) => {
          const rect = e.currentTarget.getBoundingClientRect();
          hoverPct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
          hoverTime = hoverPct * duration;
        }}
        onmouseleave={() => (hoverTime = null)}
        onkeydown={(e) => {
          if (e.key === "ArrowLeft") seekOffset(-5);
          if (e.key === "ArrowRight") seekOffset(5);
        }}
      >
        <div class="scrub-track">
          <!-- A-B Repeat Highlight -->
          {#if repeatA !== null && duration > 0}
            <div
              class="repeat-region"
              style:left="{(repeatA / duration) * 100}%"
              style:width="{((repeatB ?? currentTime) - repeatA) / duration * 100}%"
            ></div>
          {/if}

          <!-- Play Progress -->
          <div
            class="scrub-fill"
            style:width="{duration > 0 ? (currentTime / duration) * 100 : 0}%"
          ></div>

          <!-- Hover Tooltip -->
          {#if hoverTime !== null}
            <div class="scrub-tooltip" style:left="{hoverPct * 100}%">
              {formatTime(hoverTime)}
            </div>
          {/if}
        </div>
      </div>

      <!-- Action Buttons Row -->
      <div class="controls-row">
        <div class="left-controls">
          <!-- Play / Pause -->
          <button type="button" class="icon-btn primary" onclick={togglePlay} title={isPlaying ? "Pause (Space)" : "Play (Space)"}>
            {#if isPlaying}
              <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor">
                <rect x="6" y="4" width="4" height="16" />
                <rect x="14" y="4" width="4" height="16" />
              </svg>
            {:else}
              <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor">
                <polygon points="5,3 19,12 5,21" />
              </svg>
            {/if}
          </button>

          <!-- Back 10s -->
          <button type="button" class="icon-btn" onclick={() => seekOffset(-10)} title="Back 10s (J / Left)">
            <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
              <path d="M1 4v6h6M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
            </svg>
          </button>

          <!-- Forward 10s -->
          <button type="button" class="icon-btn" onclick={() => seekOffset(10)} title="Forward 10s (L / Right)">
            <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
              <path d="M23 4v6h-6M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
            </svg>
          </button>

          <!-- Volume Controls -->
          <div class="volume-group">
            <button type="button" class="icon-btn" onclick={toggleMute} title="Mute / Unmute (M)">
              {#if isMuted || volume === 0}
                <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
                  <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
                  <line x1="23" y1="9" x2="17" y2="15" />
                  <line x1="17" y1="9" x2="23" y2="15" />
                </svg>
              {:else if volume < 0.5}
                <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
                  <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
                  <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />
                </svg>
              {:else}
                <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
                  <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
                  <path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07" />
                </svg>
              {/if}
            </button>
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={isMuted ? 0 : volume}
              oninput={(e) => setVolume(parseFloat(e.currentTarget.value))}
              class="volume-slider"
            />
          </div>

          <!-- Time Display -->
          <span class="time-readout">
            {formatTime(currentTime)} <span class="time-sep">/</span> {formatTime(duration)}
          </span>
        </div>

        <div class="right-controls">
          <!-- A-B Repeat Button -->
          <button
            type="button"
            class="pill-btn"
            class:active={repeatA !== null}
            onclick={toggleAbRepeat}
            title="A-B Loop Repeat"
          >
            {#if repeatA === null}
              A-B
            {:else if repeatB === null}
              A→?
            {:else}
              A⇄B
            {/if}
          </button>

          <!-- Speed Menu Selector -->
          <div class="speed-selector">
            {#each [0.75, 1.0, 1.25, 1.5, 2.0] as rate}
              <button
                type="button"
                class="speed-btn"
                class:active={playbackRate === rate}
                onclick={() => setSpeed(rate)}
              >
                {rate}x
              </button>
            {/each}
          </div>

          <!-- PiP Button -->
          <button type="button" class="icon-btn" onclick={togglePip} title="Picture in Picture">
            <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
              <rect x="2" y="4" width="20" height="16" rx="2" />
              <rect x="12" y="10" width="8" height="8" rx="1" fill="currentColor" opacity="0.3" />
            </svg>
          </button>

          <!-- Fullscreen Button -->
          <button type="button" class="icon-btn" onclick={toggleFullscreen} title="Fullscreen (F)">
            {#if isFullscreen}
              <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
                <path d="M8 3v3a2 2 0 0 1-2 2H3m18 0h-3a2 2 0 0 1-2-2V3m0 18v-3a2 2 0 0 1 2-2h3M3 16h3a2 2 0 0 1 2 2v3" />
              </svg>
            {:else}
              <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
                <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
              </svg>
            {/if}
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- Right Drawer Panel -->
  {#if isPanelOpen}
    <aside class="side-panel">
      <!-- Tabs Header -->
      <nav class="panel-tabs">
        <button
          type="button"
          class="tab-btn"
          class:active={activeTab === "chapters"}
          onclick={() => (activeTab = "chapters")}
        >
          Chapters
        </button>
        <button
          type="button"
          class="tab-btn"
          class:active={activeTab === "transcript"}
          onclick={() => (activeTab = "transcript")}
        >
          Transcript
        </button>
        <button
          type="button"
          class="tab-btn"
          class:active={activeTab === "notes"}
          onclick={() => (activeTab = "notes")}
        >
          Notes ({notes.length})
        </button>
        <button
          type="button"
          class="tab-btn"
          class:active={activeTab === "ai"}
          onclick={() => (activeTab = "ai")}
        >
          AI Assistant
        </button>
      </nav>

      <!-- Panel Body -->
      <div class="panel-content">
        <!-- 1. Chapters Tab -->
        {#if activeTab === "chapters"}
          <div class="chapters-list">
            {#each chapters as ch, idx}
              <button
                type="button"
                class="chapter-row"
                class:active={currentChapter?.id === ch.id}
                onclick={() => seek(ch.startTime)}
              >
                <span class="ch-idx">{idx + 1}</span>
                <div class="ch-info">
                  <span class="ch-title">{ch.title}</span>
                  <span class="ch-time">{formatTime(ch.startTime)} · {ch.duration}</span>
                </div>
              </button>
            {/each}
          </div>

        <!-- 2. Interactive Transcript Tab -->
        {:else if activeTab === "transcript"}
          <div class="transcript-pane">
            <input
              type="text"
              class="filter-input"
              placeholder="Search transcript..."
              bind:value={transcriptQuery}
            />

            <div class="transcript-list">
              {#each filteredTranscript as line}
                <button
                  type="button"
                  class="transcript-row"
                  class:active={currentTime >= line.start && currentTime < line.start + 25}
                  onclick={() => seek(line.start)}
                >
                  <span class="t-stamp">{formatTime(line.start)}</span>
                  <span class="t-text">{line.text}</span>
                </button>
              {/each}
            </div>
          </div>

        <!-- 3. Timestamped Notes Tab -->
        {:else if activeTab === "notes"}
          <div class="notes-pane">
            <div class="note-creator">
              <span class="note-time-tag">At {formatTime(currentTime)}</span>
              <textarea
                class="note-textarea"
                rows="2"
                placeholder="Write a note for this moment..."
                bind:value={noteInput}
                onkeydown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    addNote();
                  }
                }}
              ></textarea>
              <button type="button" class="btn-submit" onclick={addNote} disabled={!noteInput.trim()}>
                Save Note
              </button>
            </div>

            <div class="notes-list">
              {#if notes.length === 0}
                <div class="empty-state">No notes saved for this video yet.</div>
              {:else}
                {#each notes as note (note.id)}
                  <div class="note-card">
                    <div class="note-head">
                      <button type="button" class="note-seek-btn" onclick={() => seek(note.timestamp)}>
                        {formatTime(note.timestamp)}
                      </button>
                      <span class="note-date">{note.createdAt}</span>
                      <button type="button" class="icon-del" onclick={() => deleteNote(note.id)} title="Delete note">
                        ×
                      </button>
                    </div>
                    <p class="note-body">{note.text}</p>
                  </div>
                {/each}
              {/if}
            </div>
          </div>

        <!-- 4. AI Video Assistant Tab -->
        {:else if activeTab === "ai"}
          <div class="ai-pane">
            <div class="ai-messages">
              {#each aiMessages as msg}
                <div class="ai-bubble" class:user={msg.role === "user"} class:ai={msg.role === "ai"}>
                  <p>{msg.text}</p>
                </div>
              {/each}
              {#if isAiThinking}
                <div class="ai-bubble ai thinking">
                  <span>Analyzing transcript...</span>
                </div>
              {/if}
            </div>

            <form class="ai-input-form" onsubmit={(e) => { e.preventDefault(); sendAiQuestion(); }}>
              <input
                type="text"
                class="ai-input"
                placeholder="Ask about this video..."
                bind:value={aiQuery}
                disabled={isAiThinking}
              />
              <button type="submit" class="ai-send-btn" disabled={!aiQuery.trim() || isAiThinking}>
                Send
              </button>
            </form>
          </div>
        {/if}
      </div>
    </aside>
  {/if}
</div>

<style>
  .player-wrapper {
    display: flex;
    width: 100%;
    height: 100vh;
    background: #030712;
    color: #f3f4f6;
    overflow: hidden;
    position: relative;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  }

  .video-stage {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #000;
    min-width: 0;
    overflow: hidden;
  }

  .ambient-glow {
    position: absolute;
    inset: -15%;
    filter: blur(80px);
    opacity: 0.6;
    pointer-events: none;
    z-index: 1;
    transition: opacity 0.5s ease;
  }

  video {
    width: 100%;
    height: 100%;
    object-fit: contain;
    z-index: 2;
    cursor: pointer;
  }

  .osd-banner {
    position: absolute;
    top: 70px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(17, 24, 39, 0.85);
    backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.15);
    padding: 8px 18px;
    border-radius: 9999px;
    font-size: 14px;
    font-weight: 600;
    z-index: 20;
    pointer-events: none;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
  }

  .overlay-top {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    padding: 16px 20px 32px;
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0.85), transparent);
    display: flex;
    align-items: center;
    gap: 16px;
    z-index: 15;
    transition: opacity 0.3s ease, transform 0.3s ease;
  }

  .overlay-bottom {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 32px 20px 16px;
    background: linear-gradient(to top, rgba(0, 0, 0, 0.9), transparent);
    display: flex;
    flex-direction: column;
    gap: 10px;
    z-index: 15;
    transition: opacity 0.3s ease, transform 0.3s ease;
  }

  .hidden {
    opacity: 0;
    pointer-events: none;
    transform: translateY(6px);
  }

  .video-meta {
    flex: 1;
    min-width: 0;
  }

  .video-heading {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .video-sub {
    font-size: 12px;
    color: #9ca3af;
  }

  .top-badges {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .badge {
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.15);
  }

  .badge.hd {
    background: rgba(14, 165, 233, 0.15);
    color: #38bdf8;
    border-color: rgba(14, 165, 233, 0.3);
  }

  .badge.toggle-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    background: rgba(255, 255, 255, 0.12);
  }

  .badge.toggle-btn.active {
    background: #0284c7;
    color: #fff;
    border-color: #0284c7;
  }

  /* Scrubber */
  .scrub-container {
    width: 100%;
    padding: 8px 0;
    cursor: pointer;
    position: relative;
  }

  .scrub-track {
    width: 100%;
    height: 5px;
    background: rgba(255, 255, 255, 0.2);
    border-radius: 9999px;
    position: relative;
    transition: height 0.15s ease;
  }

  .scrub-container:hover .scrub-track {
    height: 8px;
  }

  .scrub-fill {
    height: 100%;
    background: #0284c7;
    border-radius: 9999px;
    position: relative;
  }

  .repeat-region {
    position: absolute;
    top: 0;
    bottom: 0;
    background: rgba(168, 85, 247, 0.5);
    border-left: 2px solid #c084fc;
    border-right: 2px solid #c084fc;
  }

  .scrub-tooltip {
    position: absolute;
    bottom: 14px;
    transform: translateX(-50%);
    background: #111827;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    pointer-events: none;
    box-shadow: 0 4px 10px rgba(0, 0, 0, 0.5);
  }

  /* Controls Row */
  .controls-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .left-controls, .right-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon-btn {
    all: unset;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    color: #e5e7eb;
    cursor: pointer;
    transition: background 0.15s ease, transform 0.1s ease;
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
  }

  .icon-btn.primary {
    width: 42px;
    height: 42px;
    background: #0284c7;
    color: #fff;
  }

  .icon-btn.primary:hover {
    background: #0369a1;
  }

  .volume-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .volume-slider {
    width: 70px;
    accent-color: #0284c7;
    cursor: pointer;
  }

  .time-readout {
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    color: #d1d5db;
  }

  .time-sep {
    color: #6b7280;
  }

  .pill-btn {
    all: unset;
    padding: 4px 10px;
    border-radius: 9999px;
    font-size: 12px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.12);
    cursor: pointer;
  }

  .pill-btn.active {
    background: #a855f7;
    color: #fff;
  }

  .speed-selector {
    display: flex;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 2px;
  }

  .speed-btn {
    all: unset;
    padding: 3px 7px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    color: #9ca3af;
  }

  .speed-btn.active {
    background: rgba(255, 255, 255, 0.2);
    color: #fff;
    font-weight: 600;
  }

  /* Side Panel */
  .side-panel {
    width: 360px;
    border-left: 1px solid rgba(255, 255, 255, 0.1);
    background: #0b0f19;
    display: flex;
    flex-direction: column;
    z-index: 10;
  }

  .panel-tabs {
    display: flex;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: #111827;
  }

  .tab-btn {
    all: unset;
    flex: 1;
    text-align: center;
    padding: 12px 0;
    font-size: 12px;
    font-weight: 600;
    color: #9ca3af;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }

  .tab-btn.active {
    color: #38bdf8;
    border-bottom-color: #38bdf8;
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 14px;
  }

  /* Chapters */
  .chapters-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .chapter-row {
    all: unset;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 8px;
    cursor: pointer;
    background: rgba(255, 255, 255, 0.03);
    transition: background 0.15s ease;
  }

  .chapter-row:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .chapter-row.active {
    background: rgba(14, 165, 233, 0.15);
    border: 1px solid rgba(14, 165, 233, 0.3);
  }

  .ch-idx {
    font-size: 13px;
    font-weight: 700;
    color: #6b7280;
    width: 18px;
  }

  .chapter-row.active .ch-idx {
    color: #38bdf8;
  }

  .ch-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ch-title {
    font-size: 13px;
    font-weight: 500;
  }

  .ch-time {
    font-size: 11px;
    color: #9ca3af;
  }

  /* Transcript */
  .transcript-pane {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
  }

  .filter-input {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #fff;
    font-size: 12px;
    outline: none;
  }

  .filter-input:focus {
    border-color: #0284c7;
  }

  .transcript-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .transcript-row {
    all: unset;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    line-height: 1.4;
  }

  .transcript-row:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .transcript-row.active {
    background: rgba(14, 165, 233, 0.12);
  }

  .t-stamp {
    color: #38bdf8;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    font-size: 11px;
  }

  .t-text {
    flex: 1;
    color: #d1d5db;
  }

  /* Notes */
  .notes-pane {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .note-creator {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(255, 255, 255, 0.04);
    padding: 10px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .note-time-tag {
    font-size: 11px;
    font-weight: 600;
    color: #a855f7;
  }

  .note-textarea {
    width: 100%;
    box-sizing: border-box;
    background: #030712;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    color: #fff;
    padding: 8px;
    font-size: 12px;
    resize: none;
    outline: none;
  }

  .btn-submit {
    align-self: flex-end;
    padding: 6px 12px;
    border-radius: 6px;
    border: none;
    background: #0284c7;
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-submit:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .note-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 8px;
    padding: 10px;
    margin-bottom: 8px;
  }

  .note-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .note-seek-btn {
    all: unset;
    font-size: 11px;
    font-weight: 700;
    color: #38bdf8;
    background: rgba(14, 165, 233, 0.15);
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }

  .note-date {
    font-size: 10px;
    color: #6b7280;
  }

  .icon-del {
    all: unset;
    color: #9ca3af;
    cursor: pointer;
    font-size: 14px;
  }

  .icon-del:hover {
    color: #ef4444;
  }

  .note-body {
    margin: 0;
    font-size: 12px;
    color: #e5e7eb;
    line-height: 1.4;
  }

  /* AI Assistant */
  .ai-pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 10px;
  }

  .ai-messages {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ai-bubble {
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 12px;
    line-height: 1.4;
    max-width: 90%;
  }

  .ai-bubble.ai {
    background: rgba(255, 255, 255, 0.06);
    color: #e5e7eb;
    align-self: flex-start;
  }

  .ai-bubble.user {
    background: #0284c7;
    color: #fff;
    align-self: flex-end;
  }

  .ai-bubble.thinking {
    font-style: italic;
    color: #9ca3af;
  }

  .ai-input-form {
    display: flex;
    gap: 6px;
  }

  .ai-input {
    flex: 1;
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #fff;
    font-size: 12px;
    outline: none;
  }

  .ai-send-btn {
    padding: 0 14px;
    border-radius: 6px;
    border: none;
    background: #0284c7;
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .ai-send-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .empty-state {
    text-align: center;
    color: #6b7280;
    font-size: 12px;
    padding: 24px 0;
  }
</style>
