<script lang="ts">
  /** Google Docs/Slides/Sheets públicos + Extractor y compilador vectorial 16:9 de Google Slides. */
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { errText, onToolProgress, pct, pickDir, reveal, type ToolProgress } from "$lib/tools/rt";

  type Info = { kind: string; id: string; formats: string[]; is_published?: boolean };
  type CompileResult = { title: string; pages: number; output_path: string; format: string };
  type BridgeInfo = { port: number; slide_count: number; last_output: string | null; script: string };

  let url = $state("");
  let info = $state<Info | null>(null);
  let format = $state("pdf");
  let dest = $state("");
  let busy = $state(false);
  let out = $state<string | null>(null);
  let progress = $state<ToolProgress | null>(null);

  let svgDir = $state("");
  let compilingSvgDir = $state(false);
  let compiledResult = $state<CompileResult | null>(null);

  let bridgeInfo = $state<BridgeInfo | null>(null);
  let bridgeSlidesCount = $state(0);

  let unlistenProgress: (() => void) | null = null;
  let unlistenBridgeProgress: (() => void) | null = null;
  let unlistenBridgeDone: (() => void) | null = null;

  onMount(async () => {
    unlistenProgress = await onToolProgress((p) => {
      if (p.id.startsWith("gslides") || p.id === "gdocs") {
        progress = p;
      }
    });
    unlistenBridgeProgress = await listen<{ slide_count: number }>("gslides-bridge-progress", (ev) => {
      bridgeSlidesCount = ev.payload.slide_count;
    });
    unlistenBridgeDone = await listen<{ file: string; pages: number }>("gslides-bridge-complete", (ev) => {
      bridgeSlidesCount = 0;
      out = ev.payload.file;
      showToast("success", $t("tools.common.done") as string);
    });
    try {
      bridgeInfo = await invoke<BridgeInfo>("tool_gslides_bridge_info");
      bridgeSlidesCount = bridgeInfo.slide_count;
    } catch {
      // Ignorar en entorno sin puente activo
    }
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenBridgeProgress?.();
    unlistenBridgeDone?.();
  });

  async function parse(): Promise<Info | null> {
    const parsed = await invoke<Info | null>("tool_gdocs_parse", { url });
    info = parsed;
    if (!parsed) {
      showToast("error", $t("tools.gdocs.bad_url") as string);
    } else if (!parsed.formats.includes(format)) {
      format = parsed.formats[0];
    }
    return parsed;
  }

  async function run() {
    if (busy || !url.trim()) return;
    if (!info) {
      const parsed = await parse();
      if (!parsed) return;
    }
    if (!dest) {
      const d = await pickDir();
      if (!d) return;
      dest = d;
    }
    busy = true;
    out = null;
    progress = null;
    try {
      out = await invoke<string>("tool_gdocs_download", { url, format, dest });
      showToast("success", $t("tools.common.done") as string);
    } catch (e) {
      showToast("error", errText(e));
    } finally {
      busy = false;
    }
  }

  async function runCompileDir() {
    if (compilingSvgDir) return;
    if (!svgDir) {
      const d = await pickDir();
      if (!d) return;
      svgDir = d;
    }
    compilingSvgDir = true;
    compiledResult = null;
    progress = null;
    try {
      const res = await invoke<CompileResult>("tool_gslides_compile_dir", {
        svgDir,
        outputPdf: dest || null,
      });
      compiledResult = res;
      out = res.output_path;
      showToast("success", $t("tools.common.done") as string);
    } catch (e) {
      showToast("error", errText(e));
    } finally {
      compilingSvgDir = false;
    }
  }

  async function copyBrowserScript() {
    try {
      const latest = await invoke<BridgeInfo>("tool_gslides_bridge_info");
      bridgeInfo = latest;
      await navigator.clipboard.writeText(latest.script);
      showToast("success", $t("tools.common.copied") as string);
    } catch (e) {
      showToast("error", errText(e));
    }
  }
</script>

