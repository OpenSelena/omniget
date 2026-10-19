<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { invokeV100 } from "$lib/engine/ipc-bridge";

  interface CanvasNode {
    id: string;
    kind: "terminal" | "agent" | "note" | "text" | "file" | "portal" | "generation" | "connector";
    x: number;
    y: number;
    w: number;
    h: number;
    title?: string;
    content?: string;
    color?: string;
    status?: "idle" | "running" | "done" | "error";
  }

  interface CanvasConnection {
    id: string;
    from: string;
    to: string;
    label?: string;
    color?: string;
  }

  interface FloorItem {
    id: string;
    name: string;
  }

  interface WorkspaceInfo {
    id: string;
    name: string;
    floors?: FloorItem[];
    activeFloorId?: string;
  }

  // Camera Viewport State
  let camX = $state(0);
  let camY = $state(0);
  let camZoom = $state(1.0);
  let isPanning = $state(false);
  let panStartX = $state(0);
  let panStartY = $state(0);

  // Canvas Document State
  let currentWorkspaceId = $state("home");
  let currentFloorId = $state("ground");
  let floors = $state<FloorItem[]>([
    { id: "ground", name: "Ground Floor" },
    { id: "lab", name: "AI Agent Lab" },
    { id: "downloads", name: "Media Processing" }
  ]);

  let nodes = $state<CanvasNode[]>([
    {
      id: "agent-1",
      kind: "agent",
      x: 120,
      y: 100,
      w: 360,
      h: 240,
      title: "Omni Coordinator",
      status: "running",
      content: "Routing media downloads and synchronizing background tasks."
    },
    {
      id: "term-1",
      kind: "terminal",
      x: 540,
      y: 100,
      w: 440,
      h: 280,
      title: "ffmpeg Transcode Worker",
      status: "idle",
      content: "$ ffmpeg -i stream.m3u8 -c copy output.mp4\n[info] Ready for pipeline inputs."
    },
    {
      id: "note-1",
      kind: "note",
      x: 120,
      y: 380,
      w: 280,
      h: 180,
      title: "Pipeline Architecture",
      content: "• yt-dlp handoff for YouTube\n• HLS segment sniffer for DRM-free streams\n• Automated metadata enrichment"
    }
  ]);

  let connections = $state<CanvasConnection[]>([
    { id: "c1", from: "agent-1", to: "term-1", label: "exec" }
  ]);

  let activeTool = $state<"select" | "pan" | "agent" | "terminal" | "note" | "cord">("select");
  let selectedNodeId = $state<string | null>(null);
  let isDraggingNode = $state(false);
  let dragNodeStartX = $state(0);
  let dragNodeStartY = $state(0);
  let nodeOrigX = $state(0);
  let nodeOrigY = $state(0);

  // Cord creation state
  let cordSourceNodeId = $state<string | null>(null);

  function screenToWorld(sx: number, sy: number) {
    return {
      x: (sx - camX) / camZoom,
      y: (sy - camY) / camZoom
    };
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
      // Zoom
      const zoomFactor = e.deltaY < 0 ? 1.08 : 0.92;
      const nextZoom = Math.min(3.0, Math.max(0.15, camZoom * zoomFactor));
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      const mouseX = e.clientX - rect.left;
      const mouseY = e.clientY - rect.top;

      camX = mouseX - (mouseX - camX) * (nextZoom / camZoom);
      camY = mouseY - (mouseY - camY) * (nextZoom / camZoom);
      camZoom = nextZoom;
    } else {
      // Pan
      camX -= e.deltaX;
      camY -= e.deltaY;
    }
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.button === 1 || (e.button === 0 && activeTool === "pan")) {
      isPanning = true;
      panStartX = e.clientX - camX;
      panStartY = e.clientY - camY;
    } else if (e.button === 0 && activeTool === "select" && (e.target as HTMLElement).classList.contains("world-canvas")) {
      selectedNodeId = null;
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isPanning) {
      camX = e.clientX - panStartX;
      camY = e.clientY - panStartY;
    } else if (isDraggingNode && selectedNodeId) {
      const dx = (e.clientX - dragNodeStartX) / camZoom;
      const dy = (e.clientY - dragNodeStartY) / camZoom;
      const target = nodes.find((n) => n.id === selectedNodeId);
      if (target) {
        target.x = Math.round((nodeOrigX + dx) / 20) * 20;
        target.y = Math.round((nodeOrigY + dy) / 20) * 20;
      }
    }
  }

  function handleMouseUp() {
    isPanning = false;
    isDraggingNode = false;
  }

  function startDragNode(e: MouseEvent, node: CanvasNode) {
    e.stopPropagation();
    selectedNodeId = node.id;

    if (activeTool === "cord") {
      if (!cordSourceNodeId) {
        cordSourceNodeId = node.id;
      } else if (cordSourceNodeId !== node.id) {
        // Create connection
        connections = [
          ...connections,
          {
            id: `cord-${Date.now()}`,
            from: cordSourceNodeId,
            to: node.id
          }
        ];
        cordSourceNodeId = null;
      }
      return;
    }

    if (activeTool === "select") {
      isDraggingNode = true;
      dragNodeStartX = e.clientX;
      dragNodeStartY = e.clientY;
      nodeOrigX = node.x;
      nodeOrigY = node.y;
    }
  }

  function addNode(kind: "agent" | "terminal" | "note") {
    const center = screenToWorld(window.innerWidth / 2, window.innerHeight / 2);
    const id = `${kind}-${Date.now()}`;
    const titles = {
      agent: "New Agent",
      terminal: "Terminal Node",
      note: "Sticky Note"
    };
    const sizes = {
      agent: { w: 320, h: 220 },
      terminal: { w: 420, h: 260 },
      note: { w: 240, h: 160 }
    };
    const newNode: CanvasNode = {
      id,
      kind,
      x: Math.round(center.x / 20) * 20,
      y: Math.round(center.y / 20) * 20,
      w: sizes[kind].w,
      h: sizes[kind].h,
      title: titles[kind],
      status: "idle",
      content: kind === "terminal" ? "$ ready for input" : "Add description or prompt here…"
    };
    nodes = [...nodes, newNode];
    selectedNodeId = id;
    activeTool = "select";
  }

  function deleteSelectedNode() {
    if (!selectedNodeId) return;
    nodes = nodes.filter((n) => n.id !== selectedNodeId);
    connections = connections.filter((c) => c.from !== selectedNodeId && c.to !== selectedNodeId);
    selectedNodeId = null;
  }

  function zoomIn() {
    camZoom = Math.min(3.0, camZoom * 1.2);
  }

  function zoomOut() {
    camZoom = Math.max(0.2, camZoom / 1.2);
  }

  function resetZoom() {
    camZoom = 1.0;
    camX = 0;
    camY = 0;
  }

  function fitAll() {
    if (nodes.length === 0) {
      resetZoom();
      return;
    }
    const minX = Math.min(...nodes.map((n) => n.x));
    const minY = Math.min(...nodes.map((n) => n.y));
    const maxX = Math.max(...nodes.map((n) => n.x + n.w));
    const maxY = Math.max(...nodes.map((n) => n.y + n.h));

    const totalW = maxX - minX + 160;
    const totalH = maxY - minY + 160;
    const scaleX = window.innerWidth / totalW;
    const scaleY = window.innerHeight / totalH;
    camZoom = Math.min(1.2, Math.max(0.2, Math.min(scaleX, scaleY)));
    camX = (window.innerWidth - totalW * camZoom) / 2 - minX * camZoom + 80 * camZoom;
    camY = (window.innerHeight - totalH * camZoom) / 2 - minY * camZoom + 80 * camZoom;
  }

  async function loadCanvasDoc() {
    try {
      const res = await invokeV100("canvas_load", {
        workspaceId: currentWorkspaceId,
        floorId: currentFloorId
      });
      if (res && res.nodes && res.nodes.length > 0) {
        nodes = res.nodes;
        connections = res.connections ?? [];
      }
    } catch {}
  }

  onMount(() => {
    loadCanvasDoc();

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.key === "Delete" || e.key === "Backspace") && selectedNodeId && (e.target as HTMLElement).tagName !== "TEXTAREA") {
        deleteSelectedNode();
      }
      if (e.key === " " && !isPanning && (e.target as HTMLElement).tagName !== "TEXTAREA") {
        activeTool = "pan";
      }
    };
    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === " " && activeTool === "pan") {
        activeTool = "select";
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
    };
  });
