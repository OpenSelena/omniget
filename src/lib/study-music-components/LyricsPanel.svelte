<script lang="ts">
  import { onMount, tick } from "svelte";
  import { musicPlayer } from "$lib/study-music/player-store.svelte";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { lyricsStore } from "$lib/study-music/lyrics-store.svelte";
  import { t } from "$lib/i18n";
  import PasteLyricsModal from "./PasteLyricsModal.svelte";

  const USER_SCROLL_LOCKOUT_MS = 750;
  const AUTO_SCROLL_THROTTLE_MS = 33;
  const VIRTUALIZE_THRESHOLD = 300;

  let containerEl = $state<HTMLDivElement | null>(null);
  let lineRefs: HTMLLIElement[] = $state([]);
  let pasteOpen = $state(false);
  let reduceMotion = $state(false);
  let editingIdx = $state<number | null>(null);
  let editDraft = $state("");

  let _lastUserScrollAt = 0;
  let _lastAutoScrollAt = 0;
  let _isAutoScrolling = false;
  let _autoScrollTimeout: ReturnType<typeof setTimeout> | null = null;
  let _resizeObserver: ResizeObserver | null = null;
  const _lineHeights = new Map<number, number>();

  const trackId = $derived(musicPlayer.currentTrack?.id ?? null);
  const status = $derived(lyricsStore.status);
  const activeIdx = $derived(
    status === "synced" && lyricsStore.lines.length > 0
      ? lyricsStore.activeIndex(musicPlayer.currentTime)
      : -1,
  );
  const shouldVirtualize = $derived(lyricsStore.lines.length > VIRTUALIZE_THRESHOLD);
  const showTranslation = $derived(lyricsStore.showTranslation);
  const translationStatus = $derived(lyricsStore.translationStatus);
  const translations = $derived(lyricsStore.translations);

  $effect(() => {
    if (!musicUI.lyricsOpen) return;
    if (trackId === null) {
      lyricsStore.reset();
      return;
    }
    void lyricsStore.loadFor(trackId);
  });

  $effect(() => {
    if (!musicUI.lyricsOpen) return;
    if (!showTranslation) return;
    if (status !== "synced" || lyricsStore.lines.length === 0) return;
    void lyricsStore.loadTranslation("pt");
  });

  $effect(() => {
    if (!musicUI.lyricsOpen) return;
    void activeIdx;
    if (activeIdx < 0) return;
    if (!containerEl) return;
    const now = performance.now();
    if (now - _lastUserScrollAt < USER_SCROLL_LOCKOUT_MS) return;
    if (now - _lastAutoScrollAt < AUTO_SCROLL_THROTTLE_MS) return;
    _lastAutoScrollAt = now;
    void scrollToActive();
  });

  async function scrollToActive() {
    await tick();
    const el = lineRefs[activeIdx];
    if (!el || !containerEl) return;
    _isAutoScrolling = true;
    const cRect = containerEl.getBoundingClientRect();
    const offset =
      el.offsetTop - containerEl.offsetTop - cRect.height / 2 + el.offsetHeight / 2;
    containerEl.scrollTo({
      top: offset,
      behavior: reduceMotion ? "instant" : "smooth",
    });
    if (_autoScrollTimeout) clearTimeout(_autoScrollTimeout);
    _autoScrollTimeout = setTimeout(() => {
      _isAutoScrolling = false;
    }, 400);
  }

  function onUserInput() {
    if (_isAutoScrolling) return;
    _lastUserScrollAt = performance.now();
  }

  function close() {
    musicUI.closeLyrics();
  }

  function refresh() {
    if (trackId !== null) {
      void lyricsStore.loadFor(trackId, true);
    }
  }

  function openPaste() {
    pasteOpen = true;
  }

  function closePaste() {
    pasteOpen = false;
  }

  function toggleTranslationClick() {
    lyricsStore.toggleTranslation("pt");
  }

  function startEdit(idx: number) {
    if (!showTranslation) return;
    editingIdx = idx;
    editDraft = translations.get(idx)?.translation ?? "";
  }

  function cancelEdit() {
    editingIdx = null;
    editDraft = "";
  }

  async function saveEdit() {
    if (editingIdx === null) return;
    const idx = editingIdx;
    const text = editDraft.trim();
    if (text.length === 0) {
      cancelEdit();
      return;
    }
    await lyricsStore.setTranslationLine(idx, text);
    cancelEdit();
  }

  function onEditKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void saveEdit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancelEdit();
    }
  }

  async function handleSavedLocal() {
    pasteOpen = false;
  }

  onMount(() => {
    if (typeof document === "undefined") return;
    if (typeof window !== "undefined" && "matchMedia" in window) {
      const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
      reduceMotion = mq.matches;
      const listener = (e: MediaQueryListEvent) => {
        reduceMotion = e.matches;
      };
      mq.addEventListener("change", listener);
      return () => mq.removeEventListener("change", listener);
    }
  });

  onMount(() => {
    if (typeof document === "undefined") return;
    const handler = (e: KeyboardEvent) => {
      if (!musicUI.lyricsOpen) return;
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      )
        return;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        if (pasteOpen) {
          closePaste();
        } else {
          close();
        }
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  });

  $effect(() => {
    if (typeof ResizeObserver === "undefined") return;
    if (!containerEl) return;
    void lyricsStore.lines.length;
    if (_resizeObserver) {
      _resizeObserver.disconnect();
    }
    _resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const el = entry.target as HTMLElement;
        const idx = Number(el.dataset.idx);
        if (Number.isFinite(idx)) {
          _lineHeights.set(idx, entry.contentRect.height);
        }
      }
    });
    for (const el of lineRefs) {
      if (el) _resizeObserver.observe(el);
    }
    return () => {
      if (_resizeObserver) {
        _resizeObserver.disconnect();
        _resizeObserver = null;
      }
    };
  });