<div class="tool">
  <section>
    <div class="group">
      <div class="group-row">
        <div class="group-row-content"><input class="input" type="url" bind:value={url} oninput={() => (info = null)} placeholder="https://docs.google.com/presentation/d/…" onkeydown={(e) => e.key === "Enter" && run()} /></div>
        <div class="group-row-trailing"><button class="btn btn-secondary btn-sm" type="button" disabled={!url.trim()} onclick={parse}>{$t("tools.gdocs.check")}</button></div>
      </div>
      {#if info}
        <div class="group-row">
          <div class="group-row-content"><div class="group-row-title">{info.kind} <span class="mono">{info.id}</span></div></div>
          <div class="group-row-trailing"><select class="input" bind:value={format}>{#each info.formats as f (f)}<option value={f}>{f}</option>{/each}</select></div>
        </div>
      {/if}
      <div class="group-row">
        <div class="group-row-content"><div class="group-row-title">{$t("tools.common.output_folder")}</div><div class="group-row-sub mono">{dest || $t("tools.common.ask_on_run")}</div></div>
        <div class="group-row-trailing"><button class="btn btn-secondary btn-sm" type="button" onclick={async () => { const d = await pickDir(); if (d) dest = d; }}>{$t("tools.common.choose")}</button></div>
      </div>
      <div class="group-row">
        <div class="group-row-content">
          {#if busy || compilingSvgDir}
            <div class="group-row-sub">{progress?.message ?? (progress ? `${progress.done}/${progress.total ?? "?"}` : "…")}</div>
            <div class="progress"><div class="progress-fill" style:width="{pct(progress) ?? 0}%"></div></div>
          {:else}
            <div class="group-row-sub">{$t("tools.gdocs.note")}</div>
          {/if}
        </div>
        <div class="group-row-trailing"><button class="btn btn-primary" type="button" disabled={busy || !url.trim()} onclick={run}>{busy ? $t("tools.common.working") : $t("tools.common.download")}</button></div>
      </div>
    </div>
  </section>

  <section>
    <div class="group">
      <div class="group-row">
        <div class="group-row-content">
          <div class="group-row-title">SVG → PDF (16:9)</div>
          <div class="group-row-sub mono">{svgDir || $t("tools.common.ask_on_run")}</div>
        </div>
        <div class="group-row-trailing">
          <button class="btn btn-secondary btn-sm" type="button" onclick={async () => { const d = await pickDir(); if (d) svgDir = d; }}>{$t("tools.common.choose")}</button>
          <button class="btn btn-primary btn-sm" type="button" disabled={compilingSvgDir} onclick={runCompileDir}>{compilingSvgDir ? $t("tools.common.working") : "PDF"}</button>
        </div>
      </div>
      {#if bridgeInfo}
        <div class="group-row">
          <div class="group-row-content">
            <div class="group-row-title">Bookmarklet / JS ({`127.0.0.1:${bridgeInfo.port}`})</div>
            <div class="group-row-sub">{bridgeSlidesCount > 0 ? `${bridgeSlidesCount} slides en buffer` : "Extractor para consola o marcador del navegador"}</div>
          </div>
          <div class="group-row-trailing">
            <button class="btn btn-secondary btn-sm" type="button" onclick={copyBrowserScript}>{$t("tools.common.copy")}</button>
          </div>
        </div>
      {/if}
    </div>
  </section>

  {#if out}
    <section><div class="group"><div class="group-row">
      <div class="group-row-content">
        {#if compiledResult}
          <div class="group-row-title">{compiledResult.title} · {compiledResult.pages} slides</div>
        {/if}
        <div class="group-row-sub mono">{out}</div>
      </div>
      <div class="group-row-trailing"><button class="btn btn-secondary btn-sm" type="button" onclick={() => reveal(out!)}>{$t("tools.common.reveal")}</button></div>
    </div></div></section>
  {/if}
</div>

<style>
  .tool { display: flex; flex-direction: column; gap: var(--space-5); }
  .mono { font-family: var(--font-mono); font-size: var(--text-xs); overflow-wrap: anywhere; }
</style>