</script>

<svelte:window onmousemove={handleMouseMove} onmouseup={handleMouseUp} />

<div class="world-page">
  <!-- Top Navigation & Floor Switcher -->
  <header class="world-header">
    <div class="brand">
      <span class="pulse-dot"></span>
      <strong>{$t("canvas.world.title") || "Omni Spatial Canvas"}</strong>
    </div>

    <!-- Floors Bar -->
    <nav class="floor-tabs" aria-label="Canvas Floors">
      {#each floors as floor (floor.id)}
        <button
          type="button"
          class:active={currentFloorId === floor.id}
          onclick={() => { currentFloorId = floor.id; loadCanvasDoc(); }}
        >
          {floor.name}
        </button>
      {/each}
    </nav>

    <!-- Floating Zoom Controls -->
    <div class="zoom-controls">
      <button type="button" class="ctrl-btn" onclick={zoomOut} title="Zoom out">-</button>
      <button type="button" class="zoom-readout" onclick={resetZoom} title="Reset zoom">
        {Math.round(camZoom * 100)}%
      </button>
      <button type="button" class="ctrl-btn" onclick={zoomIn} title="Zoom in">+</button>
      <button type="button" class="ctrl-btn" onclick={fitAll} title="Fit all">⊡</button>
    </div>
  </header>

  <!-- Left Floating Tool Palette -->
  <aside class="tool-palette">
    <button
      type="button"
      class:active={activeTool === "select"}
      onclick={() => (activeTool = "select")}
      title="Select (V)"
    >
      ↖
    </button>
    <button
      type="button"
      class:active={activeTool === "pan"}
      onclick={() => (activeTool = "pan")}
      title="Pan canvas (Space)"
    >
      ✋
    </button>
    <div class="tool-sep"></div>
    <button type="button" onclick={() => addNode("agent")} title="Add Agent node">
      🤖
    </button>
    <button type="button" onclick={() => addNode("terminal")} title="Add Terminal node">
      💻
    </button>
    <button type="button" onclick={() => addNode("note")} title="Add Sticky note">
      📝
    </button>
    <button
      type="button"
      class:active={activeTool === "cord"}
      onclick={() => (activeTool = "cord")}
      title="Connect with cord"
    >
      ⚡
    </button>
  </aside>

  <!-- Infinite 2D Spatial Canvas Area -->
  <div
    class="world-canvas"
    class:tool-pan={activeTool === "pan" || isPanning}
    onwheel={handleWheel}
    onmousedown={handleMouseDown}
    role="region"
    aria-label="Spatial Canvas"
  >
    <!-- World Transform Container -->
    <div
      class="world-container"
      style:transform="translate({camX}px, {camY}px) scale({camZoom})"
    >
      <!-- Grid Background Pattern -->
      <div class="grid-layer"></div>

      <!-- Connecting Cords / Wires SVG -->
      <svg class="cords-svg" aria-hidden="true">
        {#each connections as conn (conn.id)}
          {@const n1 = nodes.find((n) => n.id === conn.from)}
          {@const n2 = nodes.find((n) => n.id === conn.to)}
          {#if n1 && n2}
            {@const x1 = n1.x + n1.w / 2}
            {@const y1 = n1.y + n1.h / 2}
            {@const x2 = n2.x + n2.w / 2}
            {@const y2 = n2.y + n2.h / 2}
            {@const mx = (x1 + x2) / 2}
            {@const my = (y1 + y2) / 2}
            <path
              d="M {x1} {y1} C {x1 + 60} {y1}, {x2 - 60} {y2}, {x2} {y2}"
              class="cord-line"
            />
            <circle cx={mx} cy={my} r="4" class="cord-pulse" />
          {/if}
        {/each}
      </svg>

      <!-- Spatial Nodes -->
      {#each nodes as node (node.id)}
        <div
          class="canvas-node node-{node.kind}"
          class:selected={selectedNodeId === node.id}
          style:left="{node.x}px"
          style:top="{node.y}px"
          style:width="{node.w}px"
          style:height="{node.h}px"
          onmousedown={(e) => startDragNode(e, node)}
          role="article"
        >
          <!-- Node Header Bar -->
          <div class="node-header">
            <span class="node-icon">
              {node.kind === "agent" ? "🤖" : node.kind === "terminal" ? "💻" : "📝"}
            </span>
            <span class="node-title">{node.title}</span>
            {#if node.status}
              <span class="node-status status-{node.status}"></span>
            {/if}
          </div>

          <!-- Node Content Body -->
          <div class="node-body">
            {#if node.kind === "terminal"}
              <pre class="term-content">{node.content}</pre>
            {:else if node.kind === "note"}
              <textarea
                class="note-textarea"
                bind:value={node.content}
                onmousedown={(e) => e.stopPropagation()}
              ></textarea>
            {:else}
              <p class="agent-desc">{node.content}</p>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  :global(html, body) {
    margin: 0;
    overflow: hidden;
    height: 100%;
    background: #0f1115;
  }

  .world-page {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text, #f5f5f7);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
  }

  .world-header {
    flex: none;
    height: 44px;
    background: #16181e;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    z-index: 20;
    user-select: none;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13.5px;
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #6366f1;
    box-shadow: 0 0 10px #6366f1;
  }

  .floor-tabs {
    display: flex;
    gap: 4px;
  }
  .floor-tabs button {
    background: transparent;
    border: none;
    color: #94a3b8;
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;
  }
  .floor-tabs button:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.06);
  }
  .floor-tabs button.active {
    background: rgba(99, 102, 241, 0.2);
    color: #818cf8;
  }

  .zoom-controls {
    display: flex;
    align-items: center;
    gap: 2px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 2px;
  }
  .ctrl-btn, .zoom-readout {
    background: transparent;
    border: none;
    color: #94a3b8;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11.5px;
    cursor: pointer;
  }
  .ctrl-btn:hover, .zoom-readout:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.1);
  }
  .zoom-readout {
    font-variant-numeric: tabular-nums;
  }

  .tool-palette {
    position: absolute;
    left: 16px;
    top: 60px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: #1c1e24;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    padding: 4px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    z-index: 10;
  }
  .tool-palette button {
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: #94a3b8;
    font-size: 14px;
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background 0.15s ease;
  }
  .tool-palette button:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.08);
  }
  .tool-palette button.active {
    background: #6366f1;
    color: #fff;
  }
  .tool-sep {
    height: 1px;
    background: rgba(255, 255, 255, 0.1);
    margin: 2px 0;
  }

  .world-canvas {
    flex: 1;
    position: relative;
    overflow: hidden;
    cursor: default;
    background: #0f1115;
  }
  .world-canvas.tool-pan {
    cursor: grab;
  }

  .world-container {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
    will-change: transform;
    pointer-events: auto;
  }

  .grid-layer {
    position: absolute;
    left: -10000px;
    top: -10000px;
    width: 20000px;
    height: 20000px;
    background-image: radial-gradient(rgba(255, 255, 255, 0.1) 1px, transparent 1px);
    background-size: 20px 20px;
    pointer-events: none;
  }

  .cords-svg {
    position: absolute;
    left: -10000px;
    top: -10000px;
    width: 20000px;
    height: 20000px;
    pointer-events: none;
    overflow: visible;
  }
  .cord-line {
    fill: none;
    stroke: #6366f1;
    stroke-width: 2.5;
    stroke-dasharray: 6 3;
    animation: dash-flow 1.5s linear infinite;
  }
  .cord-pulse {
    fill: #818cf8;
    filter: drop-shadow(0 0 6px #6366f1);
  }
  @keyframes dash-flow {
    to { stroke-dashoffset: -9; }
  }

  .canvas-node {
    position: absolute;
    border-radius: 12px;
    background: #1c1e24;
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    cursor: move;
    user-select: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }
  .canvas-node.selected {
    border-color: #6366f1;
    box-shadow: 0 0 0 2px #6366f1, 0 12px 36px rgba(0, 0, 0, 0.6);
  }
  .node-note {
    background: #1e1e18;
    border-color: rgba(246, 216, 96, 0.3);
  }
  .node-note .node-header {
    background: rgba(246, 216, 96, 0.1);
  }

  .node-header {
    height: 32px;
    padding: 0 10px;
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255, 255, 255, 0.04);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .node-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 650;
  }
  .node-status {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .status-running { background: #34c759; box-shadow: 0 0 6px #34c759; }
  .status-idle { background: #94a3b8; }
  .status-error { background: #ef4444; }

  .node-body {
    flex: 1;
    min-height: 0;
    padding: 10px;
    overflow: auto;
  }

  .term-content {
    margin: 0;
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
    line-height: 1.45;
    color: #cbd5e1;
    white-space: pre-wrap;
  }
  .note-textarea {
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    border: none;
    background: transparent;
    color: #fef08a;
    font-family: inherit;
    font-size: 12.5px;
    line-height: 1.5;
    resize: none;
    outline: none;
  }
  .agent-desc {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: #94a3b8;
  }
</style>
