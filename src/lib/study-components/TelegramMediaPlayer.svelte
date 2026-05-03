<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { t } from "$lib/i18n";
  import {
    telegramStreamUrl,
    detectEmbedPlatform,
    buildEmbedUrl,
    studyPlaybackGet,
    studyPlaybackSave,
    type TelegramChat,
    type TelegramMediaItem,
  } from "$lib/study-telegram-bridge";
  import PlayerShell from "$lib/study-components/player/PlayerShell.svelte";

  let {
    chat,
    playlist,
    initialIdx,
    onClose,
    onDownload,
    originRect,
  } = $props<{
    chat: TelegramChat;
    playlist: TelegramMediaItem[];
    initialIdx: number;
    onClose: () => void;
    onDownload?: (item: TelegramMediaItem) => void;
    originRect?: DOMRect;
  }>();

  let flipRoot: HTMLElement | null = $state(null);
  let flipPlayed = $state(false);

  let idx = $state(initialIdx);
  let streamUrl = $state("");
  let loading = $state(false);
  let error = $state("");
  let videoEl = $state<HTMLVideoElement | null>(null);
  let audioEl = $state<HTMLAudioElement | null>(null);
  let resumePosition = $state<number | null>(null);
  let lastSaveAt = 0;

  const SPEED_KEY_PREFIX = "study.library.tg.speed.";
  let playbackRate = $state(1);

  const item = $derived(playlist[idx] ?? null);

  function speedKey() {
    return `${SPEED_KEY_PREFIX}${chat.chat_type}:${chat.id}`;
  }

  function loadSpeed() {
    if (typeof localStorage === "undefined") return;
    const raw = localStorage.getItem(speedKey());
    if (!raw) return;
    const n = parseFloat(raw);
    if (!Number.isNaN(n) && n >= 0.25 && n <= 4) {
      playbackRate = n;
    }
  }

  function setSpeed(n: number) {
    playbackRate = n;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(speedKey(), String(n));
    }
    if (videoEl) videoEl.playbackRate = n;
    if (audioEl) audioEl.playbackRate = n;
  }

  function categorize(it: TelegramMediaItem): "video" | "audio" | "image" | "webpage" | "other" {
    if (it.media_type === "webpage" && it.webpage) return "webpage";
    const mt = it.media_type.toLowerCase();
    const ext = it.file_name.split(".").pop()?.toLowerCase() ?? "";
    if (mt === "video" || ["mp4", "webm", "mkv", "mov", "m4v", "avi"].includes(ext))
      return "video";
    if (mt === "audio" || ["mp3", "ogg", "opus", "wav", "flac", "m4a", "aac"].includes(ext))
      return "audio";
    if (mt === "photo" || ["jpg", "jpeg", "png", "webp", "gif", "bmp"].includes(ext))
      return "image";
    return "other";
  }

  const kind = $derived(item ? categorize(item) : "other");

  const webpageEmbedUrl = $derived.by(() => {
    if (!item?.webpage) return null;
    const wp = item.webpage;
    const platform = detectEmbedPlatform(wp.url, wp.site_name);
    if (!platform) return null;
    return wp.embed_url || buildEmbedUrl(platform, wp.url);
  });

  async function rebuildUrl() {
    if (!item) {
      streamUrl = "";
      resumePosition = null;
      return;
    }
    if (kind === "other" || kind === "webpage") {
      streamUrl = "";
      resumePosition = null;
      return;
    }
    loading = true;
    error = "";
    resumePosition = null;
    try {
      const [url, saved] = await Promise.all([
        telegramStreamUrl({
          chatId: chat.id,
          chatType: chat.chat_type,
          messageId: item.message_id,
        }),
        studyPlaybackGet({ chatId: chat.id, messageId: item.message_id }).catch(() => null),
      ]);
      streamUrl = url;
      if (saved && !saved.finished && saved.position_seconds > 5) {
        resumePosition = saved.position_seconds;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      streamUrl = "";
    } finally {
      loading = false;
    }
  }

  function onMediaLoaded() {
    if (resumePosition && resumePosition > 0) {
      if (videoEl) videoEl.currentTime = resumePosition;
      if (audioEl) audioEl.currentTime = resumePosition;
    }
    if (videoEl) videoEl.playbackRate = playbackRate;
    if (audioEl) audioEl.playbackRate = playbackRate;
  }

  async function persistPosition(finished: boolean) {
    if (!item) return;
    const el = videoEl ?? audioEl;
    if (!el) return;
    const pos = el.currentTime;
    const dur = el.duration && Number.isFinite(el.duration) ? el.duration : 0;
    try {
      await studyPlaybackSave({
        chatId: chat.id,
        messageId: item.message_id,
        positionSeconds: pos,
        durationSeconds: dur,
        finished,
        title: item.file_name,
      });
    } catch {
      /* best-effort */
    }
  }

  function onTimeUpdate() {
    const now = Date.now();
    if (now - lastSaveAt < 5000) return; // throttle 5s
    lastSaveAt = now;
    void persistPosition(false);
  }

  function onEnded() {
    void persistPosition(true);
  }

  function onRateChange() {
    const el = videoEl ?? audioEl;
    if (!el) return;
    if (el.playbackRate !== playbackRate) {
      setSpeed(el.playbackRate);
    }
  }

  $effect(() => {
    void rebuildUrl();
  });

  function next() {
    if (idx < playlist.length - 1) idx += 1;
  }

  function prev() {
    if (idx > 0) idx -= 1;
  }

  function onKeyDown(ev: KeyboardEvent) {
    const target = ev.target as HTMLElement | null;
    const tag = target?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || target?.isContentEditable) return;
    if (ev.key === "Escape") {
      ev.preventDefault();
      onClose();
    } else if (ev.key === "j" || ev.key === "ArrowDown") {
      ev.preventDefault();
      next();
    } else if (ev.key === "k" || ev.key === "ArrowUp") {
      ev.preventDefault();
      prev();
    }
  }

  onMount(() => {
    loadSpeed();
    if (typeof window === "undefined") return;
    window.addEventListener("keydown", onKeyDown);
    if (originRect && flipRoot && !flipPlayed) {
      const dst = flipRoot.getBoundingClientRect();
      if (dst.width > 0 && dst.height > 0 && originRect.width > 0 && originRect.height > 0) {
        const dx = originRect.left + originRect.width / 2 - (dst.left + dst.width / 2);
        const dy = originRect.top + originRect.height / 2 - (dst.top + dst.height / 2);
        const sx = originRect.width / dst.width;
        const sy = originRect.height / dst.height;
        const el = flipRoot;
        el.style.transformOrigin = "center center";
        el.style.transform = `translate(${dx}px, ${dy}px) scale(${sx}, ${sy})`;
        el.style.opacity = "0.65";
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            el.style.transition =
              "transform 280ms cubic-bezier(0.22, 1, 0.36, 1), opacity 220ms ease";
            el.style.transform = "none";
            el.style.opacity = "1";
            const cleanup = () => {
              el.style.transition = "";
              el.style.transform = "";
              el.style.opacity = "";
              flipPlayed = true;
              el.removeEventListener("transitionend", cleanup);
            };
            el.addEventListener("transitionend", cleanup, { once: true });
          });
        });
      }
    }
  });

  onDestroy(() => {
    void persistPosition(false);
    if (typeof window === "undefined") return;
    window.removeEventListener("keydown", onKeyDown);
  });

  function fmtBytes(n: number) {
    if (!n) return "";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let v = n;
    let i = 0;
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024;
      i++;
    }
    return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
  }

  let chromeVisible = $state(true);
  let chromeTimer: ReturnType<typeof setTimeout> | null = null;

  function showChrome() {
    chromeVisible = true;
    if (chromeTimer) clearTimeout(chromeTimer);
    chromeTimer = setTimeout(() => {
      chromeVisible = false;
    }, 3000);
  }

  function onStageMouseMove() {
    showChrome();
  }

  $effect(() => {
    if (!item) return;
    showChrome();
    return () => {
      if (chromeTimer) {
        clearTimeout(chromeTimer);
        chromeTimer = null;
      }
    };
  });

  let dragY = $state(0);
  let isDragging = $state(false);
  let dragStartY = 0;
  let captionExpanded = $state(false);

  function onStagePointerDown(ev: PointerEvent) {
    if (kind !== "image") return;
    if (ev.button !== 0) return;
    const t = ev.target as HTMLElement;
    if (t.closest("button, a, select, video, audio, iframe")) return;
    isDragging = true;
    dragStartY = ev.clientY;
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
  }

  function onStagePointerMove(ev: PointerEvent) {
    if (!isDragging) return;
    const dy = ev.clientY - dragStartY;
    dragY = Math.max(0, dy);
  }

  function onStagePointerUp() {
    if (!isDragging) return;
    isDragging = false;
    if (dragY > 100) {
      onClose();
    } else {
      dragY = 0;
    }
  }

  function onStageClick(ev: MouseEvent) {
    if (kind !== "image") return;
    const t = ev.target as HTMLElement;
    if (t.closest("button, a, select")) return;
    const rect = (ev.currentTarget as HTMLElement).getBoundingClientRect();
    const x = ev.clientX - rect.left;
    if (x < rect.width * 0.3) {
      prev();
    } else if (x > rect.width * 0.7) {
      next();
    }
  }

  let zoomScale = $state(1);
  let zoomOriginX = $state(50);
  let zoomOriginY = $state(50);
  let lastClickTime = 0;

  function onImageDoubleClick(ev: MouseEvent) {
    if (kind !== "image") return;
    ev.preventDefault();
    ev.stopPropagation();
    if (zoomScale === 1) {
      const target = ev.currentTarget as HTMLElement;
      const rect = target.getBoundingClientRect();
      zoomOriginX = ((ev.clientX - rect.left) / rect.width) * 100;
      zoomOriginY = ((ev.clientY - rect.top) / rect.height) * 100;
      zoomScale = 2;
    } else {
      zoomScale = 1;
    }
  }

  function onImageClickCapture(ev: MouseEvent) {
    const now = Date.now();
    if (now - lastClickTime < 300) {
      onImageDoubleClick(ev);
      lastClickTime = 0;
    } else {
      lastClickTime = now;
    }
  }

  $effect(() => {
    if (item) {
      zoomScale = 1;
    }
  });

  let pipSupported = $derived(
    typeof document !== "undefined" &&
      "pictureInPictureEnabled" in document &&
      (document as Document & { pictureInPictureEnabled?: boolean })
        .pictureInPictureEnabled === true,
  );

  async function togglePip() {
    if (!videoEl) return;
    try {
      const doc = document as Document & {
        pictureInPictureElement?: Element | null;
        exitPictureInPicture?: () => Promise<void>;
      };
      if (doc.pictureInPictureElement) {
        await doc.exitPictureInPicture?.();
      } else {
        const v = videoEl as HTMLVideoElement & {
          requestPictureInPicture?: () => Promise<PictureInPictureWindow>;
        };
        await v.requestPictureInPicture?.();
      }
    } catch {
      /* user denied or unsupported */
    }
  }
