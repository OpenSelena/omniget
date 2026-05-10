<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import {
    soundcloudStore,
    type ScQuality,
    type ScTrack,
  } from "$lib/study-music/soundcloud-store.svelte";

  type Props = {
    track: ScTrack;
    onClose: () => void;
  };

  let { track, onClose }: Props = $props();

  let qualities = $state<ScQuality[]>([]);
  let selectedQuality = $state<string>("");
  let codec = $state<string>("mp3");
  let outputDir = $state<string>("");
  let loading = $state(true);
  let downloading = $state(false);
  let progressMs = $state(0);
  let stage = $state<string>("");
  let unlisten: UnlistenFn | null = null;

  const codecs = [
    { id: "mp3", label: "MP3 192k" },
    { id: "opus", label: "Opus 96k" },
    { id: "vorbis", label: "Vorbis q3" },
    { id: "aac", label: "AAC 192k" },
    { id: "flac", label: "FLAC" },
    { id: "wav", label: "WAV PCM" },
  ];

  onMount(() => {
    void (async () => {
      try {
        qualities = await soundcloudStore.listQualities(track.id);
        const original = qualities.find((q) => q.quality === "original");
        const hq = qualities.find((q) => q.is_premium);
        const prog = qualities.find((q) => q.protocol === "progressive");
        selectedQuality = (original ?? hq ?? prog ?? qualities[0])?.quality ?? "progressive";
      } catch (e) {
        showToast("error", e instanceof Error ? e.message : String(e));
      } finally {
        loading = false;
      }

      try {
        unlisten = await listen<{
          job_id: number;
          track_id: number;
          stage: string;
          progress_ms?: number;
          error?: string;
        }>("study-soundcloud-download-progress", (e) => {
          if (e.payload.track_id !== track.id) return;
          stage = e.payload.stage;
          if (typeof e.payload.progress_ms === "number") progressMs = e.payload.progress_ms;
        });
      } catch {
        /* ignore */
      }
    })();

    return () => {
      unlisten?.();
    };
  });

  async function pickFolder() {
    try {
      const dialog = await import("@tauri-apps/plugin-dialog");
      const picked = await dialog.open({ directory: true, multiple: false });
      if (typeof picked === "string" && picked) {
        outputDir = picked;
      }
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function startDownload() {
    if (!outputDir) {
      showToast("error", "Escolha uma pasta de destino");
      return;
    }
    downloading = true;
    progressMs = 0;
    stage = "starting";
    try {
      const res = await soundcloudStore.download({
        trackId: track.id,
        codec,
        outputDir,
        quality: selectedQuality,
      });
      showToast("success", `Salvo em: ${res.path}`);
      onClose();
    } catch (e) {
      showToast("error", e instanceof Error ? e.message : String(e));
      downloading = false;
    }
  }

  const totalMs = $derived(track.duration);
  const progressPct = $derived(
    totalMs > 0 ? Math.min(100, Math.round((progressMs / totalMs) * 100)) : 0,
  );
</script>

<div class="backdrop" role="presentation" onclick={onClose} onkeydown={(e) => { if (e.key === "Escape") onClose(); }}>
  <div class="dialog" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} tabindex="-1">
    <header class="head">
      <div class="info">
        {#if track.artwork_url}
          <img src={soundcloudStore.pickArtwork(track.artwork_url)} alt="" />
        {/if}
        <div class="meta">
          <span class="title">{track.title}</span>
          <span class="artist">{track.user.username}</span>
        </div>
      </div>
      <button type="button" class="close" onclick={onClose} aria-label="Fechar">×</button>
    </header>

    {#if loading}
      <p class="muted">Carregando opções de qualidade…</p>
    {:else}
      <section class="section">
        <h3>Qualidade</h3>
        <div class="quality-grid">
          {#each qualities as q (q.id)}
            <label class="quality-card" class:on={selectedQuality === q.quality}>
              <input
                type="radio"
                name="quality"
                value={q.quality}
                bind:group={selectedQuality}
              />
              <div class="quality-content">
                <span class="quality-label">{q.label}</span>
                <span class="quality-meta">
                  {q.protocol === "download" ? "Download direto" : q.protocol.toUpperCase()}
                  {#if q.is_premium}
                    <span class="premium-tag">Go+</span>
                  {/if}
                </span>
              </div>
            </label>
          {/each}
        </div>
      </section>

      <section class="section">
        <h3>Formato de saída</h3>
        <div class="codec-row">
          {#each codecs as c (c.id)}
            <button
              type="button"
              class="codec-pill"
              class:on={codec === c.id}
              onclick={() => (codec = c.id)}
            >
              {c.label}
            </button>
          {/each}
        </div>
      </section>

      <section class="section">
        <h3>Salvar em</h3>
        <div class="folder-row">
          <input type="text" bind:value={outputDir} placeholder="Escolha a pasta…" readonly />
          <button type="button" class="ghost" onclick={pickFolder}>Procurar…</button>
        </div>
      </section>

      {#if downloading}
        <div class="progress-block">
          <div class="progress-bar">
            <div class="progress-fill" style:width={`${progressPct}%`}></div>
          </div>
          <p class="progress-text">{stage} · {progressPct}%</p>
        </div>
      {/if}

      <footer class="actions">
        <button type="button" class="ghost" onclick={onClose} disabled={downloading}>
          Cancelar
        </button>
        <button
          type="button"
          class="cta"
          onclick={startDownload}
          disabled={downloading || !outputDir || !selectedQuality}
        >
          {downloading ? "Baixando…" : "Baixar"}
        </button>
      </footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed; inset: 0; background: rgba(0, 0, 0, 0.7); z-index: 1000;
    display: grid; place-items: center; padding: 16px;
  }
  .dialog {
    width: min(560px, 100%); max-height: 90vh; overflow-y: auto;
    background: #111; border: 1px solid rgba(255,255,255,0.08); border-radius: 12px;
    color: white; padding: 20px; display: flex; flex-direction: column; gap: 16px;
  }
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
  .info { display: flex; gap: 12px; align-items: center; min-width: 0; }
  .info img { width: 56px; height: 56px; border-radius: 6px; object-fit: cover; }
  .meta { display: flex; flex-direction: column; min-width: 0; }
  .title { font-size: 15px; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .artist { font-size: 12px; color: rgba(255,255,255,0.55); }
  .close { background: transparent; border: 0; color: rgba(255,255,255,0.6); font-size: 24px; cursor: pointer; line-height: 1; }
  .close:hover { color: white; }
  .section { display: flex; flex-direction: column; gap: 8px; }
  .section h3 { margin: 0; font-size: 13px; font-weight: 700; color: rgba(255,255,255,0.7); text-transform: uppercase; letter-spacing: 0.05em; }
  .quality-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .quality-card { display: flex; gap: 8px; padding: 10px 12px; background: rgba(255,255,255,0.04); border: 1px solid transparent; border-radius: 8px; cursor: pointer; }
  .quality-card:hover { background: rgba(255,255,255,0.06); }
  .quality-card.on { border-color: #ff5500; background: rgba(255, 85, 0, 0.1); }
  .quality-card input { display: none; }
  .quality-content { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .quality-label { font-size: 13px; font-weight: 600; }
  .quality-meta { font-size: 11px; color: rgba(255,255,255,0.5); display: flex; align-items: center; gap: 6px; }
  .premium-tag { background: linear-gradient(90deg, #ff5500, #ffd700); color: black; padding: 1px 6px; border-radius: 3px; font-size: 9px; font-weight: 800; text-transform: uppercase; }
  .codec-row { display: flex; flex-wrap: wrap; gap: 6px; }
  .codec-pill { padding: 6px 14px; background: rgba(255,255,255,0.06); border: 1px solid transparent; border-radius: 999px; color: white; font-family: inherit; font-size: 12px; font-weight: 600; cursor: pointer; transition: background 200ms ease; }
  .codec-pill:hover { background: rgba(255,255,255,0.1); }
  .codec-pill.on { background: #ff5500; color: white; }
  .folder-row { display: flex; gap: 8px; }
  .folder-row input { flex: 1; background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.05); border-radius: 6px; color: white; padding: 8px 12px; font-family: inherit; font-size: 13px; outline: none; }
  .progress-block { display: flex; flex-direction: column; gap: 6px; }
  .progress-bar { height: 6px; background: rgba(255,255,255,0.1); border-radius: 3px; overflow: hidden; }
  .progress-fill { height: 100%; background: #ff5500; transition: width 200ms ease; }
  .progress-text { margin: 0; font-size: 12px; color: rgba(255,255,255,0.6); }
  .actions { display: flex; justify-content: flex-end; gap: 8px; }
  .ghost { padding: 8px 18px; background: rgba(255,255,255,0.05); border: 0; border-radius: 999px; color: white; font-family: inherit; font-size: 13px; font-weight: 600; cursor: pointer; }
  .ghost:hover:not(:disabled) { background: rgba(255,255,255,0.1); }
  .cta { padding: 8px 24px; background: #ff5500; border: 0; border-radius: 999px; color: white; font-family: inherit; font-size: 13px; font-weight: 700; cursor: pointer; transition: background 200ms ease; }
  .cta:hover:not(:disabled) { background: #ff7733; }
  .cta:disabled, .ghost:disabled { opacity: 0.5; cursor: default; }
  .muted { color: rgba(255,255,255,0.5); font-size: 13px; }
</style>
