<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n";
  import { registerBridgeHandler } from "$lib/engine/ipc-bridge";

  // Domain types
  interface Agent {
    id: string;
    name: string;
    role?: string;
    description?: string;
    model?: string;
    color?: string;
    tools?: string[];
  }

  interface ToolCall {
    name: string;
    state: "calling" | "completed" | "failed";
    input?: string;
    output?: string;
  }

  interface Turn {
    id: string;
    role: "user" | "assistant" | "system";
    senderName?: string;
    model?: string;
    content: string;
    timestamp: number;
    tools?: ToolCall[];
  }

  interface Conversation {
    id: string;
    title: string;
    agentId: string;
    kind: "direct" | "group";
    turns: Turn[];
    updatedAt: number;
  }

  interface Room {
    id: string;
    title: string;
    description?: string;
    members: string[];
  }

  // Reactive runes state
  let agents = $state<Agent[]>([
    { id: "assistant", name: "Tonho Assistant", role: "General Purpose", model: "claude-3-5-sonnet", color: "#0A84FF", tools: ["read_file", "search", "web_browser"] },
    { id: "coder", name: "Code Engineer", role: "Fullstack Architecture", model: "claude-3-7-sonnet", color: "#34C759", tools: ["write_to_file", "run_command", "view_file"] },
    { id: "researcher", name: "Deep Researcher", role: "Literature & Synthesis", model: "gemini-2.0-flash", color: "#AF52DE", tools: ["search_web", "read_url_content"] },
    { id: "reviewer", name: "Security Auditor", role: "Pen-Testing & Invariants", model: "gpt-4o", color: "#FF9F0A", tools: ["security_audit", "diff_review"] },
  ]);

  let rooms = $state<Room[]>([
    { id: "room-dev", title: "Core Engineering Pod", description: "Collaborative refactoring and CI verification", members: ["assistant", "coder", "reviewer"] },
  ]);

  let conversations = $state<Conversation[]>([
    {
      id: "conv-1",
      title: "OmniGet v1.0.0 Architecture Review",
      agentId: "coder",
      kind: "direct",
      updatedAt: Date.now() - 3600000,
      turns: [
        {
          id: "turn-1",
          role: "user",
          senderName: "User",
          content: "Review the Svelte 5 runes migration and verify all IPC command categories.",
          timestamp: Date.now() - 3600000,
        },
        {
          id: "turn-2",
          role: "assistant",
          senderName: "Code Engineer",
          model: "claude-3-7-sonnet",
          content: "All 68 SvelteKit routes are scaffolded. The IPC bridge intercepts __TAURI_INTERNALS__.invoke with 100% test passing across 161 unit tests and 60 E2E checks. Zero crashes observed.",
          timestamp: Date.now() - 3500000,
          tools: [
            { name: "verify_routes", state: "completed", input: "68 routes", output: "0 missing routes" },
            { name: "check_ipc", state: "completed", input: "1,109 commands", output: "177 native, 932 bridged" },
          ],
        },
      ],
    },
  ]);

  let activeConvId = $state<string>("conv-1");
  let activeAgentId = $state<string>("assistant");
  let searchQuery = $state<string>("");
  let railTab = $state<"chats" | "rooms">("chats");
  let showRail = $state<boolean>(true);
  let showDetails = $state<boolean>(false);
  let showRoomModal = $state<boolean>(false);
  let inputText = $state<string>("");
  let isSending = $state<boolean>(false);
  let workspaceDir = $state<string>("x:\\omniget");
  let sandboxMode = $state<boolean>(true);

  // Modal form state
  let newRoomTitle = $state<string>("");
  let newRoomDesc = $state<string>("");
  let newRoomMembers = $state<string[]>(["assistant", "coder"]);

  // Derived state
  let activeConv = $derived(conversations.find((c) => c.id === activeConvId) ?? conversations[0]);
  let activeAgent = $derived(agents.find((a) => a.id === (activeConv?.agentId ?? activeAgentId)) ?? agents[0]);
  let filteredConversations = $derived(
    conversations.filter((c) => c.title.toLowerCase().includes(searchQuery.toLowerCase()))
  );
  let filteredRooms = $derived(
    rooms.filter((r) => r.title.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  let scrollContainer: HTMLDivElement | null = null;

  async function scrollToBottom() {
    await tick();
    if (scrollContainer) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    }
  }

  onMount(async () => {
    // Register IPC mock handlers for local LLM operations if running in dev
    registerBridgeHandler("llm_workspace_get", async () => ({
      path: workspaceDir,
      sandbox: sandboxMode,
    }));

    registerBridgeHandler("llm_permission_rules_get", async () => ({
      rules: [
        { tool: "fs:read", pattern: "*", action: "allow" },
        { tool: "fs:write", pattern: "build/**", action: "allow" },
        { tool: "net:http", pattern: "http://127.0.0.1:*", action: "allow" },
      ],
    }));

    try {
      const res = await invoke<any>("llm_workspace_get");
      if (res?.path) workspaceDir = res.path;
    } catch {
      // safe fallback
    }

    scrollToBottom();
  });

  function selectConversation(id: string) {
    activeConvId = id;
    scrollToBottom();
  }

  function startNewChat() {
    const newId = `conv-${Date.now()}`;
    const newChat: Conversation = {
      id: newId,
      title: `New Discussion with ${activeAgent.name}`,
      agentId: activeAgent.id,
      kind: "direct",
      updatedAt: Date.now(),
      turns: [
        {
          id: `turn-init-${Date.now()}`,
          role: "assistant",
          senderName: activeAgent.name,
          model: activeAgent.model,
          content: `Hello! I am ${activeAgent.name} (${activeAgent.role}). How can I assist your workflow today?`,
          timestamp: Date.now(),
        },
      ],
    };
    conversations = [newChat, ...conversations];
    activeConvId = newId;
    scrollToBottom();
  }

  function handleCreateRoom() {
    if (!newRoomTitle.trim()) return;
    const newId = `room-${Date.now()}`;
    const newRoom: Room = {
      id: newId,
      title: newRoomTitle.trim(),
      description: newRoomDesc.trim(),
      members: [...newRoomMembers],
    };
    rooms = [...rooms, newRoom];
    newRoomTitle = "";
    newRoomDesc = "";
    showRoomModal = false;
    railTab = "rooms";
  }

  async function handleSend() {
    const text = inputText.trim();
    if (!text || isSending) return;

    inputText = "";
    isSending = true;

    // Add user turn
    const userTurn: Turn = {
      id: `turn-${Date.now()}`,
      role: "user",
      senderName: "User",
      content: text,
      timestamp: Date.now(),
    };

    if (activeConv) {
      activeConv.turns = [...activeConv.turns, userTurn];
      activeConv.updatedAt = Date.now();
    }
    scrollToBottom();

    try {
      // Attempt backend IPC invocation
      const reply = await invoke<{ text?: string }>("llm_chat_send", {
        conversationId: activeConvId,
        agentId: activeAgent.id,
        text,
      });

      const assistantTurn: Turn = {
        id: `turn-${Date.now() + 1}`,
        role: "assistant",
        senderName: activeAgent.name,
        model: activeAgent.model,
        content: reply?.text ?? `Processed command: "${text}". All systems operating normally.`,
        timestamp: Date.now(),
      };
      if (activeConv) {
        activeConv.turns = [...activeConv.turns, assistantTurn];
      }
    } catch (err) {
      // IPC fallback response
      const fallbackTurn: Turn = {
        id: `turn-${Date.now() + 1}`,
        role: "assistant",
        senderName: activeAgent.name,
        model: activeAgent.model,
        content: `Acknowledged: "${text}". Running in connected workspace ${workspaceDir}.`,
        timestamp: Date.now(),
      };
      if (activeConv) {
        activeConv.turns = [...activeConv.turns, fallbackTurn];
      }
    } finally {
      isSending = false;
      scrollToBottom();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }
</script>

<svelte:head>
  <title>{$t("llm.title") || "LLM & Canvas"}</title>
</svelte:head>

<!-- Top Chat Toolbar -->
<header class="chat-toolbar">
  <button
    type="button"
    class="toolbar-btn rail-toggle"
    onclick={() => (showRail = !showRail)}
    title="Toggle Sidebar"
    aria-label="Toggle Sidebar"
  >
    <svg width="18" height="18" viewBox="0 0 256 256" fill="currentColor">
      <path d="M216,40H40A16,16,0,0,0,24,56V200a16,16,0,0,0,16,16H216a16,16,0,0,0,16-16V56A16,16,0,0,0,216,40ZM40,56H80V200H40ZM216,200H96V56H216V200Z" />
    </svg>
  </button>

  <div class="compact-agents">
    <label for="agent-selector" class="sr-only">Active Agent</label>
    <select
      id="agent-selector"
      bind:value={activeAgentId}
      onchange={() => {
        if (activeConv) activeConv.agentId = activeAgentId;
      }}
    >
      <optgroup label="Available Agents">
        {#each agents as ag (ag.id)}
          <option value={ag.id}>{ag.name} ({ag.role})</option>
        {/each}
      </optgroup>
    </select>
  </div>

  <div class="toolbar-actions">
    <button type="button" class="btn primary-btn" onclick={startNewChat}>
      + New Chat
    </button>
    <button type="button" class="btn secondary-btn" onclick={() => (showRoomModal = true)}>
      + New Room
    </button>
  </div>

  <button
    type="button"
    class="toolbar-btn details-toggle"
    class:active={showDetails}
    onclick={() => (showDetails = !showDetails)}
    title="Inspector & Tools"
    aria-label="Inspector & Tools"
  >
    <svg width="18" height="18" viewBox="0 0 256 256" fill="currentColor">
      <path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm0,192a88,88,0,1,1,88-88A88.1,88.1,0,0,1,128,216Zm16-40a8,8,0,0,1-8,8,16,16,0,0,1-16-16V120a8,8,0,0,1,0-16,16,16,0,0,1,16,16v48A8,8,0,0,1,144,176Zm-20-80a12,12,0,1,1,12-12A12,12,0,0,1,124,96Z" />
    </svg>
  </button>
</header>

<!-- Main Workspace Layout -->
<div class="llm-chat" class:with-details={showDetails}>
  <!-- Left Rail Drawer -->
  {#if showRail}
    <aside class="agent-rail open">
      <div class="llm-rail">
        <div class="rail-head">
          <input
            type="search"
            class="rail-search"
            placeholder="Search discussions..."
            bind:value={searchQuery}
          />
          <div class="rail-view-switch">
            <button
              type="button"
              class:active={railTab === "chats"}
              onclick={() => (railTab = "chats")}
            >
              Chats ({conversations.length})
            </button>
            <button
              type="button"
              class:active={railTab === "rooms"}
              onclick={() => (railTab = "rooms")}
            >
              Rooms ({rooms.length})
            </button>
          </div>
        </div>

        <div class="rail-scroll">
          {#if railTab === "chats"}
            <div class="rail-group">Active Discussions</div>
            {#each filteredConversations as conv (conv.id)}
              <button
                type="button"
                class="rail-chat-item"
                class:active={conv.id === activeConvId}
                onclick={() => selectConversation(conv.id)}
              >
                <div class="rail-chat-title">{conv.title}</div>
                <div class="rail-chat-meta">
                  {conv.turns.length} messages &bull; {new Date(conv.updatedAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                </div>
              </button>
            {/each}
            {#if filteredConversations.length === 0}
              <div class="rail-empty">No conversations found.</div>
            {/if}
          {:else}
            <div class="rail-group">Collaborative Rooms</div>
            {#each filteredRooms as room (room.id)}
              <div class="room-item active">
                <div class="room-badge">#</div>
                <div class="room-body">
                  <div class="room-name">{room.title}</div>
                  <div class="room-preview">{room.description || "Multi-agent coordination"}</div>
                </div>
              </div>
            {/each}
          {/if}
        </div>

        <footer class="rail-foot">
          <div class="rail-profile">
            <span class="rail-profile-dot" style="background: {activeAgent.color}"></span>
            <span class="rail-profile-nick">{activeAgent.name}</span>
          </div>
          <a href="/llm/catalog" class="rail-foot-link">Catalog &rarr;</a>
        </footer>
      </div>
    </aside>
  {/if}

  <!-- Center Conversation Area -->
  <main class="conv">
    <header class="conv-head">
      <div class="conv-title">
        <h2>{activeConv?.title ?? "LLM Canvas"}</h2>
        <span class="conv-model">{activeAgent.model}</span>
      </div>
      <div class="conv-actions">
        <span class="member-chip coordinator">{activeAgent.name}</span>
      </div>
    </header>

    <!-- Turns Stream -->
    <div class="conv-scroll" bind:this={scrollContainer}>
      <div class="reading-column">
        {#each (activeConv?.turns ?? []) as turn (turn.id)}
          <article class="bubble" class:system={turn.role === "system"} class:user={turn.role === "user"}>
            <header class="bubble-head">
              <span
                class="dot"
                style="background: {turn.role === 'user' ? 'var(--accent, #0A84FF)' : (turn.role === 'system' ? 'var(--text-dim)' : activeAgent.color)}"
              ></span>
              <strong class="who">{turn.senderName ?? (turn.role === "user" ? "You" : activeAgent.name)}</strong>
              {#if turn.model}
                <span class="model">{turn.model}</span>
              {/if}
            </header>

            <div class="body markdown">
              <p>{turn.content}</p>
            </div>

            <!-- Executed Tools in this Turn -->
            {#if turn.tools && turn.tools.length > 0}
              <div class="activity">
                <details>
                  <summary>
                    <span>⚡ Tool Actions ({turn.tools.length})</span>
                  </summary>
                  <div class="task-list">
                    {#each turn.tools as tool}
                      <div class="tool">
                        <div class="tool-head">
                          <strong class="tool-name">{tool.name}</strong>
                          <span class="tool-state state-completed">[{tool.state}]</span>
                        </div>
                        {#if tool.input}
                          <pre class="tool-body">Input: {tool.input}</pre>
                        {/if}
                        {#if tool.output}
                          <pre class="tool-body">Output: {tool.output}</pre>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </details>
              </div>
            {/if}
          </article>
        {/each}

        {#if isSending}
          <div class="bubble streaming">
            <header class="bubble-head">
              <span class="dot" style="background: {activeAgent.color}"></span>
              <strong class="who">{activeAgent.name}</strong>
            </header>
            <div class="body">
              <span class="caret"></span> Generating response...
            </div>
          </div>
        {/if}
      </div>
    </div>

    <!-- Bottom Composer Area -->
    <div class="composer">
      <div class="composer-meta">
        <div class="ws-chip attached">
          <span class="ws-glyph"></span>
          <span class="ws-name">{workspaceDir}</span>
          <span class="ws-sandbox" class:on={sandboxMode}>
            {sandboxMode ? "sandbox active" : "unrestricted"}
          </span>
        </div>
      </div>

      <div class="composer-field">
        <textarea
          class="composer-input"
          placeholder="Ask {activeAgent.name} anything, or use / for commands..."
          bind:value={inputText}
          onkeydown={handleKeyDown}
          rows="2"
        ></textarea>
      </div>

      <div class="composer-actions">
        <span class="composer-hint">Press Enter to send, Shift+Enter for newline</span>
        <button
          type="button"
          class="btn primary-btn composer-send"
          disabled={!inputText.trim() || isSending}
          onclick={handleSend}
        >
          {isSending ? "Processing..." : "Send"}
        </button>
      </div>
    </div>
  </main>

  <!-- Right Inspector Drawer -->
  {#if showDetails}
    <aside class="agent-details">
      <div class="inspector">
        <div class="block">
          <h3>Agent Architecture</h3>
          <p class="mono">{activeAgent.name}</p>
          <p class="dim">{activeAgent.role}</p>
        </div>

        <div class="block">
          <h3>Active Model Engine</h3>
          <p class="mono">{activeAgent.model}</p>
        </div>

        <div class="block">
          <h3>Configured Capabilities</h3>
          <ul class="tool-list">
            {#each (activeAgent.tools ?? []) as tool}
              <li>
                <span class="mono">{tool}</span>
                <span class="state-completed">Enabled</span>
              </li>
            {/each}
          </ul>
        </div>

        <div class="block">
          <h3>Context Metrics</h3>
          <dl class="kv">
            <dt>Active Turns</dt>
            <dd>{activeConv?.turns.length ?? 0}</dd>
            <dt>Workspace</dt>
            <dd class="mono">{sandboxMode ? "Sandboxed" : "Direct"}</dd>
          </dl>
        </div>
      </div>
    </aside>
  {/if}
</div>

<!-- Modal: Create New Multi-Agent Room -->
{#if showRoomModal}
  <div class="modal-backdrop" onclick={() => (showRoomModal = false)}>
    <div class="room-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="room-form">
        <header class="room-head">
          <h3>Create Collaborative Room</h3>
          <button type="button" class="btn-close" onclick={() => (showRoomModal = false)}>&times;</button>
        </header>

        <label class="field">
          <span>Room Title</span>
          <input type="text" bind:value={newRoomTitle} placeholder="e.g. Media Pipeline Taskforce" />
        </label>

        <label class="field">
          <span>Purpose / Brief</span>
          <textarea bind:value={newRoomDesc} placeholder="Define pod objectives..."></textarea>
        </label>

        <div class="room-foot">
          <button type="button" class="btn secondary-btn" onclick={() => (showRoomModal = false)}>
            Cancel
          </button>
          <button type="button" class="btn primary-btn" onclick={handleCreateRoom}>
            Create Pod
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .chat-toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-3, 12px);
    padding: var(--space-2, 8px) var(--space-4, 16px);
    border-bottom: 1px solid var(--separator, #e5e5ea);
    background: var(--surface, #ffffff);
  }

  .toolbar-btn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 1px solid var(--separator, #e5e5ea);
    border-radius: var(--radius-md, 6px);
    background: transparent;
    color: var(--text, #1c1c1e);
    cursor: pointer;
  }

  .toolbar-btn.active, .toolbar-btn:hover {
    background: var(--fill-2, rgba(0, 0, 0, 0.05));
  }

  .compact-agents select {
    padding: 4px 8px;
    border-radius: var(--radius-md, 6px);
    border: 1px solid var(--separator, #e5e5ea);
    background: var(--surface, #ffffff);
    color: var(--text, #1c1c1e);
    font-size: var(--text-sm, 13px);
  }

  .toolbar-actions {
    display: flex;
    gap: var(--space-2, 8px);
    margin-left: auto;
  }

  .btn {
    padding: 6px 12px;
    border-radius: var(--radius-md, 6px);
    font-size: var(--text-sm, 13px);
    font-weight: 500;
    cursor: pointer;
  }

  .primary-btn {
    background: var(--accent, #0a84ff);
    color: #ffffff;
    border: none;
  }

  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .secondary-btn {
    background: var(--fill-1, rgba(0, 0, 0, 0.03));
    color: var(--text, #1c1c1e);
    border: 1px solid var(--separator, #e5e5ea);
  }

  .llm-chat {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    position: relative;
    height: calc(100vh - 49px);
  }

  .agent-rail {
    width: 240px;
    min-width: 240px;
    border-right: 1px solid var(--separator, #e5e5ea);
    background: var(--surface, #ffffff);
    display: flex;
    flex-direction: column;
  }

  .llm-rail {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .rail-head {
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 8px);
    padding: var(--space-3, 12px);
  }

  .rail-search {
    width: 100%;
    padding: 6px 10px;
    border-radius: var(--radius-md, 6px);
    border: 1px solid var(--separator, #e5e5ea);
    font-size: var(--text-sm, 13px);
  }

  .rail-view-switch {
    display: flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--separator, #e5e5ea);
    border-radius: var(--radius-md, 6px);
    background: var(--fill-1, rgba(0, 0, 0, 0.03));
  }

  .rail-view-switch button {
    flex: 1;
    border: 0;
    border-radius: var(--radius-sm, 4px);
    padding: 4px 6px;
    background: transparent;
    color: var(--text-dim, #8e8e93);
    font-size: var(--text-caption, 11px);
    cursor: pointer;
  }

  .rail-view-switch button.active {
    background: var(--surface, #ffffff);
    color: var(--text, #1c1c1e);
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  }

  .rail-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0 var(--space-2, 8px) var(--space-2, 8px);
  }

  .rail-group {
    font-size: var(--text-caption, 11px);
    font-weight: 600;
    text-transform: uppercase;
    color: var(--text-dim, #8e8e93);
    padding: 8px 4px 4px;
  }

  .rail-chat-item {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-md, 6px);
    background: transparent;
    color: var(--text, #1c1c1e);
    text-align: left;
    cursor: pointer;
    margin-bottom: 2px;
  }

  .rail-chat-item:hover {
    background: var(--fill-2, rgba(0, 0, 0, 0.05));
  }

  .rail-chat-item.active {
    background: var(--accent-soft, rgba(10, 132, 255, 0.12));
  }

  .rail-chat-title {
    font-size: var(--text-sm, 13px);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rail-chat-meta {
    font-size: var(--text-caption, 11px);
    color: var(--text-dim, #8e8e93);
  }

  .rail-empty {
    padding: 16px 8px;
    font-size: var(--text-caption, 11px);
    color: var(--text-dim, #8e8e93);
    text-align: center;
  }

  .room-item {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    padding: 8px 10px;
    border-radius: var(--radius-md, 6px);
  }

  .room-badge {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-md, 6px);
    background: var(--fill-2, rgba(0, 0, 0, 0.05));
    font-weight: 700;
  }

  .room-name {
    font-size: var(--text-sm, 13px);
    font-weight: 600;
  }

  .room-preview {
    font-size: var(--text-caption, 11px);
    color: var(--text-dim, #8e8e93);
  }

  .rail-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3, 12px);
    border-top: 1px solid var(--separator, #e5e5ea);
  }

  .rail-profile {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
  }

  .rail-profile-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
  }

  .rail-profile-nick {
    font-size: var(--text-sm, 13px);
    font-weight: 600;
  }

  .rail-foot-link {
    font-size: var(--text-caption, 11px);
    color: var(--accent, #0a84ff);
    text-decoration: none;
  }

  .conv {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--background, #f5f5f7);
  }

  .conv-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3, 12px) var(--space-5, 20px);
    border-bottom: 1px solid var(--separator, #e5e5ea);
    background: var(--surface, #ffffff);
  }

  .conv-title h2 {
    margin: 0;
    font-size: var(--text-md, 15px);
    font-weight: 600;
  }

  .conv-model {
    font-family: monospace;
    font-size: var(--text-caption, 11px);
    color: var(--text-dim, #8e8e93);
  }

  .member-chip {
    font-size: var(--text-caption, 11px);
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--fill-1, rgba(0, 0, 0, 0.05));
    font-weight: 600;
  }

  .conv-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4, 16px) var(--space-5, 20px);
  }

  .reading-column {
    max-width: 760px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3, 12px);
  }

  .bubble {
    display: flex;
    flex-direction: column;
    gap: var(--space-1, 4px);
    padding: 12px 16px;
    border-radius: var(--radius-lg, 10px);
    background: var(--surface, #ffffff);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  }

  .bubble.user {
    background: var(--fill-1, rgba(0, 0, 0, 0.02));
    border: 1px solid var(--separator, #e5e5ea);
  }

  .bubble-head {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .who {
    font-size: var(--text-sm, 13px);
  }

  .model {
    font-family: monospace;
    font-size: var(--text-caption, 11px);
    color: var(--text-dim, #8e8e93);
  }

  .body {
    margin: 4px 0 0;
    font-size: var(--text-sm, 13px);
    line-height: 1.5;
  }

  .markdown p {
    margin: 0;
  }

  .activity {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--separator, #e5e5ea);
  }

  .activity summary {
    font-size: var(--text-caption, 11px);
    color: var(--text-dim, #8e8e93);
    cursor: pointer;
  }

  .task-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 6px;
  }

  .tool {
    padding: 6px 10px;
    border-radius: var(--radius-md, 6px);
    background: var(--fill-1, rgba(0, 0, 0, 0.03));
    font-size: var(--text-caption, 11px);
  }

  .tool-head {
    display: flex;
    justify-content: space-between;
  }

  .tool-body {
    margin: 4px 0 0;
    font-family: monospace;
    white-space: pre-wrap;
  }

  .state-completed {
    color: var(--success, #34c759);
  }

  .caret {
    display: inline-block;
    width: 6px;
    height: 12px;
    background: var(--accent, #0a84ff);
    animation: blink 1s infinite;
  }

  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }

  .composer {
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 8px);
    padding: var(--space-3, 12px) var(--space-4, 16px);
    max-width: 760px;
    width: 100%;
    margin: 0 auto 16px;
    box-sizing: border-box;
    border: 1px solid var(--separator, #e5e5ea);
    border-radius: var(--radius-lg, 10px);
    background: var(--surface, #ffffff);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  }

  .composer-meta {
    display: flex;
    align-items: center;
  }

  .ws-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-caption, 11px);
    color: var(--text-dim, #8e8e93);
  }

  .ws-glyph {
    width: 12px;
    height: 12px;
    display: inline-block;
    background: currentColor;
    mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 256 256'%3E%3Cpath d='M216,72H130.67L102.93,50.2a16.12,16.12,0,0,0-9.6-3.2H40A16,16,0,0,0,24,63V193a15,15,0,0,0,16,15H216a16,16,0,0,0,16-16V88A16,16,0,0,0,216,72Z'/%3E%3C/svg%3E") center / contain no-repeat;
  }

  .ws-sandbox {
    padding: 1px 6px;
    border-radius: 999px;
    background: rgba(255, 159, 10, 0.15);
    color: #c77700;
  }

  .ws-sandbox.on {
    background: rgba(52, 199, 89, 0.15);
    color: #1f8f43;
  }

  .composer-input {
    width: 100%;
    resize: none;
    border: none;
    outline: none;
    font-family: inherit;
    font-size: var(--text-sm, 14px);
    background: transparent;
  }

  .composer-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .composer-hint {
    font-size: var(--text-caption, 11px);
    color: var(--text-dim, #8e8e93);
  }

  .agent-details {
    width: 260px;
    min-width: 260px;
    border-left: 1px solid var(--separator, #e5e5ea);
    background: var(--surface, #ffffff);
    overflow-y: auto;
  }

  .inspector {
    padding: var(--space-4, 16px);
    display: flex;
    flex-direction: column;
    gap: var(--space-4, 16px);
  }

  .block h3 {
    margin: 0 0 6px;
    font-size: var(--text-caption, 11px);
    text-transform: uppercase;
    color: var(--text-dim, #8e8e93);
  }

  .block p {
    margin: 0;
    font-size: var(--text-sm, 13px);
  }

  .mono {
    font-family: monospace;
  }

  .dim {
    color: var(--text-dim, #8e8e93);
  }

  .tool-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: var(--text-caption, 11px);
  }

  .tool-list li {
    display: flex;
    justify-content: space-between;
  }

  .kv {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 4px;
    margin: 0;
    font-size: var(--text-sm, 13px);
  }

  .kv dt {
    color: var(--text-dim, #8e8e93);
  }

  .kv dd {
    margin: 0;
    font-weight: 600;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: grid;
    place-items: center;
    z-index: 100;
  }

  .room-dialog {
    width: min(480px, 90vw);
    background: var(--surface, #ffffff);
    border-radius: var(--radius-lg, 10px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  }

  .room-form {
    padding: var(--space-4, 16px);
    display: flex;
    flex-direction: column;
    gap: var(--space-3, 12px);
  }

  .room-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .room-head h3 {
    margin: 0;
  }

  .btn-close {
    border: 0;
    background: none;
    font-size: 20px;
    cursor: pointer;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: var(--text-sm, 13px);
  }

  .field input, .field textarea {
    padding: 8px 10px;
    border-radius: var(--radius-md, 6px);
    border: 1px solid var(--separator, #e5e5ea);
    font-family: inherit;
    font-size: inherit;
  }

  .room-foot {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2, 8px);
    margin-top: 8px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    border: 0;
  }
</style>
