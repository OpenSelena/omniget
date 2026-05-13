<script lang="ts">
  import { onMount } from "svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import {
    soundcloudStore,
    type ScTrack,
  } from "$lib/study-music/soundcloud-store.svelte";
  import { downloadStore } from "$lib/study-music/download-store.svelte";
  import {
    getLastCodec,
    getLastDownloadDir,
    getRememberDefaults,
    setFirstDownloadDone,
    setLastCodec,
    setLastDownloadDir,
    setRememberDefaults,
  } from "$lib/study-music/local-prefs";

  type Props = {
    track: ScTrack;
    onClose: () => void;
  };

  let { track, onClose }: Props = $props();

  type Mode = "mp3" | "flac" | "advanced";
  let mode = $state<Mode>("mp3");
  let codec = $state<string>("mp3");
  let quality = $state<string>("progressive");
  let outputDir = $state<string>("");
  let remember = $state<boolean>(true);
  let downloading = $state<boolean>(false);

  onMount(() => {
    const lastCodec = getLastCodec();
    const lastDir = getLastDownloadDir();
    remember = getRememberDefaults();
    if (lastDir) outputDir = lastDir;
    if (lastCodec === "flac") {
      mode = "flac";
      codec = "flac";
    } else if (lastCodec && lastCodec !== "mp3") {
      mode = "advanced";
      codec = lastCodec;
    } else {
      mode = "mp3";
      codec = "mp3";
    }
  });

  function setMode(next: Mode) {
    mode = next;
    if (next === "mp3") codec = "mp3";
    else if (next === "flac") codec = "flac";
  }

  const advCodecs = [
    { id: "mp3", label: "MP3 192k" },
    { id: "opus", label: "Opus 96k" },
    { id: "vorbis", label: "Vorbis q3" },
    { id: "aac", label: "AAC 192k" },
    { id: "flac", label: "FLAC" },
    { id: "wav", label: "WAV PCM" },
  ];

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

  function folderName(dir: string): string {
    const parts = dir.split(/[\\/]/).filter(Boolean);
    return parts[parts.length - 1] ?? dir;
  }

  async function startDownload() {
    if (!outputDir) {
      showToast("error", "Escolhe uma pasta primeiro");
      return;
    }
    downloading = true;
    if (remember) {
      setLastCodec(codec);
      setLastDownloadDir(outputDir);
    }
    setRememberDefaults(remember);
    setFirstDownloadDone();

    const optimisticId = downloadStore.addOptimisticSingleJob({
      trackId: track.id,
      title: track.title,
      artist: track.user.username,
      artwork:
        soundcloudStore.pickArtwork(track.artwork_url ?? track.user.avatar_url) ?? null,
    });
    onClose();

    try {
      await soundcloudStore.download({
        trackId: track.id,
        codec,
        outputDir,
        quality,
      });
      showToast("success", `Pronto — salvo na pasta ${folderName(outputDir)}`);
    } catch (e) {
      downloadStore.markJobError(
        optimisticId,
        e instanceof Error ? e.message : String(e),
      );
    }
  }
</script>

