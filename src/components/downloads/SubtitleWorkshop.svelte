<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { t } from "$lib/i18n";
  import { showToast } from "$lib/stores/toast-store.svelte";

  let { onClose }: { onClose: () => void } = $props();

  type Cue = { start_ms: number; end_ms: number; text: string };

  let cues = $state<Cue[]>([]);
  let loadedPath = $state("");
  let busy = $state(false);
  let shiftMs = $state(0);
  let findText = $state("");
  let replaceText = $state("");
  let targetLang = $state("");
  let grammarStyle = $state<"original" | "formal" | "casual">("original");
  let tpOldA = $state("");
  let tpNewA = $state("");
  let tpOldB = $state("");
  let tpNewB = $state("");
  let peaks = $state<number[]>([]);
  let shots = $state<number[]>([]);
  let durationMs = $derived(cues.length ? cues[cues.length - 1].end_ms : 0);

  function err(e: unknown) {
    const raw = typeof e === "string" ? e : ($t("common.error") as string);
    const msg =
      raw === "ai_not_configured"
        ? ($t("downloads.sw.not_configured") as string)
        : raw === "translation_mismatch"
          ? ($t("downloads.sw.translate_mismatch") as string)
          : raw === "grammar_mismatch"
            ? ($t("downloads.sw.grammar_mismatch") as string)
            : raw;
    showToast("error", msg);
  }

  function fmt(ms: number): string {
    const s = Math.floor(ms / 1000);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    const milli = ms % 1000;
    const p = (n: number, l = 2) => String(n).padStart(l, "0");
    return `${p(h)}:${p(m)}:${p(sec)},${p(milli, 3)}`;
  }

  function parseTime(v: string, fallback: number): number {
    const m = v.trim().match(/^(\d+):(\d{1,2}):(\d{1,2})[,.](\d{1,3})$/);
    if (!m) return fallback;
    return (
      (parseInt(m[1]) * 3600 + parseInt(m[2]) * 60 + parseInt(m[3])) * 1000 +
      parseInt(m[4].padEnd(3, "0"))
    );
  }

  async function loadFile() {
    const sel = await openDialog({
      multiple: false,
      filters: [{ name: "Subtitles", extensions: ["srt", "vtt", "ass"] }],
    });
    if (!sel || typeof sel !== "string") return;
    busy = true;
    try {
      cues = await invoke<Cue[]>("subtitle_load", { path: sel });
      loadedPath = sel;
      peaks = [];
      shots = [];
    } catch (e) {
      err(e);
    } finally {
      busy = false;
    }
  }

  async function loadMedia() {
    const sel = await openDialog({ multiple: false });
    if (!sel || typeof sel !== "string") return;
    busy = true;
    try {
      peaks = await invoke<number[]>("waveform_peaks", { input: sel, buckets: 600 });
      shots = await invoke<number[]>("detect_shot_changes", { input: sel, threshold: 0.4 });
    } catch (e) {
      err(e);
    } finally {
      busy = false;
    }
  }

  function applyShift() {
    if (!shiftMs) return;
    cues = cues.map((c) => ({
      ...c,
      start_ms: Math.max(0, c.start_ms + shiftMs),
      end_ms: Math.max(0, c.end_ms + shiftMs),
    }));
  }

  function applyReplace() {
    if (!findText) return;
    let n = 0;
    cues = cues.map((c) => {
      if (c.text.includes(findText)) n++;
      return { ...c, text: c.text.split(findText).join(replaceText) };
    });
    showToast("info", $t("downloads.sw.replaced", { count: n }) as string);
  }

  function mergeWithNext(i: number) {
    if (i >= cues.length - 1) return;
    const a = cues[i];
    const b = cues[i + 1];
    const merged: Cue = {
      start_ms: a.start_ms,
      end_ms: b.end_ms,
      text: `${a.text} ${b.text}`.trim(),
    };
    cues = [...cues.slice(0, i), merged, ...cues.slice(i + 2)];
  }

  function splitCue(i: number) {
    const c = cues[i];
    const mid = Math.floor((c.start_ms + c.end_ms) / 2);
    if (mid <= c.start_ms || mid >= c.end_ms) return;
    const words = c.text.split(/\s+/);
    const half = Math.ceil(words.length / 2);
    const first: Cue = {
      start_ms: c.start_ms,
      end_ms: mid,
      text: words.slice(0, half).join(" "),
    };
    const second: Cue = {
      start_ms: mid,
      end_ms: c.end_ms,
      text: words.slice(half).join(" "),
    };
    cues = [...cues.slice(0, i), first, second, ...cues.slice(i + 1)];
  }

  function qc(i: number): string | null {
    const c = cues[i];
    if (c.end_ms <= c.start_ms) return $t("downloads.sw.qc_negative") as string;
    if (i < cues.length - 1 && cues[i + 1].start_ms < c.end_ms)
      return $t("downloads.sw.qc_overlap") as string;
    const secs = (c.end_ms - c.start_ms) / 1000;
    const cps = secs > 0 ? c.text.replace(/\s/g, "").length / secs : 99;
    if (cps > 21) return $t("downloads.sw.qc_fast") as string;
    return null;
  }

  async function translateAll() {
    if (!targetLang.trim() || !cues.length || busy) return;
    busy = true;
    try {
      cues = await invoke<Cue[]>("subtitle_translate", {
        cues,
        targetLang: targetLang.trim(),
      });
      showToast("success", $t("downloads.sw.translated") as string);
    } catch (e) {
      err(e);
    } finally {
      busy = false;
    }
  }

  async function grammarFix() {
    if (!cues.length || busy) return;
    busy = true;
    try {
      cues = await invoke<Cue[]>("subtitle_grammar_fix", {
        cues,
        style: grammarStyle,
      });
      showToast("success", $t("downloads.sw.grammar_done") as string);
    } catch (e) {
      err(e);
    } finally {
      busy = false;
    }
  }

  function autoFix() {
    if (!cues.length) return;
    const MIN_MS = 700;
    const MAX_LINE = 42;
    let changes = 0;

    let next = cues
      .filter((c) => c.text.trim().length > 0)
      .map((c) => ({ ...c, text: c.text.trim() }))
      .sort((a, b) => a.start_ms - b.start_ms);
    if (next.length !== cues.length) changes += cues.length - next.length;

    for (let i = 0; i < next.length; i++) {
      const c = next[i];
      if (c.end_ms <= c.start_ms) {
        c.end_ms = c.start_ms + MIN_MS;
        changes++;
      }
      if (c.end_ms - c.start_ms < MIN_MS) {
        c.end_ms = c.start_ms + MIN_MS;
        changes++;
      }
      if (i < next.length - 1 && c.end_ms > next[i + 1].start_ms) {
        c.end_ms = Math.max(c.start_ms + 1, next[i + 1].start_ms - 1);
        changes++;
      }
      if (!c.text.includes("\n") && c.text.length > MAX_LINE) {
        const mid = Math.floor(c.text.length / 2);
        let sp = c.text.lastIndexOf(" ", mid);
        if (sp < 1) sp = c.text.indexOf(" ", mid);
        if (sp > 0) {
          c.text = c.text.slice(0, sp) + "\n" + c.text.slice(sp + 1);
          changes++;
        }
      }
    }

    cues = next;
    showToast(
      changes ? "success" : "info",
      changes
        ? ($t("downloads.sw.autofixed", { count: changes }) as string)
        : ($t("downloads.sw.autofix_none") as string),
    );
  }

  function twoPoint() {
    const oa = parseTime(tpOldA, NaN);
    const na = parseTime(tpNewA, NaN);
    const ob = parseTime(tpOldB, NaN);
    const nb = parseTime(tpNewB, NaN);
    if ([oa, na, ob, nb].some((v) => Number.isNaN(v)) || oa === ob) {
      showToast("error", $t("downloads.sw.tp_invalid") as string);
      return;
    }
    const a = (nb - na) / (ob - oa);
    const b = na - a * oa;
    cues = cues.map((c) => ({
      ...c,
      start_ms: Math.max(0, Math.round(a * c.start_ms + b)),
      end_ms: Math.max(0, Math.round(a * c.end_ms + b)),
    }));
  }

  async function saveAs(format: "srt" | "vtt" | "ass") {
    if (!cues.length) return;
    const path = await saveDialog({
      filters: [{ name: format.toUpperCase(), extensions: [format] }],
    });
    if (!path) return;
    try {
      await invoke("subtitle_save", { path, cues, format });
      showToast("success", $t("downloads.sw.saved") as string);
    } catch (e) {
      err(e);
    }
  }
