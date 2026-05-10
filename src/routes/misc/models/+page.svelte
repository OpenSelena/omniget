<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { pluginInvoke } from "$lib/plugin-invoke";

  type ModelKind = "whisper" | "translation" | "llm";

  type ModelSpec = {
    id: string;
    kind: ModelKind;
    display_name: string;
    description: string;
    repo: string;
    filename: string;
    revision: string | null;
    size_bytes_estimate: number;
    license: string;
  };

  type InstalledModel = {
    id: string;
    kind: ModelKind;
    display_name: string;
    local_path: string;
    size_bytes: number;
  };

  type ProgressPayload = {
    id: string;
    bytes_completed: number;
    total_bytes: number;
    percent: number;
    stage: "started" | "progress" | "complete";
  };

  let registry = $state<ModelSpec[]>([]);
  let installed = $state<Record<string, InstalledModel>>({});
  let progress = $state<Record<string, ProgressPayload>>({});
  let busy = $state<Record<string, boolean>>({});
  let lastError = $state<string | null>(null);

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenInstalled: UnlistenFn | null = null;

  onMount(() => {
    void initListeners();
    void refresh();
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenInstalled?.();
  });

  async function initListeners() {
    unlistenProgress = await listen("misc:models:progress", (e) => {
      const p = e.payload as ProgressPayload;
      progress = { ...progress, [p.id]: p };
    });
    unlistenInstalled = await listen("misc:models:installed", () => {
      void refreshInstalled();
    });
  }

  async function refresh() {
    try {
      registry = await pluginInvoke<ModelSpec[]>("misc", "misc:models:registry", {});
      await refreshInstalled();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    }
  }

  async function refreshInstalled() {
    try {
      const list = await pluginInvoke<InstalledModel[]>("misc", "misc:models:list", {});
      const next: Record<string, InstalledModel> = {};
      for (const m of list) next[m.id] = m;
      installed = next;
    } catch (e) {
      console.warn("[models] list failed:", e);
    }
  }

  async function install(spec: ModelSpec) {
    if (busy[spec.id]) return;
    busy = { ...busy, [spec.id]: true };
    lastError = null;
    progress = {
      ...progress,
      [spec.id]: {
        id: spec.id,
        bytes_completed: 0,
        total_bytes: spec.size_bytes_estimate,
        percent: 0,
        stage: "started",
      },
    };
    try {
      await pluginInvoke("misc", "misc:models:install", { id: spec.id });
      await refreshInstalled();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    } finally {
      busy = { ...busy, [spec.id]: false };
      const next = { ...progress };
      delete next[spec.id];
      progress = next;
    }
  }

  async function remove(spec: ModelSpec) {
    if (busy[spec.id]) return;
    busy = { ...busy, [spec.id]: true };
    lastError = null;
    try {
      await pluginInvoke("misc", "misc:models:remove", { id: spec.id });
      await refreshInstalled();
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    } finally {
      busy = { ...busy, [spec.id]: false };
    }
  }

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function kindLabel(kind: ModelKind): string {
    switch (kind) {
      case "whisper":
        return "Transcription";
      case "translation":
        return "Translation";
      case "llm":
        return "Chat / Q&A";
    }
  }

  const grouped = $derived.by(() => {
    const out: Record<ModelKind, ModelSpec[]> = {
      whisper: [],
      translation: [],
      llm: [],
    };
    for (const s of registry) out[s.kind].push(s);
    return out;
  });
</script>

