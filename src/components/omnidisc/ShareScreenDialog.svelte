<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    refreshSources,
    startStream,
    currentPolicy,
    allowedResolutions,
    allowedFramerates,
    kbpsFor,
    codecFor,
    widthForHeight,
    getStreamError,
    clearStreamError,
    isStreamBusy,
    type StreamSources,
    type StreamSource,
    type SourceId,
    type AudioMode,
    type StreamMode,
  } from "$lib/stores/omnidisc-stream-store.svelte";

  let { onClose }: { onClose: () => void } = $props();

  let sources = $state<StreamSources | null>(null);
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let selected = $state<SourceId | null>(null);
  let audio = $state<AudioMode>({ mode: "none" });
  let height = $state(1080);
  let fps = $state(60);
  let mode = $state<StreamMode>("text");
  let cursor = $state(true);
  let custom = $state<number | null>(null);

  let policy = $derived(currentPolicy());
  let resolutions = $derived(allowedResolutions(policy));
  let framerates = $derived(allowedFramerates(policy));
  let kbps = $derived(custom ?? kbpsFor(policy, height, fps));
  let codec = $derived(codecFor(policy, height, fps));
  let busy = $derived(isStreamBusy());
  let error = $derived(getStreamError());
  let allowCustom = $derived(policy?.allow_custom_bitrate ?? true);
  let hevcNote = $derived(height >= 2160 && fps > 60);

  $effect(() => {
    void load();
    return () => clearStreamError();
  });

  async function load() {
    loading = true;
    loadError = null;
    const s = await refreshSources();
    loading = false;
    if (!s) {
      loadError = getStreamError() ?? "ERR_STREAM_CAPTURE_FAILED";
      return;
    }
    sources = s;
    if (!selected && s.displays.length > 0) selected = s.displays[0].id;
    if (resolutions.length && !resolutions.includes(height)) height = resolutions[resolutions.length - 1];
    if (framerates.length && !framerates.includes(fps)) fps = framerates.includes(60) ? 60 : framerates[0];
  }

  function pick(src: StreamSource) {
    selected = src.id;
  }

  function isSelected(id: SourceId): boolean {
    return selected?.kind === id.kind && selected?.id === id.id;
  }

  function label(h: number): string {
    return `${h}p`;
  }

  async function share() {
    if (!selected) return;
    const ok = await startStream({
      source: selected,
      fps,
      height,
      audio,
      bitrate_kbps: custom ?? undefined,
      mode,
      cursor,
    });
    if (ok) onClose();
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="backdrop" role="presentation" onclick={onBackdrop}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label={$t("omnidisc.stream.share_title")}>
    <header>
      <h2>{$t("omnidisc.stream.share_title")}</h2>
      <button type="button" class="close" onclick={onClose} aria-label={$t("common.cancel")}>
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    </header>

    <div class="body">
      {#if loading}
        <div class="state">
          <div class="grid" aria-hidden="true">
            {#each Array(4) as _}<div class="skeleton"></div>{/each}
          </div>
          <p class="muted">{$t("omnidisc.stream.loading_sources")}</p>
        </div>
      {:else if loadError}
        <div class="state">
          <p class="err" role="alert">{translateBackendError(loadError, $t)}</p>
          {#if loadError === "ERR_SCREEN_PERMISSION"}
            <button type="button" class="btn" onclick={() => void load()}>{$t("omnidisc.stream.recheck")}</button>
          {:else}
            <button type="button" class="btn" onclick={() => void load()}>{$t("omnidisc.voice.retry")}</button>
          {/if}
        </div>
      {:else if sources}
        <section>
          <h3>{$t("omnidisc.stream.pick_source")}</h3>
          <div class="grid">
            {#each [...sources.displays, ...sources.windows] as src (`${src.id.kind}-${src.id.id}`)}
              <button type="button" class="source" class:on={isSelected(src.id)} onclick={() => pick(src)}>
                <span class="thumb">
                  {#if src.thumbnail}<img src={src.thumbnail} alt="" />{:else}<span class="ph" aria-hidden="true"></span>{/if}
                </span>
                <span class="cap">
                  <span class="title">{src.title}</span>
                  {#if src.app_name}<span class="sub">{src.app_name}</span>{/if}
                </span>
              </button>
            {/each}
          </div>
        </section>

        <section>
          <h3>{$t("omnidisc.stream.audio")}</h3>
          <div class="pills">
            <button type="button" class="pill" class:on={audio.mode === "none"} onclick={() => (audio = { mode: "none" })}>{$t("omnidisc.stream.audio_none")}</button>
            {#if sources.system_audio_supported}
              <button type="button" class="pill" class:on={audio.mode === "system"} onclick={() => (audio = { mode: "system" })}>{$t("omnidisc.stream.audio_system")}</button>
            {/if}
          </div>
          {#if sources.app_audio_supported && sources.apps.length > 0}
            <div class="pills apps">
              {#each sources.apps as app (app.pid)}
                <button type="button" class="pill" class:on={audio.mode === "app" && audio.pid === app.pid} onclick={() => (audio = { mode: "app", pid: app.pid })}>{app.name}</button>
              {/each}
            </div>
          {/if}
        </section>

        <section>
          <h3>{$t("omnidisc.stream.quality")}</h3>
          <div class="rowlabel">{$t("omnidisc.stream.resolution")}</div>
          <div class="pills">
            {#each resolutions as h}
              <button type="button" class="pill" class:on={height === h} onclick={() => { height = h; custom = null; }}>{label(h)}</button>
            {/each}
          </div>
          <div class="rowlabel">{$t("omnidisc.stream.framerate")}</div>
          <div class="pills">
            {#each framerates as f}
              <button type="button" class="pill" class:on={fps === f} onclick={() => { fps = f; custom = null; }}>{f}</button>
            {/each}
          </div>
          <p class="muted implied">{$t("omnidisc.stream.implied_bitrate", { kbps: Math.round(kbps / 100) / 10, codec: codec.toUpperCase() })}</p>
          {#if hevcNote}
            <p class="muted note">{$t("omnidisc.stream.hevc_note")}</p>
          {/if}
          {#if allowCustom}
            <label class="custom">
              <input type="checkbox" checked={custom !== null} onchange={(e) => (custom = e.currentTarget.checked ? kbpsFor(policy, height, fps) : null)} />
              {$t("omnidisc.stream.custom_bitrate")}
            </label>
            {#if custom !== null}
              <input class="range" type="range" min={policy?.min_kbps ?? 500} max={policy?.max_kbps ?? 20000} step={policy?.step_kbps ?? 500} value={custom} oninput={(e) => (custom = Number(e.currentTarget.value))} />
            {/if}
          {/if}
        </section>

        <section>
          <h3>{$t("omnidisc.stream.mode")}</h3>
          <div class="pills">
            <button type="button" class="pill" class:on={mode === "text"} onclick={() => (mode = "text")}>{$t("omnidisc.stream.mode_text")}</button>
            <button type="button" class="pill" class:on={mode === "game"} onclick={() => (mode = "game")}>{$t("omnidisc.stream.mode_game")}</button>
          </div>
          <label class="custom">
            <input type="checkbox" bind:checked={cursor} />
            {$t("omnidisc.stream.cursor")}
          </label>
        </section>

        {#if error}
          <p class="err" role="alert">{translateBackendError(error, $t)}</p>
        {/if}
      {/if}
    </div>

    <footer>
      <button type="button" class="btn" onclick={onClose}>{$t("common.cancel")}</button>
      <button type="button" class="btn primary" disabled={!selected || busy || loading} onclick={share}>
        {busy ? $t("omnidisc.stream.starting") : $t("omnidisc.stream.share_cta", { res: `${height}p${fps}` })}
      </button>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--bg) 72%, transparent);
    padding: var(--space-4);
  }
  .dialog {
    width: min(680px, 100%);
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: none;
    border-radius: var(--radius-lg, 12px);
    box-shadow: 0 24px 60px -12px color-mix(in srgb, var(--bg) 70%, transparent);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    padding: var(--space-3) var(--space-4);
  }
  header {
    justify-content: space-between;
    border-bottom: none;
  }
  header h2 {
    font-size: var(--text-lg);
    color: var(--text);
  }
  footer {
    justify-content: flex-end;
    gap: var(--space-2);
    border-top: none;
  }
  .close {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .close:hover {
    background: var(--fill-1);
    color: var(--text);
  }
  .body {
    overflow-y: auto;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  section h3 {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: var(--space-2);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: var(--space-2);
  }
  .skeleton {
    aspect-ratio: 16 / 10;
    border-radius: var(--radius-sm);
    background: var(--surface-mut);
  }
  .source {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: 0;
    border: none;
    border-radius: var(--radius-md);
    background: var(--surface-mut);
    cursor: pointer;
    overflow: hidden;
    text-align: left;
  }
  .source.on {
    border-color: var(--accent);
  }
  .thumb {
    aspect-ratio: 16 / 10;
    background: var(--bg);
    display: block;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .thumb .ph {
    display: block;
    width: 100%;
    height: 100%;
    background: repeating-linear-gradient(45deg, var(--surface), var(--surface) 8px, var(--surface-mut) 8px, var(--surface-mut) 16px);
  }
  .cap {
    display: flex;
    flex-direction: column;
    padding: var(--space-2);
    min-width: 0;
  }
  .title {
    font-size: var(--text-sm);
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .apps {
    margin-top: var(--space-2);
  }
  .pill {
    padding: 6px var(--space-3);
    border: none;
    border-radius: var(--radius-full, 999px);
    background: var(--surface-mut);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .pill.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .rowlabel {
    font-size: var(--text-xs);
    color: var(--text-muted);
    margin: var(--space-2) 0 var(--space-1);
  }
  .implied {
    margin-top: var(--space-2);
  }
  .note {
    color: var(--text-secondary);
  }
  .muted {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .custom {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text);
    margin-top: var(--space-2);
  }
  .range {
    width: 100%;
    margin-top: var(--space-2);
    accent-color: var(--accent);
  }
  .btn {
    padding: 8px var(--space-4);
    border: none;
    border-radius: var(--radius-md);
    background: var(--surface-mut);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .btn:focus-visible,
  .pill:focus-visible,
  .source:focus-visible,
  .close:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
  .err {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--danger) 14%, transparent);
    color: var(--text);
    font-size: var(--text-sm);
    line-height: 1.5;
  }
  .state {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    align-items: center;
    padding: var(--space-4);
  }
</style>