</script>

<div
  class="overlay"
  role="button"
  tabindex="0"
  onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}
  onkeydown={(e) => { if (e.key === "Escape") onClose(); }}
>
  <div class="panel" role="dialog" aria-modal="true" aria-label={$t('downloads.sw.title') as string}>
    <div class="head">
      <h2>{$t('downloads.sw.title')}</h2>
      <button class="x" onclick={onClose} aria-label={$t('common.close') as string}>✕</button>
    </div>

    <div class="toolbar">
      <button onclick={loadFile}>{$t('downloads.sw.open')}</button>
      <button onclick={loadMedia} disabled={!cues.length}>{$t('downloads.sw.load_media')}</button>
      <button class="primary" onclick={() => saveAs('srt')} disabled={!cues.length}>{$t('downloads.sw.save_srt')}</button>
      <button class="primary" onclick={() => saveAs('vtt')} disabled={!cues.length}>{$t('downloads.sw.save_vtt')}</button>
      <button class="primary" onclick={() => saveAs('ass')} disabled={!cues.length}>{$t('downloads.sw.save_ass')}</button>
    </div>

    {#if cues.length}
      {#if peaks.length}
        <div class="wave" aria-hidden="true">
          {#each peaks as p, idx (idx)}
            <span class="bar" style="height:{Math.max(2, p * 100)}%"></span>
          {/each}
          {#each shots as sec (sec)}
            <span class="shot" style="left:{durationMs ? (sec * 1000 / durationMs) * 100 : 0}%"></span>
          {/each}
        </div>
      {/if}

      <div class="ops">
        <div class="op">
          <input type="number" bind:value={shiftMs} step="100" />
          <button onclick={applyShift}>{$t('downloads.sw.shift')}</button>
        </div>
        <div class="op">
          <input type="text" placeholder={$t('downloads.sw.find') as string} bind:value={findText} />
          <input type="text" placeholder={$t('downloads.sw.replace') as string} bind:value={replaceText} />
          <button onclick={applyReplace}>{$t('downloads.sw.replace_btn')}</button>
        </div>
        <div class="op">
          <input type="text" placeholder={$t('downloads.sw.target_lang') as string} bind:value={targetLang} />
          <button onclick={translateAll} disabled={busy}>{$t('downloads.sw.translate')}</button>
        </div>
        <div class="op">
          <label for="sw-gstyle">{$t('downloads.sw.grammar_style')}</label>
          <select id="sw-gstyle" bind:value={grammarStyle}>
            <option value="original">{$t('downloads.sw.style_original')}</option>
            <option value="formal">{$t('downloads.sw.style_formal')}</option>
            <option value="casual">{$t('downloads.sw.style_casual')}</option>
          </select>
          <button onclick={grammarFix} disabled={busy}>{$t('downloads.sw.grammar')}</button>
          <button onclick={autoFix} disabled={busy}>{$t('downloads.sw.autofix')}</button>
        </div>
        <div class="op">
          <input type="text" placeholder={$t('downloads.sw.tp_old') as string} bind:value={tpOldA} />
          <input type="text" placeholder={$t('downloads.sw.tp_new') as string} bind:value={tpNewA} />
          <input type="text" placeholder={$t('downloads.sw.tp_old') as string} bind:value={tpOldB} />
          <input type="text" placeholder={$t('downloads.sw.tp_new') as string} bind:value={tpNewB} />
          <button onclick={twoPoint}>{$t('downloads.sw.twopoint')}</button>
        </div>
      </div>

      <div class="cues">
        {#each cues as c, i (i)}
          {@const flag = qc(i)}
          <div class="cue" class:flagged={flag}>
            <div class="times">
              <input
                type="text"
                value={fmt(c.start_ms)}
                onchange={(e) => (c.start_ms = parseTime((e.target as HTMLInputElement).value, c.start_ms))}
              />
              <input
                type="text"
                value={fmt(c.end_ms)}
                onchange={(e) => (c.end_ms = parseTime((e.target as HTMLInputElement).value, c.end_ms))}
              />
            </div>
            <textarea bind:value={c.text} rows="2"></textarea>
            <div class="cue-actions">
              <button onclick={() => splitCue(i)}>{$t('downloads.sw.split')}</button>
              <button onclick={() => mergeWithNext(i)} disabled={i >= cues.length - 1}>{$t('downloads.sw.merge')}</button>
            </div>
            {#if flag}<span class="flag">{flag}</span>{/if}
          </div>
        {/each}
      </div>
    {:else}
      <p class="empty">{$t('downloads.sw.empty')}</p>
    {/if}
  </div>
</div>

<style>
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
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    width: min(900px, 100%);
    max-height: 92vh;
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

  .toolbar,
  .ops,
  .op {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .ops {
    flex-direction: column;
    align-items: stretch;
  }

  button {
    padding: 7px 12px;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--button-elevated);
    color: var(--text);
    cursor: pointer;
  }

  button.primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: transparent;
    font-weight: 600;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  input,
  textarea {
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
  }

  .op input[type="text"] {
    flex: 1;
    min-width: 100px;
  }

  .wave {
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: 1px;
    height: 56px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
    padding: 2px;
    overflow: hidden;
  }

  .bar {
    flex: 1;
    background: var(--accent);
    opacity: 0.55;
    min-width: 1px;
  }

  .shot {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--error);
  }

  .cues {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .cue {
    display: grid;
    grid-template-columns: 130px 1fr auto;
    gap: 8px;
    align-items: start;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: var(--border-radius);
  }

  .cue.flagged {
    border-color: var(--warning);
  }

  .times {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .times input {
    width: 120px;
    font-variant-numeric: tabular-nums;
  }

  .cue textarea {
    resize: vertical;
    width: 100%;
  }

  .cue-actions {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .flag {
    grid-column: 1 / -1;
    font-size: 12px;
    color: var(--warning);
  }

  .empty {
    color: var(--gray);
    text-align: center;
    padding: 32px;
  }
</style>