<section class="models-page">
  <header class="head">
    <h1>Models</h1>
    <p class="subtitle">
      Download AI models locally. Used for transcription, translation, and chat. All run
      on your machine — nothing is uploaded.
    </p>
  </header>

  {#if lastError}
    <div class="error" role="alert"><strong>Error:</strong> {lastError}</div>
  {/if}

  {#if registry.length === 0}
    <p class="empty">Loading registry…</p>
  {:else}
    {#each Object.keys(grouped) as kind (kind)}
      {#if grouped[kind as ModelKind].length > 0}
        <section class="kind-section">
          <h2>{kindLabel(kind as ModelKind)}</h2>
          <div class="cards">
            {#each grouped[kind as ModelKind] as spec (spec.id)}
              {@const isInstalled = !!installed[spec.id]}
              {@const isBusy = !!busy[spec.id]}
              {@const prog = progress[spec.id]}
              <div class="card" class:installed={isInstalled}>
                <div class="card-head">
                  <div class="card-title">{spec.display_name}</div>
                  <div class="card-tags">
                    <span class="tag">{spec.license}</span>
                    <span class="tag size">{fmtSize(spec.size_bytes_estimate)}</span>
                    {#if isInstalled}
                      <span class="tag tag-ok">Installed</span>
                    {/if}
                  </div>
                </div>
                <p class="card-desc">{spec.description}</p>
                <div class="repo-row mono">
                  {spec.repo} · {spec.filename}
                </div>

                {#if prog && isBusy}
                  <div class="progress-row">
                    <div class="progress-bar">
                      <div
                        class="progress-fill"
                        style="width: {Math.min(100, prog.percent).toFixed(1)}%"
                      ></div>
                    </div>
                    <div class="progress-label mono">
                      {fmtSize(prog.bytes_completed)} / {fmtSize(prog.total_bytes)}
                      ({prog.percent.toFixed(1)}%)
                    </div>
                  </div>
                {/if}

                <div class="card-actions">
                  {#if isInstalled}
                    <button
                      type="button"
                      class="btn-secondary"
                      onclick={() => remove(spec)}
                      disabled={isBusy}
                    >
                      {isBusy ? "Removing…" : "Remove"}
                    </button>
                  {:else}
                    <button
                      type="button"
                      class="btn-primary"
                      onclick={() => install(spec)}
                      disabled={isBusy}
                    >
                      {isBusy ? "Downloading…" : "Install"}
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </section>
      {/if}
    {/each}
  {/if}
</section>

<style>
  .models-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    width: 100%;
    max-width: 980px;
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
    max-width: 640px;
    line-height: 1.5;
  }

  .empty {
    color: var(--tertiary);
    font-size: 13px;
    margin: 0;
  }

  .error {
    padding: 10px 14px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, oklch(60% 0.22 25) 14%, transparent);
    color: oklch(70% 0.22 25);
    font-size: 13px;
  }

  .kind-section h2 {
    margin: 0 0 12px;
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--tertiary);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 12px;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px 18px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
  }

  .card.installed {
    border-color: color-mix(in oklab, var(--accent) 50%, var(--input-border));
    background: color-mix(in oklab, var(--accent) 4%, var(--surface));
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .card-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--secondary);
  }

  .card-tags {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .tag {
    font-size: 10px;
    font-weight: 500;
    padding: 2px 7px;
    border-radius: 4px;
    background: color-mix(in oklab, var(--input-border) 30%, transparent);
    color: var(--tertiary);
  }

  .tag.size {
    background: color-mix(in oklab, var(--input-border) 60%, transparent);
    color: var(--secondary);
  }

  .tag-ok {
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    color: var(--accent);
  }

  .card-desc {
    margin: 0;
    font-size: 12px;
    color: var(--tertiary);
    line-height: 1.5;
  }

  .repo-row {
    font-size: 11px;
    color: var(--tertiary);
    opacity: 0.75;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mono {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-variant-numeric: tabular-nums;
  }

  .progress-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .progress-bar {
    height: 6px;
    border-radius: 3px;
    background: color-mix(in oklab, var(--input-border) 60%, transparent);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease;
  }

  .progress-label {
    font-size: 11px;
    color: var(--tertiary);
  }

  .card-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }

  .btn-primary {
    padding: 7px 16px;
    border: 1px solid var(--accent);
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--accent) 14%, var(--bg));
    color: var(--accent);
    font-size: 12px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: background 120ms ease;
  }

  .btn-primary:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 22%, var(--bg));
  }

  .btn-secondary {
    padding: 7px 16px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--secondary);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
  }

  .btn-secondary:hover:not(:disabled) {
    border-color: oklch(60% 0.22 25);
    color: oklch(70% 0.22 25);
  }

  .btn-primary:disabled,
  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