<div
  class="backdrop"
  role="presentation"
  onclick={onClose}
  onkeydown={(e) => { if (e.key === "Escape") onClose(); }}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    tabindex="-1"
  >
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

    <section class="section">
      <h3>Como salvar?</h3>
      <div class="mode-pills">
        <button
          type="button"
          class="mode-pill"
          class:on={mode === "mp3"}
          onclick={() => setMode("mp3")}
        >
          <span class="pill-title">MP3</span>
          <span class="pill-sub">Recomendado · funciona em tudo</span>
        </button>
        <button
          type="button"
          class="mode-pill"
          class:on={mode === "flac"}
          onclick={() => setMode("flac")}
        >
          <span class="pill-title">FLAC</span>
          <span class="pill-sub">Sem perda · arquivo grande</span>
        </button>
        <button
          type="button"
          class="mode-pill ghost"
          class:on={mode === "advanced"}
          onclick={() => setMode("advanced")}
        >
          <span class="pill-title">⚙ Avançado…</span>
          <span class="pill-sub">Outros codecs</span>
        </button>
      </div>

      {#if mode === "advanced"}
        <div class="adv-block">
          <div class="adv-row">
            <span class="adv-label" id="codec-group-label">Codec</span>
            <div class="codec-row" role="group" aria-labelledby="codec-group-label">
              {#each advCodecs as c (c.id)}
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
          </div>
          <div class="adv-row">
            <label class="adv-label" for="quality-select">Fonte</label>
            <select id="quality-select" class="quality-select" bind:value={quality}>
              <option value="progressive">Progressivo MP3 128 (padrão)</option>
              <option value="hq">HQ AAC ~256 (Go+)</option>
              <option value="original">Original do uploader (se permitido)</option>
            </select>
          </div>
        </div>
      {/if}
    </section>

    <section class="section">
      <h3>Onde salvar?</h3>
      <div class="folder-row">
        <input
          type="text"
          bind:value={outputDir}
          placeholder="Escolhe uma pasta…"
          readonly
        />
        <button type="button" class="ghost" onclick={pickFolder}>Procurar…</button>
      </div>
      <label class="remember">
        <input type="checkbox" bind:checked={remember} />
        <span>Lembrar pasta e formato pra próxima</span>
      </label>
    </section>

    <footer class="actions">
      <button type="button" class="ghost" onclick={onClose} disabled={downloading}>
        Cancelar
      </button>
      <button
        type="button"
        class="cta"
        onclick={startDownload}
        disabled={downloading || !outputDir}
      >
        {downloading ? "Iniciando…" : "Baixar"}
      </button>
    </footer>
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
  .section { display: flex; flex-direction: column; gap: 10px; }
  .section h3 { margin: 0; font-size: 13px; font-weight: 700; color: rgba(255,255,255,0.7); text-transform: uppercase; letter-spacing: 0.05em; }
  .mode-pills { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .mode-pill {
    display: flex; flex-direction: column; gap: 2px; align-items: flex-start;
    padding: 12px 14px; background: rgba(255,255,255,0.04); border: 1px solid transparent;
    border-radius: 10px; color: white; cursor: pointer; text-align: left;
    transition: background 200ms ease, border-color 200ms ease;
  }
  .mode-pill:hover { background: rgba(255,255,255,0.08); }
  .mode-pill.on { border-color: #ff5500; background: rgba(255, 85, 0, 0.12); }
  .mode-pill.ghost { background: rgba(255,255,255,0.025); }
  .pill-title { font-size: 14px; font-weight: 700; }
  .pill-sub { font-size: 11px; color: rgba(255,255,255,0.55); }
  .mode-pill.on .pill-sub { color: rgba(255,255,255,0.7); }
  .adv-block { display: flex; flex-direction: column; gap: 10px; padding: 12px; background: rgba(255,255,255,0.03); border-radius: 8px; }
  .adv-row { display: flex; flex-direction: column; gap: 6px; }
  .adv-label { font-size: 11px; font-weight: 700; color: rgba(255,255,255,0.55); text-transform: uppercase; letter-spacing: 0.05em; }
  .codec-row { display: flex; flex-wrap: wrap; gap: 6px; }
  .codec-pill { padding: 6px 14px; background: rgba(255,255,255,0.06); border: 1px solid transparent; border-radius: 999px; color: white; font-family: inherit; font-size: 12px; font-weight: 600; cursor: pointer; transition: background 200ms ease; }
  .codec-pill:hover { background: rgba(255,255,255,0.1); }
  .codec-pill.on { background: #ff5500; color: white; }
  .quality-select { background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.08); border-radius: 6px; color: white; padding: 8px 10px; font-family: inherit; font-size: 13px; }
  .quality-select:focus { outline: 0; border-color: #ff5500; }
  .folder-row { display: flex; gap: 8px; }
  .folder-row input { flex: 1; background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.05); border-radius: 6px; color: white; padding: 8px 12px; font-family: inherit; font-size: 13px; outline: none; }
  .remember { display: flex; align-items: center; gap: 8px; font-size: 12px; color: rgba(255,255,255,0.7); cursor: pointer; user-select: none; }
  .remember input { accent-color: #ff5500; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; }
  .ghost { padding: 8px 18px; background: rgba(255,255,255,0.05); border: 0; border-radius: 999px; color: white; font-family: inherit; font-size: 13px; font-weight: 600; cursor: pointer; }
  .ghost:hover:not(:disabled) { background: rgba(255,255,255,0.1); }
  .cta { padding: 8px 24px; background: #ff5500; border: 0; border-radius: 999px; color: white; font-family: inherit; font-size: 13px; font-weight: 700; cursor: pointer; transition: background 200ms ease; }
  .cta:hover:not(:disabled) { background: #ff7733; }
  .cta:disabled, .ghost:disabled { opacity: 0.5; cursor: default; }
</style>