</script>

{#if item}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="player-overlay"
    bind:this={flipRoot}
    class:dragging={isDragging}
    style:--drag-y="{dragY}px"
    style:--drag-fade={Math.max(0.4, 1 - dragY / 280)}
    role="dialog"
    aria-modal="true"
    aria-label={item.file_name}
    tabindex="-1"
    onmousemove={onStageMouseMove}
  >
    <header class="head" class:chrome-hidden={!chromeVisible}>
      <button
        type="button"
        class="back-btn"
        onclick={onClose}
        aria-label={$t("study.common.close")}
      >
        ← {$t("study.common.close")}
      </button>
      <div class="title-block">
        <span class="title" title={item.file_name}>{item.file_name}</span>
        <span class="muted small">
          {fmtBytes(item.file_size)} · {idx + 1} / {playlist.length}
        </span>
      </div>
      <div class="head-actions">
        {#if kind === "video" || kind === "audio"}
          <select
            class="speed-select"
            value={String(playbackRate)}
            onchange={(e) => {
              const v = parseFloat((e.target as HTMLSelectElement).value);
              if (!Number.isNaN(v)) setSpeed(v);
            }}
            title={$t("study.library.telegram.player_speed")}
          >
            {#each [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2] as s (s)}
              <option value={String(s)}>{s === 1 ? "1×" : `${s}×`}</option>
            {/each}
          </select>
        {/if}
        {#if kind === "video" && pipSupported}
          <button
            type="button"
            class="ghost-btn"
            onclick={togglePip}
            title={$t("study.library.telegram.player_pip")}
            aria-label={$t("study.library.telegram.player_pip")}
          >
            ⧉
          </button>
        {/if}
        <button
          type="button"
          class="ghost-btn"
          onclick={prev}
          disabled={idx <= 0}
          title="k / ↑"
        >
          ↑
        </button>
        <button
          type="button"
          class="ghost-btn"
          onclick={next}
          disabled={idx >= playlist.length - 1}
          title="j / ↓"
        >
          ↓
        </button>
        {#if onDownload}
          <button
            type="button"
            class="ghost-btn"
            onclick={() => onDownload?.(item)}
          >
            ↓ {$t("study.library.telegram.downloads.download")}
          </button>
        {/if}
      </div>
    </header>

    <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
    <div
      class="stage"
      onpointerdown={onStagePointerDown}
      onpointermove={onStagePointerMove}
      onpointerup={onStagePointerUp}
      onpointercancel={onStagePointerUp}
      onclick={onStageClick}
    >
      {#if loading}
        <p class="muted">{$t("study.common.loading")}</p>
      {:else if error}
        <p class="error small">{error}</p>
      {:else if kind === "video" && streamUrl}
        {#key streamUrl}
          <div class="media-player-wrap">
            <PlayerShell
              videoSrc={streamUrl}
              title={item.file_name}
              courseTitle={chat.title}
              backHref=""
              durationMs={null}
              initialSeconds={resumePosition ?? 0}
              initialPlaybackSpeed={playbackRate}
              subtitles={[]}
              audioTracks={[]}
              skipGaps={null}
              thumbnails={[]}
              settings={null}
              selectedSubtitleLang={null}
              selectedAudioLang={null}
              theaterMode={false}
              onTimeUpdate={() => onTimeUpdate()}
              onSeek={() => {}}
              onPlay={() => {}}
              onPause={() => {}}
              onEnded={onEnded}
              onLoadedMetadata={onMediaLoaded}
              onSubtitleChange={() => {}}
              onAudioChange={() => {}}
              onSpeedChange={(s) => setSpeed(s)}
              onSkipGap={() => {}}
              onTheaterToggle={() => {}}
              onClose={onClose}
              onVideoEl={(el) => (videoEl = el)}
            />
          </div>
        {/key}
      {:else if kind === "audio" && streamUrl}
        {#key streamUrl}
          <audio
            bind:this={audioEl}
            class="media-audio"
            src={streamUrl}
            controls
            autoplay
            onloadedmetadata={onMediaLoaded}
            ontimeupdate={onTimeUpdate}
            onended={onEnded}
            onratechange={onRateChange}
          ></audio>
        {/key}
      {:else if kind === "image" && streamUrl}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
        <img
          class="media-img"
          class:zoomed={zoomScale > 1}
          src={streamUrl}
          alt={item.file_name}
          style:transform={zoomScale > 1 ? `scale(${zoomScale})` : "scale(1)"}
          style:transform-origin="{zoomOriginX}% {zoomOriginY}%"
          onclick={onImageClickCapture}
        />
      {:else if kind === "webpage" && item.webpage}
        {#if webpageEmbedUrl}
          <iframe
            class="media"
            src={webpageEmbedUrl}
            title={item.webpage.title ?? item.webpage.url}
            allowfullscreen
            allow="autoplay; encrypted-media; picture-in-picture"
            referrerpolicy="strict-origin-when-cross-origin"
          ></iframe>
        {:else}
          <div class="webpage-card">
            <h3>{item.webpage.title ?? item.webpage.url}</h3>
            {#if item.webpage.site_name}
              <p class="muted small">{item.webpage.site_name}</p>
            {/if}
            {#if item.webpage.description}
              <p>{item.webpage.description}</p>
            {/if}
            <a class="ghost-btn" href={item.webpage.url} target="_blank" rel="noreferrer noopener">
              {$t("study.library.telegram.webpage_open_external")}
            </a>
          </div>
        {/if}
      {:else}
        <div class="not-playable">
          {#if item.file_name.toLowerCase().endsWith(".pdf") || item.file_name.toLowerCase().endsWith(".epub")}
            <p class="muted">{$t("study.library.telegram.player_book_hint")}</p>
          {:else}
            <p class="muted">{$t("study.library.telegram.player_not_playable")}</p>
          {/if}
          {#if onDownload}
            <button type="button" class="ghost-btn" onclick={() => onDownload?.(item)}>
              ↓ {$t("study.library.telegram.downloads.download")}
            </button>
          {/if}
        </div>
      {/if}
    </div>

    {#if item.caption}
      <div
        class="caption-strip"
        class:chrome-hidden={!chromeVisible}
        class:expanded={captionExpanded}
      >
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_click_events_have_key_events -->
        <p
          class="caption-text"
          onclick={(e) => {
            e.stopPropagation();
            captionExpanded = !captionExpanded;
          }}
        >
          {item.caption}
        </p>
      </div>
    {/if}

    <footer class="foot muted small" class:chrome-hidden={!chromeVisible}>
      {$t("study.library.telegram.player_kbd_hint")}
    </footer>
  </div>
{/if}

<style>
  .player-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, calc(0.92 * var(--drag-fade, 1)));
    display: flex;
    flex-direction: column;
    z-index: 100;
    color: var(--secondary);
    animation: fade-in 200ms cubic-bezier(0.22, 1, 0.36, 1);
    transform: translateY(var(--drag-y, 0));
    transition: transform var(--tg-duration-normal, 200ms) var(--tg-easing-default, cubic-bezier(0.22, 1, 0.36, 1)),
      background var(--tg-duration-normal, 200ms) var(--tg-easing-default, ease);
  }
  .player-overlay.dragging {
    transition: none;
  }
  @keyframes fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }
  .head,
  .foot,
  .caption-strip {
    transition: opacity var(--tg-duration-normal, 200ms) var(--tg-easing-default, ease),
      transform var(--tg-duration-normal, 200ms) var(--tg-easing-default, ease);
  }
  .head.chrome-hidden {
    opacity: 0;
    transform: translateY(-8px);
    pointer-events: none;
  }
  .foot.chrome-hidden {
    opacity: 0;
    transform: translateY(8px);
    pointer-events: none;
  }
  .caption-strip {
    position: absolute;
    top: 64px;
    left: 0;
    right: 0;
    padding: 10px 24px;
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0.65), transparent);
    color: white;
    font-size: 13px;
    line-height: 1.4;
    pointer-events: none;
    z-index: 5;
  }
  .caption-strip.chrome-hidden {
    opacity: 0;
    transform: translateY(-8px);
  }
  .caption-text {
    margin: 0;
    max-width: 800px;
    margin-inline: auto;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    pointer-events: auto;
    cursor: pointer;
    user-select: text;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(6px);
    border-radius: 6px;
    padding: 6px 10px;
  }
  .caption-strip.expanded .caption-text {
    -webkit-line-clamp: unset;
    line-clamp: unset;
    max-height: 40vh;
    overflow-y: auto;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    background: rgba(0, 0, 0, 0.5);
    border-bottom: 1px solid color-mix(in oklab, white 12%, transparent);
  }
  .back-btn {
    background: transparent;
    border: 1px solid color-mix(in oklab, white 22%, transparent);
    color: white;
    border-radius: var(--border-radius);
    padding: 6px 12px;
    font-family: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .back-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .title-block {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title {
    color: white;
    font-size: 14px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .muted {
    color: rgba(255, 255, 255, 0.5);
  }
  .small {
    font-size: 11px;
  }
  .head-actions {
    display: flex;
    gap: 6px;
  }
  .ghost-btn {
    background: transparent;
    border: 1px solid color-mix(in oklab, white 22%, transparent);
    color: white;
    border-radius: var(--border-radius);
    padding: 6px 12px;
    font-family: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .speed-select {
    background: transparent;
    border: 1px solid color-mix(in oklab, white 22%, transparent);
    color: white;
    border-radius: var(--border-radius);
    padding: 6px 8px;
    font-family: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .speed-select option {
    color: black;
  }
  .ghost-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .ghost-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .stage {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    overflow: hidden;
  }
  .media {
    max-width: 100%;
    max-height: 100%;
    background: black;
  }
  .media-player-wrap {
    width: min(1200px, 90vw);
    max-width: 100%;
    max-height: 100%;
  }
  .media-audio {
    width: min(560px, 90%);
  }
  .media-img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    cursor: zoom-in;
    transition: transform var(--tg-duration-normal, 200ms) var(--tg-easing-default, cubic-bezier(0.22, 1, 0.36, 1));
  }
  .media-img.zoomed {
    cursor: zoom-out;
  }
  @media (prefers-reduced-motion: reduce) {
    .media-img { transition: none; }
  }
  .not-playable {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: center;
  }
  .webpage-card {
    max-width: 600px;
    padding: 32px;
    background: var(--surface);
    border: 1px solid var(--content-border);
    border-radius: var(--border-radius);
    color: var(--secondary);
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }
  .webpage-card h3 {
    margin: 0;
    color: var(--secondary);
  }
  iframe.media {
    width: 100%;
    height: 100%;
    border: 0;
    background: black;
  }
  .error {
    color: var(--error);
  }
  .foot {
    text-align: center;
    padding: 8px;
    border-top: 1px solid color-mix(in oklab, white 12%, transparent);
  }
  @media (prefers-reduced-motion: reduce) {
    .player-overlay { animation: none; }
  }
</style>
