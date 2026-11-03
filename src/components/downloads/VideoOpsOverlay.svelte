<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { mediaKind } from "$lib/video-ops/media-kind";
  import { pickWhisperModel, type WhisperModel } from "$lib/video-ops/whisper-model";

  let { filePath, onClose }: { filePath: string; onClose: () => void } = $props();

  const kind = $derived(mediaKind(filePath));

  type Stage = "idle" | "review" | "busy" | "done";
  let stage = $state<Stage>("idle");
  let instruction = $state("");
  let trimStart = $state("");
  let trimEnd = $state("");
  let proposedArgs = $state<string[]>([]);
  let outExt = $state("mp4");
  let resultPath = $state("");
  let silenceHint = $state("");

  // Preview antes de aplicar: mede o silencio sem escrever arquivo, para o
  // usuario decidir se o ganho justifica a conversao.
  async function estimateSmartSpeed() {
    if (stage === "busy") return;
    silenceHint = $t("downloads.vop.smart_speed_measuring") as string;
    try {
      const r = await invoke<{ silence_secs: number; saved_percent: number }>(
        "video_op_silence_estimate",
        { input: filePath },
      );
      silenceHint =
        r.saved_percent >= 1
          ? ($t("downloads.vop.smart_speed_estimate", {
              minutes: Math.round(r.silence_secs / 60),
              percent: Math.round(r.saved_percent),
            }) as string)
          : ($t("downloads.vop.smart_speed_no_gain") as string);
    } catch {
      silenceHint = "";
    }
  }

  function fail(e: unknown) {
    const raw = typeof e === "string" ? e : ($t("common.error") as string);
    const msg =
      raw === "ai_not_configured"
        ? ($t("downloads.vop.not_configured") as string)
        : raw;
    showToast("error", msg);
    stage = stage === "busy" ? "idle" : stage;
  }

  async function runPreset(action: string) {
    if (stage === "busy") return;
    stage = "busy";
    try {
      const r = await invoke<{ output_path: string }>("video_op_preset", {
        input: filePath,
        action,
        start: action === "trim" ? trimStart.trim() || null : null,
        end: action === "trim" ? trimEnd.trim() || null : null,
      });
      resultPath = r.output_path;
      stage = "done";
    } catch (e) {
      fail(e);
    }
  }

  let share = $state<{ id: string; url: string; qr_svg: string } | null>(null);

  async function sendToPhone(path: string) {
    try {
      await stopShare();
      share = await invoke<{ id: string; url: string; qr_svg: string }>("phone_share_start", { path });
    } catch (e) {
      fail(e);
    }
  }

  async function stopShare() {
    if (!share) return;
    const id = share.id;
    share = null;
    await invoke("phone_share_stop", { id }).catch(() => {});
  }

  function close() {
    void stopShare();
    onClose();
  }

  let burnSubtitles = $state(false);
  let subtitlesLanguage = $state("auto");

  async function makeSubtitles() {
    if (stage === "busy") return;
    stage = "busy";
    try {
      const status = await invoke<{ installed: boolean; models: WhisperModel[] }>(
        "tool_whisper_status",
      );
      const model = pickWhisperModel(status.models);
      if (!status.installed || !model) {
        showToast("info", $t("downloads.vop.subtitles_setup") as string);
        stage = "idle";
        return;
      }
      const r = await invoke<{ srt_path: string; vtt_path: string }>(
        "tool_whisper_transcribe",
        { opts: { input: filePath, model, language: subtitlesLanguage, max_len: 42 } },
      );
      resultPath = r.srt_path;
      if (burnSubtitles) {
        const b = await invoke<{ output_path: string }>("video_op_burn_subtitles", {
          input: filePath,
          subtitles: r.srt_path,
        });
        resultPath = b.output_path;
      }
      stage = "done";
    } catch (e) {
      fail(e);
    }
  }

  async function propose() {
    if (!instruction.trim() || stage === "busy") return;
    stage = "busy";
    try {
      const r = await invoke<{ args: string[]; out_ext: string }>(
        "video_op_propose",
        { instruction: instruction.trim() },
      );
      proposedArgs = r.args;
      outExt = r.out_ext;
      stage = "review";
    } catch (e) {
      fail(e);
    }
  }

  async function runProposed() {
    stage = "busy";
    try {
      const r = await invoke<{ output_path: string }>("video_op_run", {
        input: filePath,
        args: proposedArgs,
        outExt,
      });
      resultPath = r.output_path;
      stage = "done";
    } catch (e) {
      fail(e);
    }
  }