</script>

{#if musicUI.lyricsOpen}
  <aside class="lyrics-panel" role="complementary">
    <header class="head">
      <h3>{$t("study.music.lyrics_title")}</h3>
      <div class="actions">
        <button
          type="button"
          class="toggle"
          class:on={showTranslation}
          onclick={toggleTranslationClick}
          aria-pressed={showTranslation}
          title={(showTranslation
            ? $t("study.music.lyrics_translation_hide")
            : $t("study.music.lyrics_translation_show")) as string}
        >
          <span class="dot"></span>
          <span class="lbl">PT</span>
        </button>
        <button
          type="button"
          class="ico"
          onclick={refresh}
          title={$t("study.music.lyrics_refresh") as string}
          aria-label={$t("study.music.lyrics_refresh") as string}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="23 4 23 10 17 10"/>
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
          </svg>
        </button>
        <button
          type="button"
          class="ico"
          onclick={close}
          aria-label={$t("study.common.close") as string}
        >×</button>
      </div>
    </header>

    {#if showTranslation && status === "synced"}
      {#if translationStatus === "loading"}
        <div class="translation-banner banner-info">
          <span>{$t("study.music.lyrics_translation_loading")}</span>
        </div>
      {:else if translationStatus === "skipped"}
        <div class="translation-banner banner-muted">
          <span>{$t("study.music.lyrics_translation_skipped_pt")}</span>
        </div>
      {:else if translationStatus === "no_translator"}
        <button
          type="button"
          class="translation-banner banner-warn banner-clickable"
          onclick={() => musicUI.openTranslationSettings()}
        >
          <strong>{$t("study.music.lyrics_translation_no_translator")}</strong>
          <span class="muted-inline">{$t("study.music.lyrics_translation_no_translator_hint")}</span>
        </button>
      {:else if translationStatus === "error"}
        <div class="translation-banner banner-error">
          <span>{$t("study.music.lyrics_translation_error")}</span>
        </div>
      {/if}
    {/if}

    {#if !musicPlayer.currentTrack}
      <div class="empty">
        <p>{$t("study.music.lyrics_no_track")}</p>
      </div>
    {:else if status === "loading"}
      <div class="loading-skeleton" aria-busy="true" aria-live="polite">
        <span class="skel skel-a"></span>
        <span class="skel skel-b"></span>
        <span class="skel skel-c"></span>
        <p class="muted">{$t("study.music.lyrics_loading")}</p>
      </div>
    {:else if status === "notfound"}
      <div class="empty">
        <p class="empty-title">{$t("study.music.lyrics_not_found")}</p>
        <p class="muted">{$t("study.music.lyrics_lrclib_hint")}</p>
        <button type="button" class="cta" onclick={openPaste}>
          {$t("study.music.lyrics_add_manual")}
        </button>
      </div>
    {:else if status === "error"}
      <div class="empty">
        <p class="empty-title">{$t("study.music.lyrics_error")}</p>
        <p class="muted">{$t("study.music.lyrics_error_hint")}</p>
        <div class="error-actions">
          <button type="button" class="cta" onclick={refresh}>
            {$t("study.music.lyrics_retry")}
          </button>
          <button type="button" class="cta cta-ghost" onclick={openPaste}>
            {$t("study.music.lyrics_add_manual")}
          </button>
        </div>
      </div>
    {:else if status === "synced" && lyricsStore.lines.length > 0}
      <div
        bind:this={containerEl}
        class="scroll synced"
        class:virtualized={shouldVirtualize}
        onwheel={onUserInput}
        ontouchstart={onUserInput}
        onkeydown={onUserInput}
        role="presentation"
      >
        <ul class="lines">
          {#each lyricsStore.lines as line, i (i)}
            <li
              bind:this={lineRefs[i]}
              data-idx={i}
              class="line"
              class:active={i === activeIdx}
              class:past={i < activeIdx}
            >
              <span class="line-original">{line.text || "♪"}</span>
              {#if showTranslation && translationStatus === "available"}
                {#if editingIdx === i}
                  <div class="line-edit">
                    <input
                      type="text"
                      class="line-edit-input"
                      bind:value={editDraft}
                      placeholder={$t("study.music.lyrics_translation_edit_placeholder") as string}
                      onkeydown={onEditKeydown}
                      autofocus
                    />
                    <button type="button" class="line-edit-btn save" onclick={saveEdit}>
                      {$t("study.music.lyrics_translation_edit_save")}
                    </button>
                    <button type="button" class="line-edit-btn cancel" onclick={cancelEdit}>
                      {$t("study.music.lyrics_translation_edit_cancel")}
                    </button>
                  </div>
                {:else}
                  <button
                    type="button"
                    class="line-translation"
                    class:edited={translations.get(i)?.edited ?? false}
                    onclick={() => startEdit(i)}
                    title={$t("study.music.lyrics_translation_edit_placeholder") as string}
                  >
                    <span class="translation-text">
                      {translations.get(i)?.translation || "—"}
                    </span>
                    {#if translations.get(i)?.edited}
                      <span class="edited-badge">
                        {$t("study.music.lyrics_translation_edited_badge")}
                      </span>
                    {/if}
                  </button>
                {/if}
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {:else if status === "plain" && lyricsStore.plain}
      <div class="scroll plain">
        {#each lyricsStore.plain.split(/\r?\n/) as line, i (i)}
          <p class="plain-line">{line || " "}</p>
        {/each}
      </div>
    {/if}

    {#if lyricsStore.source}
      <footer class="foot">
        <span>{$t("study.music.lyrics_source", { source: lyricsStore.source })}</span>
        {#if showTranslation && translationStatus === "available" && lyricsStore.translator && lyricsStore.translator !== "skipped"}
          <span class="foot-sep">·</span>
          <span>{$t("study.music.lyrics_translation_attribution", { translator: lyricsStore.translator })}</span>
        {/if}
      </footer>
    {/if}
  </aside>

  <PasteLyricsModal
    open={pasteOpen}
    {trackId}
    onclose={closePaste}
    onsaved={handleSavedLocal}
  />
{/if}

<style>
  .lyrics-panel {
    position: fixed;
    top: 14px;
    right: 14px;
    bottom: 96px;
    width: min(420px, 92vw);
    z-index: 95;
    background: rgb(20, 20, 20);
    color: rgba(255, 255, 255, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .head h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.85);
  }
  .actions { display: inline-flex; gap: 4px; }
  .ico {
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.5);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
  }
  .ico:hover { color: white; background: rgba(255, 255, 255, 0.08); }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    height: 26px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 999px;
    color: rgba(255, 255, 255, 0.55);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: color 160ms ease, background 160ms ease, border-color 160ms ease;
  }
  .toggle .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.3);
    transition: background 160ms ease, box-shadow 160ms ease;
  }
  .toggle.on {
    color: white;
    background: color-mix(in oklab, var(--accent, #7c5cff) 25%, transparent);
    border-color: color-mix(in oklab, var(--accent, #7c5cff) 60%, transparent);
  }
  .toggle.on .dot {
    background: var(--accent, #7c5cff);
    box-shadow: 0 0 8px color-mix(in oklab, var(--accent, #7c5cff) 80%, transparent);
  }
  .toggle:hover { color: white; }
  .translation-banner {
    padding: 8px 14px;
    font-size: 12px;
    line-height: 1.4;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }
  .banner-info { color: rgba(255, 255, 255, 0.7); }
  .banner-muted { color: rgba(255, 255, 255, 0.45); font-style: italic; }
  .banner-warn { color: rgba(255, 200, 120, 0.95); background: rgba(255, 200, 120, 0.06); }
  .banner-clickable {
    width: 100%;
    text-align: left;
    border: 0;
    cursor: pointer;
    align-items: flex-start;
  }
  .banner-clickable:hover { background: rgba(255, 200, 120, 0.12); }
  .banner-error { color: rgba(255, 130, 130, 0.95); background: rgba(255, 130, 130, 0.06); }
  .muted-inline { color: rgba(255, 255, 255, 0.55); font-size: 11px; }
  .line-original { display: block; }
  .line-translation {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    padding: 2px 4px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: rgba(255, 255, 255, 0.55);
    font-size: 14px;
    font-style: italic;
    line-height: 1.35;
    text-align: left;
    cursor: text;
    transition: background 160ms ease, color 160ms ease;
  }
  .line-translation:hover {
    background: rgba(255, 255, 255, 0.04);
    color: rgba(255, 255, 255, 0.8);
  }
  .line.active .line-translation { color: rgba(255, 255, 255, 0.78); }
  .line-translation.edited { color: rgba(180, 220, 255, 0.85); }
  .edited-badge {
    display: inline-flex;
    align-items: center;
    height: 16px;
    padding: 0 6px;
    border-radius: 999px;
    background: rgba(120, 180, 255, 0.14);
    color: rgba(180, 220, 255, 0.85);
    font-size: 9px;
    font-style: normal;
    font-weight: 600;
    letter-spacing: 0.04em;
  }
  .line-edit {
    display: flex;
    gap: 4px;
    margin-top: 4px;
  }
  .line-edit-input {
    flex: 1;
    min-width: 0;
    padding: 6px 10px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    color: white;
    font-size: 14px;
    font-style: italic;
  }
  .line-edit-input:focus {
    outline: none;
    border-color: var(--accent, #7c5cff);
    background: rgba(255, 255, 255, 0.08);
  }
  .line-edit-btn {
    padding: 0 10px;
    height: 32px;
    background: rgba(255, 255, 255, 0.06);
    border: 0;
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.85);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .line-edit-btn.save {
    background: var(--accent, #7c5cff);
    color: white;
  }
  .line-edit-btn:hover { filter: brightness(1.15); }
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 16px 22px 32px;
    scrollbar-width: thin;
  }
  .scroll.virtualized .line {
    content-visibility: auto;
    contain-intrinsic-size: 0 36px;
  }
  .lines {
    list-style: none;
    margin: 0;
    padding: 30vh 0;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .line {
    font-size: 18px;
    line-height: 1.4;
    color: rgba(255, 255, 255, 0.4);
    transition: color 220ms ease, transform 220ms ease, font-size 220ms ease, opacity 220ms ease;
    transform-origin: left;
    opacity: 0.85;
  }
  .line.past {
    color: rgba(255, 255, 255, 0.25);
    opacity: 0.6;
  }
  .line.active {
    color: white;
    font-weight: 700;
    font-size: 22px;
    transform: scale(1.02);
    opacity: 1;
  }
  .plain-line {
    margin: 0 0 10px;
    font-size: 14px;
    line-height: 1.6;
    color: rgba(255, 255, 255, 0.85);
  }
  .empty,
  .loading-skeleton {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: center;
    justify-content: center;
    padding: 24px;
    color: rgba(255, 255, 255, 0.65);
    font-size: 13px;
    text-align: center;
  }
  .empty-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.92);
  }
  .muted { color: rgba(255, 255, 255, 0.5); font-size: 12px; max-width: 280px; }
  .cta {
    margin-top: 4px;
    padding: 10px 18px;
    background: var(--accent, #7c5cff);
    color: white;
    border: 0;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: filter 180ms ease, transform 180ms ease;
  }
  .cta:hover { filter: brightness(1.1); }
  .cta:active { transform: scale(0.98); }
  .cta-ghost {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.18);
    color: rgba(255, 255, 255, 0.85);
  }
  .error-actions {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    justify-content: center;
  }
  .loading-skeleton {
    gap: 14px;
  }
  .skel {
    display: block;
    height: 14px;
    border-radius: 6px;
    background: linear-gradient(
      90deg,
      rgba(255, 255, 255, 0.06),
      rgba(255, 255, 255, 0.16),
      rgba(255, 255, 255, 0.06)
    );
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
  }
  .skel-a { width: 68%; }
  .skel-b { width: 82%; }
  .skel-c { width: 54%; }
  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .line {
      transition: none;
    }
    .skel {
      animation: none;
    }
  }
  .foot {
    padding: 8px 14px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.35);
    font-size: 10px;
    text-align: right;
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    flex-wrap: wrap;
  }
  .foot-sep { opacity: 0.4; }
</style>
