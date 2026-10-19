<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { invokeV100 } from "$lib/v100/ipc-bridge";

  interface WorkspaceItem {
    id: string;
    name: string;
    floors?: { id: string; name?: string }[];
    activeFloorId?: string;
  }

  interface TerminalNode {
    id: string;
    title: string;
    kind?: string;
  }

  const STORAGE_KEY = "omni.window.workspace";

  let workspaces = $state<WorkspaceItem[]>([]);
  let currentWorkspaceId = $state("");
  let terminals = $state<TerminalNode[]>([]);
  let isLoading = $state(true);

  function saveWorkspace(id: string) {
    try {
      localStorage.setItem(STORAGE_KEY, id);
    } catch {}
  }

  function getSavedWorkspace(): string {
    try {
      return localStorage.getItem(STORAGE_KEY) ?? "";
    } catch {
      return "";
    }
  }

  async function loadFloorNodes(wsId: string) {
    const ws = workspaces.find((w) => w.id === wsId);
    const floors = ws?.floors?.map((f) => f.id) ?? [ws?.activeFloorId ?? "ground"];
    const found: TerminalNode[] = [];

    for (const fl of floors) {
      try {
        const res = await invokeV100("canvas_load", { workspaceId: wsId, floorId: fl });
        for (const n of res?.nodes ?? []) {
          if (n.kind === "terminal" || n.kind === "agent") {
            found.push({
              id: n.id,
              title: n.title ?? n.id.slice(0, 6),
              kind: n.kind
            });
          }
        }
      } catch {}
    }
    terminals = found;
  }

  async function refreshWorkspaces() {
    try {
      const res = await invokeV100("canvas_workspaces", {});
      workspaces = Array.isArray(res) ? res : [];
    } catch {
      workspaces = [];
    }

    if (workspaces.length === 0) {
      // Default initial workspace
      workspaces = [{ id: "home", name: "Home Workspace" }];
    }

    const saved = getSavedWorkspace();
    currentWorkspaceId = workspaces.some((w) => w.id === saved) ? saved : (workspaces[0]?.id ?? "");

    if (currentWorkspaceId) {
      await loadFloorNodes(currentWorkspaceId);
    }
    isLoading = false;
  }

  async function handleWorkspaceChange(wsId: string) {
    currentWorkspaceId = wsId;
    saveWorkspace(wsId);
    await loadFloorNodes(wsId);
  }

  async function focusMainWindow() {
    try {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const main = await WebviewWindow.getByLabel("main");
      await main?.show();
      await main?.setFocus();
    } catch {}
  }

  async function hideCompanion() {
    try {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      getCurrentWebviewWindow().hide();
    } catch {}
  }

  function handleKeyDown(e: KeyboardEvent) {
    const isHotkey = (e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === "o";
    if (e.key === "Escape" || isHotkey) {
      e.preventDefault();
      hideCompanion();
    }
  }

  onMount(() => {
    refreshWorkspaces();
    const handleFocus = () => {
      refreshWorkspaces();
    };
    window.addEventListener("focus", handleFocus);
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("focus", handleFocus);
      window.removeEventListener("keydown", handleKeyDown);
    };
  });
</script>

<svelte:window oncontextmenu={(e) => e.preventDefault()} />

<main class="omni">
  <!-- Header Bar with Tauri Drag Region -->
  <header data-tauri-drag-region>
    <div class="header-brand" data-tauri-drag-region>
      <span class="omni-orb"></span>
      <strong data-tauri-drag-region>Omni Companion</strong>
    </div>

    <!-- Workspace Selector Dropdown -->
    {#if workspaces.length > 1}
      <select
        value={currentWorkspaceId}
        onchange={(e) => handleWorkspaceChange(e.currentTarget.value)}
        aria-label={$t("canvas.companion.window.workspace") || "Workspace"}
      >
        {#each workspaces as ws (ws.id)}
          <option value={ws.id}>{ws.name}</option>
        {/each}
      </select>
    {/if}

    <button
      type="button"
      class="ghost"
      title={$t("canvas.companion.window.hide") || "Hide (⌘⇧O / Esc)"}
      onclick={hideCompanion}
    >
      ✕
    </button>
  </header>

  <!-- Companion Node Body -->
  <div class="body">
    <div class="companion-pane">
      {#if isLoading}
        <div class="empty-state">
          <span>Loading spatial nodes…</span>
        </div>
      {:else if terminals.length === 0}
        <div class="empty-state">
          <p class="muted">No active terminals or agent nodes on this floor.</p>
          <button type="button" class="btn-focus" onclick={focusMainWindow}>
            Open Canvas in Main Window &rarr;
          </button>
        </div>
      {:else}
        <div class="terminals-list">
          {#each terminals as term (term.id)}
            <div class="term-row">
              <span class="term-tag {term.kind ?? 'terminal'}">
                {term.kind === "agent" ? "AGENT" : "TERM"}
              </span>
              <span class="term-title">{term.title}</span>
              <button
                type="button"
                class="btn-nav"
                onclick={focusMainWindow}
                title="Focus on canvas"
              >
                ↗
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</main>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    background: var(--surface, #1c1c1e);
  }

  .omni {
    display: flex;
    flex-direction: column;
    height: 100vh;
    color: var(--text, #f5f5f7);
    background: var(--surface, #1c1c1e);
    font-size: 13px;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    user-select: none;
    -webkit-user-select: none;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border, rgba(255, 255, 255, 0.08));
    cursor: default;
  }

  .header-brand {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .omni-orb {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #6366f1;
    box-shadow: 0 0 8px #6366f1;
  }

  header strong {
    font-size: 13.5px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  select {
    max-width: 150px;
    height: 24px;
    padding: 0 8px;
    border-radius: 6px;
    border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
    background: rgba(255, 255, 255, 0.06);
    color: inherit;
    font: inherit;
    font-size: 11.5px;
    outline: none;
  }

  .ghost {
    all: unset;
    cursor: pointer;
    padding: 2px 7px;
    border-radius: 6px;
    color: var(--text-muted, #98989d);
    font-size: 12px;
  }
  .ghost:hover {
    background: var(--hover, rgba(127, 127, 127, 0.15));
    color: var(--text, #f5f5f7);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px;
  }

  .companion-pane {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    text-align: center;
    color: var(--text-muted, #98989d);
    padding: 20px;
  }
  .muted {
    margin: 0;
    font-size: 12px;
  }

  .btn-focus {
    padding: 6px 14px;
    border-radius: 8px;
    border: 1px solid rgba(99, 102, 241, 0.4);
    background: rgba(99, 102, 241, 0.12);
    color: #818cf8;
    font: inherit;
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
  }
  .btn-focus:hover {
    background: rgba(99, 102, 241, 0.22);
  }

  .terminals-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .term-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .term-tag {
    font-size: 9px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-muted, #98989d);
  }
  .term-tag.agent {
    background: rgba(99, 102, 241, 0.2);
    color: #818cf8;
  }

  .term-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .btn-nav {
    all: unset;
    cursor: pointer;
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    color: var(--text-muted, #98989d);
  }
  .btn-nav:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }
</style>