</script>

<div
  class="overlay"
  role="button"
  tabindex="0"
  onclick={(e) => { if (e.target === e.currentTarget) close(); }}
  onkeydown={(e) => { if (e.key === "Escape") close(); }}
>
  <div class="panel" role="dialog" aria-modal="true" aria-label={$t('downloads.vop.title') as string}>
    <div class="head">
      <h2>{$t('downloads.vop.title')}</h2>
      <button class="x" onclick={close} aria-label={$t('common.close') as string}>✕</button>
    </div>

    <p class="file" title={filePath}>{filePath}</p>

    {#if share}
      <div class="share">
        <img src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(share.qr_svg)}`} alt={$t('downloads.vop.send_to_phone_qr') as string} width="200" height="200" />
        <p>{$t('downloads.vop.send_to_phone_hint')}</p>
        <button onclick={() => { void stopShare(); }}>{$t('downloads.vop.send_to_phone_stop')}</button>
      </div>
    {/if}

    {#if stage === "done"}
      <div class="done">
        <p>{$t('downloads.vop.done')}</p>
        <p class="result" title={resultPath}>{resultPath}</p>
        <button onclick={() => { void sendToPhone(resultPath); }}>{$t('downloads.vop.send_to_phone')}</button>
        <button class="primary" onclick={close}>{$t('common.close')}</button>
      </div>
    {:else}
      <h3>{$t('downloads.vop.quick_actions')}</h3>
      <div class="quick">
        {#if kind === "video"}<button disabled={stage === "busy"} onclick={() => runPreset("extract_audio")}>{$t('downloads.vop.extract_audio')}</button>{/if}
        {#if kind === "video"}<button disabled={stage === "busy"} onclick={() => runPreset("mute")}>{$t('downloads.vop.mute')}</button>{/if}
        {#if kind === "video"}<button disabled={stage === "busy"} onclick={() => runPreset("to_mp4")}>{$t('downloads.vop.to_mp4')}</button>{/if}
        {#if kind === "video"}<button disabled={stage === "busy"} onclick={() => runPreset("to_gif")}>{$t('downloads.vop.to_gif')}</button>{/if}
        {#if kind === "video"}<button disabled={stage === "busy"} onclick={() => runPreset("vertical_blur")} title={$t('downloads.vop.vertical_blur_hint') as string}>{$t('downloads.vop.vertical_blur')}</button>{/if}
        {#if kind === "video"}<button disabled={stage === "busy"} onclick={() => runPreset("vertical_crop")} title={$t('downloads.vop.vertical_crop_hint') as string}>{$t('downloads.vop.vertical_crop')}</button>{/if}
        <button disabled={stage === "busy"} onclick={() => { void sendToPhone(filePath); }} title={$t('downloads.vop.send_to_phone_hint') as string}>{$t('downloads.vop.send_to_phone')}</button>
        {#if kind === "video"}<button disabled={stage === "busy"} onclick={() => runPreset("voice_boost")}>{$t('downloads.vop.voice_boost')}</button>{/if}
        {#if kind === "video" || kind === "audio"}
          <button disabled={stage === "busy"} onclick={() => { void estimateSmartSpeed(); }}>{$t('downloads.vop.smart_speed_check')}</button>
          <button disabled={stage === "busy"} onclick={() => runPreset("smart_speed")}>{$t('downloads.vop.smart_speed')}</button>
        {/if}
      </div>

      {#if kind === "video"}
        <div class="trim">
          <input type="text" placeholder={$t('downloads.vop.trim_start')} bind:value={trimStart} />
          <input type="text" placeholder={$t('downloads.vop.trim_end')} bind:value={trimEnd} />
          <button disabled={stage === "busy"} onclick={() => runPreset("trim")}>{$t('downloads.vop.trim')}</button>
        </div>
      {/if}

      {#if kind === "video" || kind === "audio"}
      <h3>{$t('downloads.vop.subtitles_title')}</h3>
      <div class="trim">
        <select bind:value={subtitlesLanguage} aria-label={$t('downloads.vop.subtitles_language') as string}>
          <option value="auto">{$t('downloads.vop.subtitles_auto')}</option>
          <option value="en">English</option>
          <option value="fr">Français</option>
          <option value="es">Español</option>
          <option value="pt">Português</option>
          <option value="de">Deutsch</option>
        </select>
        <label><input type="checkbox" bind:checked={burnSubtitles} /> {$t('downloads.vop.subtitles_burn')}</label>
        <button disabled={stage === "busy"} onclick={() => { void makeSubtitles(); }} title={$t('downloads.vop.subtitles_hint') as string}>{$t('downloads.vop.subtitles_make')}</button>
      </div>
      {/if}

      {#if silenceHint}
        <p class="vop-hint">{silenceHint}</p>
      {/if}

      <h3>{$t('downloads.vop.nl_label')}</h3>
      <div class="nl">
        <input
          type="text"
          placeholder={$t('downloads.vop.nl_placeholder')}
          bind:value={instruction}
          onkeydown={(e) => { if (e.key === "Enter") propose(); }}
        />
        <button disabled={!instruction.trim() || stage === 'busy'} onclick={propose}>
          {$t('downloads.vop.propose')}
        </button>
      </div>

      {#if stage === "review"}
        <div class="review">
          <span class="review-title">{$t('downloads.vop.review_title')}</span>
          <code>ffmpeg -i … {proposedArgs.join(" ")} …{outExt}</code>
          <span class="review-hint">{$t('downloads.vop.review_hint')}</span>
          <button class="primary" onclick={runProposed}>{$t('downloads.vop.run')}</button>
        </div>
      {/if}

      {#if stage === "busy"}
        <p class="busy">{$t('downloads.vop.processing')}</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .vop-hint {
    margin: var(--space-2, 8px) 0 0;
    font-size: 12px;
    color: var(--text-secondary, var(--text-dim));
  }

  .overlay {
    position: fixed;
    inset: 0;
    background: color-mix(in srgb, var(--bg) 70%, transparent);
    backdrop-filter: blur(3px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 24px;
  }

  .panel {
    background: var(--surface);
    border: none;
    border-radius: var(--border-radius);
    width: min(640px, 100%);
    max-height: 90vh;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .head h2 {
    margin: 0;
    font-size: 18px;
  }

  .x {
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    font-size: 16px;
  }

  .file,
  .result {
    font-size: 12px;
    color: var(--gray);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin: 0;
  }

  h3 {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--gray);
  }

  .quick,
  .trim,
  .nl {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .nl input,
  .trim input {
    flex: 1;
    min-width: 120px;
    padding: 8px 10px;
    border: none;
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
  }

  button {
    padding: 8px 14px;
    border: none;
    border-radius: var(--border-radius);
    background: var(--button-elevated);
    color: var(--text);
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  button.primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: transparent;
    font-weight: 600;
  }

  .review {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border: none;
    border-radius: var(--border-radius);
    background: var(--bg);
  }

  .review code {
    font-size: 12px;
    word-break: break-all;
    color: var(--text);
  }

  .review-title {
    font-weight: 600;
    font-size: 13px;
  }

  .review-hint {
    font-size: 12px;
    color: var(--gray);
  }

  .busy {
    color: var(--gray);
    font-size: 13px;
  }

  .done {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }
  .share {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    margin: 8px 0 12px;
    text-align: center;
  }
  .share img {
    background: #fff;
    border-radius: 8px;
  }
</style>
