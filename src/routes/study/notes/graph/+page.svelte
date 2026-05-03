<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { goto } from "$app/navigation";
  import {
    notesGraph,
    type Graph,
    type GraphFilter,
    type GraphNode,
  } from "$lib/notes-bridge";

  type CyHandle = {
    destroy: () => void;
    json: (s: { elements: unknown[] }) => unknown;
    elements: (selector?: string) => {
      removeClass: (c: string) => unknown;
      addClass: (c: string) => unknown;
    };
    $: (s: string) => {
      addClass: (c: string) => unknown;
      removeClass: (c: string) => unknown;
      neighborhood: () => {
        addClass: (c: string) => unknown;
        removeClass: (c: string) => unknown;
      };
    };
    fit: (eles?: unknown, padding?: number) => void;
    layout: (opts: Record<string, unknown>) => { run: () => void };
    on: {
      (event: string, handler: (e: unknown) => void): void;
      (event: string, selector: string, handler: (e: unknown) => void): void;
    };
    reset: () => void;
  };

  let container = $state<HTMLDivElement | undefined>(undefined);
  let cy: CyHandle | null = null;

  let graphData = $state<Graph>({ nodes: [], edges: [] });
  let loading = $state(true);
  let error = $state("");

  let filter = $state<GraphFilter>({
    include_tags: [],
    exclude_tags: [],
    namespace_prefix: null,
    min_refs: null,
  });

  let layoutKind = $state<"force" | "circle" | "grid" | "concentric">("force");
  let includeTagInput = $state("");
  let excludeTagInput = $state("");
  let prefixInput = $state("");
  let minRefsInput = $state(0);

  let hoverInfo = $state<{ node: GraphNode; x: number; y: number } | null>(null);

  function tagColor(name: string | undefined): string {
    if (!name) return "#5b9eff";
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = (hash << 5) - hash + name.charCodeAt(i);
      hash |= 0;
    }
    const palette = [
      "#5b9eff",
      "#7bd389",
      "#e76f51",
      "#f4a261",
      "#a78bfa",
      "#ec4899",
      "#14b8a6",
      "#f59e0b",
      "#06b6d4",
      "#ef4444",
    ];
    return palette[Math.abs(hash) % palette.length];
  }

  function nodeSize(n: GraphNode): number {
    return Math.max(8, Math.min(32, 8 + Math.sqrt(n.ref_count) * 4));
  }

  async function loadGraph() {
    loading = true;
    error = "";
    try {
      graphData = await notesGraph(filter);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function buildElements(): { group: "nodes" | "edges"; data: Record<string, unknown> }[] {
    const out: { group: "nodes" | "edges"; data: Record<string, unknown> }[] = [];
    for (const n of graphData.nodes) {
      const primary = n.tags[0] ?? "";
      out.push({
        group: "nodes",
        data: {
          id: String(n.id),
          label: n.name,
          refCount: n.ref_count,
          color: tagColor(primary),
          size: nodeSize(n),
          name: n.name,
        },
      });
    }
    for (let i = 0; i < graphData.edges.length; i++) {
      const e = graphData.edges[i];
      out.push({
        group: "edges",
        data: {
          id: `e${i}`,
          source: String(e.from),
          target: String(e.to),
          weight: e.weight,
        },
      });
    }
    return out;
  }

  function getLayoutOptions(): Record<string, unknown> {
    if (layoutKind === "circle") {
      return { name: "circle", animate: false, padding: 30 };
    }
    if (layoutKind === "grid") {
      return { name: "grid", animate: false, padding: 30 };
    }
    if (layoutKind === "concentric") {
      return {
        name: "concentric",
        animate: false,
        padding: 30,
        concentric: (node: { data: (k: string) => number }) => node.data("refCount"),
        levelWidth: () => 1,
      };
    }
    return {
      name: "cose",
      animate: false,
      padding: 30,
      idealEdgeLength: () => 80,
      nodeOverlap: 20,
      gravity: 0.5,
      numIter: 1000,
      randomize: true,
    };
  }

  async function mountCytoscape() {
    if (!container) return;
    const mod = await import("cytoscape");
    const cytoscape = (mod.default ?? mod) as unknown as (
      opts: Record<string, unknown>,
    ) => CyHandle;
    if (cy) {
      cy.destroy();
      cy = null;
    }
    cy = cytoscape({
      container,
      elements: buildElements(),
      style: [
        {
          selector: "node",
          style: {
            "background-color": "data(color)",
            label: "data(label)",
            width: "data(size)",
            height: "data(size)",
            "font-size": "10px",
            color: "var(--text)",
            "text-valign": "bottom",
            "text-halign": "center",
            "text-margin-y": 4,
            "text-outline-color": "var(--bg)",
            "text-outline-width": 2,
            "border-width": 0,
            "transition-property": "opacity, border-width",
            "transition-duration": "180ms",
          },
        },
        {
          selector: "node.selected",
          style: { "border-width": 3, "border-color": "#5b9eff" },
        },
        { selector: "node.dimmed", style: { opacity: 0.2 } },
        {
          selector: "edge",
          style: {
            width: "mapData(weight, 1, 8, 1, 4)",
            "line-color": "#888888",
            "curve-style": "bezier",
            opacity: 0.5,
          },
        },
        { selector: "edge.dimmed", style: { opacity: 0.1 } },
        { selector: "edge.highlighted", style: { "line-color": "#5b9eff", opacity: 1 } },
      ],
      layout: getLayoutOptions(),
      wheelSensitivity: 0.2,
      minZoom: 0.2,
      maxZoom: 4,
    });

    cy.on("tap", "node", (evt: unknown) => {
      const target = (evt as { target: { id: () => string } }).target;
      const id = Number(target.id());
      selectNode(id);
    });

    cy.on("dbltap", "node", (evt: unknown) => {
      const target = (evt as { target: { data: (k: string) => string } }).target;
      const name = target.data("name");
      void goto(`/study/notes?page=${encodeURIComponent(name)}`);
    });

    cy.on("mouseover", "node", (evt: unknown) => {
      const target = (evt as {
        target: {
          id: () => string;
          renderedPosition: () => { x: number; y: number };
        };
      }).target;
      const id = Number(target.id());
      const node = graphData.nodes.find((n) => n.id === id);
      if (!node) return;
      const pos = target.renderedPosition();
      const rect = container?.getBoundingClientRect();
      if (rect) {
        hoverInfo = {
          node,
          x: rect.left + pos.x,
          y: rect.top + pos.y - 12,
        };
      }
    });

    cy.on("mouseout", "node", () => {
      hoverInfo = null;
    });

    cy.on("tap", (evt: unknown) => {
      const e = evt as { target: unknown; cy: unknown };
      if (e.target === e.cy) clearSelection();
    });
  }

  function selectNode(id: number) {
    if (!cy) return;
    cy.elements().removeClass("selected dimmed highlighted");
    cy.elements().addClass("dimmed");
    const sel = cy.$(`node[id = "${id}"]`);
    sel.addClass("selected");
    sel.removeClass("dimmed");
    const neighborhood = sel.neighborhood();
    neighborhood.removeClass("dimmed");
    neighborhood.addClass("highlighted");
  }

  function clearSelection() {
    if (!cy) return;
    cy.elements().removeClass("selected dimmed highlighted");
  }

  function reset() {
    if (!cy) return;
    cy.reset();
    cy.fit(undefined, 30);
  }

  function applyFilter() {
    filter = {
      include_tags: includeTagInput
        .split(",")
        .map((s) => s.trim())
        .filter((s) => s.length > 0),
      exclude_tags: excludeTagInput
        .split(",")
        .map((s) => s.trim())
        .filter((s) => s.length > 0),
      namespace_prefix: prefixInput.trim() || null,
      min_refs: minRefsInput > 0 ? minRefsInput : null,
    };
    void (async () => {
      await loadGraph();
      if (cy) {
        cy.json({ elements: buildElements() });
        cy.layout(getLayoutOptions()).run();
      }
    })();
  }

  function changeLayout(kind: "force" | "circle" | "grid" | "concentric") {
    layoutKind = kind;
    if (!cy) return;
    cy.layout(getLayoutOptions()).run();
  }

  function onKeyDown(e: KeyboardEvent) {
    const t = e.target as HTMLElement;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA")) return;
    if (e.key === "r" || e.key === "R") {
      e.preventDefault();
      reset();
    }
  }

  onMount(async () => {
    await loadGraph();
    await mountCytoscape();
    window.addEventListener("keydown", onKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onKeyDown);
    if (cy) {
      cy.destroy();
      cy = null;
    }
  });
</script>

<div class="graph-shell" data-surface="notes">
  <header class="head">
    <a href="/study/notes" class="back">← Notas</a>
    <h1 class="page-title">Grafo</h1>
    <span class="meta">
      {graphData.nodes.length} pages · {graphData.edges.length} arestas
    </span>
    <button type="button" class="btn ghost sm" onclick={reset}>Reset (R)</button>
  </header>

  <aside class="filters">
    <h3>Filtros</h3>
    <label class="field">
      <span>Include tags (vírgula)</span>
      <input
        type="text"
        bind:value={includeTagInput}
        placeholder="cardio, fisio"
        onkeydown={(e) => {
          if (e.key === "Enter") applyFilter();
        }}
      />
    </label>
    <label class="field">
      <span>Exclude tags</span>
      <input
        type="text"
        bind:value={excludeTagInput}
        placeholder="rascunho"
        onkeydown={(e) => {
          if (e.key === "Enter") applyFilter();
        }}
      />
    </label>
    <label class="field">
      <span>Namespace prefix</span>
      <input
        type="text"
        bind:value={prefixInput}
        placeholder="medicina/, course/, journal/"
        onkeydown={(e) => {
          if (e.key === "Enter") applyFilter();
        }}
      />
    </label>
    <label class="field">
      <span>Min refs ({minRefsInput})</span>
      <input
        type="range"
        min="0"
        max="20"
        bind:value={minRefsInput}
        onchange={applyFilter}
      />
    </label>
    <button type="button" class="btn primary sm full" onclick={applyFilter}>
      Aplicar
    </button>

    <h3>Layout</h3>
    <div class="layout-row">
      <button
        type="button"
        class="btn ghost sm"
        class:active={layoutKind === "force"}
        onclick={() => changeLayout("force")}
      >Força</button>
      <button
        type="button"
        class="btn ghost sm"
        class:active={layoutKind === "circle"}
        onclick={() => changeLayout("circle")}
      >Círculo</button>
      <button
        type="button"
        class="btn ghost sm"
        class:active={layoutKind === "grid"}
        onclick={() => changeLayout("grid")}
      >Grade</button>
      <button
        type="button"
        class="btn ghost sm"
        class:active={layoutKind === "concentric"}
        onclick={() => changeLayout("concentric")}
      >Concêntrico</button>
    </div>

    <p class="hint">
      Click no nó: foca + dim vizinhos. Dblclick: abre página. R: reset.
    </p>
  </aside>

  <main class="canvas-host">
    {#if loading}
      <div class="state">Carregando grafo…</div>
    {:else if error}
      <div class="state err">{error}</div>
    {:else if graphData.nodes.length === 0}
      <div class="state">
        <p>Sem páginas pra plotar.</p>
        <a class="btn primary sm" href="/study/notes">Criar primeira página</a>
      </div>
    {/if}
    <div bind:this={container} class="canvas"></div>
    {#if hoverInfo}
      <div
        class="tooltip"
        style:left={`${hoverInfo.x}px`}
        style:top={`${hoverInfo.y}px`}
        role="tooltip"
      >
        <strong>{hoverInfo.node.name}</strong>
        <span>
          {hoverInfo.node.ref_count} refs · {hoverInfo.node.block_count} blocos
        </span>
        {#if hoverInfo.node.tags.length > 0}
          <span class="tag-line">
            {hoverInfo.node.tags.map((t) => `#${t}`).join(" ")}
          </span>
        {/if}
      </div>
    {/if}
  </main>
</div>

<style>
  .graph-shell {
    display: grid;
    grid-template-columns: 240px 1fr;
    grid-template-rows: auto 1fr;
    grid-template-areas:
      "head head"
      "filters canvas";
    height: calc(100vh - var(--header-height, 64px));
    overflow: hidden;
  }
  @media (max-width: 760px) {
    .graph-shell {
      grid-template-columns: 1fr;
      grid-template-areas:
        "head"
        "filters"
        "canvas";
    }
  }
  .head {
    grid-area: head;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
  }
  .back {
    color: var(--tertiary);
    font-size: 12px;
    text-decoration: none;
  }
  .back:hover {
    color: var(--accent);
  }
  .page-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }
  .meta {
    color: var(--tertiary);
    font-size: 12px;
    margin-left: auto;
  }
  .filters {
    grid-area: filters;
    border-right: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
  }
  .filters h3 {
    margin: 4px 0 0;
    font-size: 11px;
    font-weight: 600;
    color: var(--tertiary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field span {
    font-size: 11px;
    color: var(--secondary);
  }
  .field input[type="text"] {
    padding: 6px 8px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 12px;
  }
  .field input[type="text"]:focus {
    outline: none;
    border-color: var(--accent);
  }
  .field input[type="range"] {
    width: 100%;
  }
  .layout-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }
  .hint {
    font-size: 11px;
    color: var(--tertiary);
    margin: 4px 0 0;
    line-height: 1.4;
  }
  .canvas-host {
    grid-area: canvas;
    position: relative;
    overflow: hidden;
  }
  .canvas {
    position: absolute;
    inset: 0;
    background: var(--bg);
  }
  .state {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--secondary);
    z-index: 5;
    pointer-events: none;
  }
  .state.err {
    color: var(--error, var(--accent));
  }
  .state .btn {
    pointer-events: auto;
  }
  .tooltip {
    position: fixed;
    transform: translate(-50%, -100%);
    background: var(--surface);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    padding: 6px 10px;
    font-size: 11px;
    color: var(--text);
    box-shadow: 0 8px 20px color-mix(in oklab, black 24%, transparent);
    pointer-events: none;
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-width: 280px;
  }
  .tooltip strong {
    font-size: 12px;
  }
  .tag-line {
    color: var(--accent);
    font-size: 10px;
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .btn {
    padding: 6px 10px;
    border-radius: var(--border-radius);
    border: 1px solid transparent;
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 150ms ease, border-color 150ms ease;
    color: var(--text);
  }
  .btn.primary {
    background: var(--accent);
    color: var(--on-accent, var(--on-cta, white));
  }
  .btn.ghost {
    background: transparent;
    border-color: var(--input-border);
  }
  .btn.ghost:hover,
  .btn.ghost.active {
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn.sm {
    padding: 4px 10px;
    font-size: 11px;
  }
  .btn.full {
    width: 100%;
  }
</style>
