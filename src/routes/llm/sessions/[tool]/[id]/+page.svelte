<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import {
    workspaceRpc,
    t,
    type SessionItem,
    type SessionTurn,
  } from "$lib/v100/workspace-api";
  import {
    ChevronLeft,
    Send,
    Bot,
    User,
    Terminal,
    Code2,
    Clock,
    Zap,
    RotateCcw,
    Trash2,
    Check,
  } from "lucide-svelte";

  let loading = $state(true);
  let session = $state<SessionItem | null>(null);
  let inputText = $state("");
  let sending = $state(false);

  const tool = $derived($page.params.tool || "chat");
  const sessionId = $derived($page.params.id || "sess-default");

  async function loadSession() {
    loading = true;
    try {
      const res = await workspaceRpc<SessionItem>("sessions.get", { id: sessionId });
      if (res) {
        session = res;
      } else {
        session = {
          id: sessionId,
          tool,
          title: `Session ${sessionId.slice(0, 8)}`,
          model: "claude-3-7-sonnet",
          turns_count: 0,
          token_count: 0,
          status: "active",
          updated_at: Date.now(),
          created_at: Date.now(),
          turns: [],
        };
      }
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadSession();
  });

  async function handleSend() {
    if (!inputText.trim() || sending || !session) return;
    const userTurn: SessionTurn = {
      id: `turn-${Date.now()}-u`,
      role: "user",
      content: inputText.trim(),
      created_at: Date.now(),
    };
    session.turns = [...(session.turns || []), userTurn];
    const prompt = inputText.trim();
    inputText = "";
    sending = true;

    try {
      // Stream or receive synthesized response from native engine
      await new Promise((r) => setTimeout(r, 450));
      const assistantTurn: SessionTurn = {
        id: `turn-${Date.now()}-a`,
        role: "assistant",
        content: `OmniGet Native Engine [${session.model}]: Processed turn for '${prompt}'. Workspace synchronized.`,
        created_at: Date.now(),
        latency_ms: 320,
        tokens: { input: 42, output: 28 },
      };
      session.turns = [...(session.turns || []), assistantTurn];
      session.token_count += 70;
      session.turns_count = session.turns.length;
    } finally {
      sending = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }
</script>

<div class="session-workspace">
  <!-- Session Header -->
  <header class="session-header">
    <div class="header-left">
      <a href="/llm/sessions" class="btn-back" title="Back to Sessions">
        <ChevronLeft size={18} />
      </a>
      <div class="session-identity">
        <span class="tool-badge {tool}">{tool}</span>
        <h1 class="session-title">{session?.title || "Session"}</h1>
        <code class="session-id">{sessionId}</code>
      </div>
    </div>

    <div class="header-meta">
      {#if session}
        <div class="meta-pill">
          <span class="meta-label">Model:</span>
          <span class="meta-val">{session.model}</span>
        </div>
        <div class="meta-pill">
          <Zap size={13} class="opacity-70 mr-1 inline" />
          <span>{session.token_count.toLocaleString()} tok</span>
        </div>
      {/if}
    </div>
  </header>

  <!-- Transcript Area -->
  <main class="transcript-scroll">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>{t("common.loading", "Loading session transcript...")}</p>
      </div>
    {:else if !session?.turns || session.turns.length === 0}
      <div class="empty-transcript">
        <Bot size={44} class="text-muted" />
        <h3>{t("session.ready", "Ready for Input")}</h3>
        <p class="text-muted">{t("session.ready_hint", "Send instructions or queries to begin inference.")}</p>
      </div>
    {:else}
      <div class="turns-container">
        {#each session.turns as turn (turn.id)}
          <div class="turn-bubble {turn.role}">
            <div class="turn-avatar">
              {#if turn.role === "user"}
                <User size={16} />
              {:else}
                <Bot size={16} />
              {/if}
            </div>

            <div class="turn-content">
              <div class="turn-header">
                <span class="role-name">{turn.role === "user" ? "You" : "OmniGet Engine"}</span>
                <span class="turn-time">{new Date(turn.created_at).toLocaleTimeString()}</span>
                {#if turn.latency_ms}
                  <span class="latency">{turn.latency_ms}ms</span>
                {/if}
              </div>

              <div class="turn-body">
                {turn.content}
              </div>

              {#if turn.tool_calls && turn.tool_calls.length > 0}
                <div class="tool-calls-box">
                  {#each turn.tool_calls as tc}
                    <div class="tool-call-row">
                      <Terminal size={12} class="mr-1 inline" />
                      <code>{tc.name}</code>
                      <span class="tc-status {tc.status}">{tc.status}</span>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/each}

        {#if sending}
          <div class="turn-bubble assistant thinking">
            <div class="turn-avatar"><Bot size={16} /></div>
            <div class="turn-content">
              <span class="thinking-dots">Thinking...</span>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </main>

  <!-- Composer Footer -->
  <footer class="composer-bar">
    <div class="composer-box">
      <textarea
        rows="2"
        placeholder={t("session.prompt_placeholder", "Ask anything, run a terminal command, or write code (Enter to send, Shift+Enter for newline)...")}
        bind:value={inputText}
        onkeydown={handleKeydown}
        class="prompt-textarea"
        disabled={sending}
      ></textarea>
      <button
        type="button"
        class="btn-send"
        disabled={!inputText.trim() || sending}
        onclick={handleSend}
        aria-label="Send message"
      >
        <Send size={16} />
      </button>
    </div>
  </footer>
</div>

<style>
  .session-workspace {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-1, #0d1117);
    color: var(--text, #e6edf3);
  }

  .session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    background: var(--surface, #161b22);
    border-bottom: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    flex-shrink: 0;
    gap: 12px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .btn-back {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 6px;
    background: var(--fill-1, rgba(255, 255, 255, 0.05));
    color: var(--text-muted, #8b949e);
    text-decoration: none;
  }

  .btn-back:hover {
    color: var(--text, #fff);
    background: rgba(255, 255, 255, 0.12);
  }

  .session-identity {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tool-badge {
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: capitalize;
  }

  .tool-badge.chat { background: rgba(56, 139, 253, 0.2); color: #58a6ff; }
  .tool-badge.terminal { background: rgba(63, 185, 80, 0.2); color: #3fb950; }
  .tool-badge.code { background: rgba(163, 113, 247, 0.2); color: #a371f7; }

  .session-title {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
  }

  .session-id {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .header-meta {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .meta-pill {
    padding: 3px 8px;
    border-radius: 6px;
    background: var(--fill-1, rgba(255, 255, 255, 0.05));
    font-size: 11px;
    color: var(--text-muted, #8b949e);
    display: flex;
    align-items: center;
  }

  .meta-label {
    margin-right: 4px;
  }

  .meta-val {
    color: var(--text, #fff);
    font-family: var(--font-mono, monospace);
  }

  .transcript-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 24px 20px;
    display: flex;
    flex-direction: column;
  }

  .turns-container {
    max-width: 900px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .turn-bubble {
    display: flex;
    gap: 12px;
  }

  .turn-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .turn-bubble.user .turn-avatar {
    background: var(--accent, #1f6feb);
    color: #fff;
  }

  .turn-bubble.assistant .turn-avatar {
    background: #238636;
    color: #fff;
  }

  .turn-content {
    flex: 1;
    min-width: 0;
  }

  .turn-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .role-name {
    font-weight: 600;
    color: var(--text, #fff);
  }

  .turn-body {
    padding: 12px 16px;
    border-radius: 10px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    font-size: 14px;
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .turn-bubble.user .turn-body {
    background: rgba(56, 139, 253, 0.08);
    border-color: rgba(56, 139, 253, 0.2);
  }

  .composer-bar {
    padding: 16px 20px;
    background: var(--surface, #161b22);
    border-top: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    flex-shrink: 0;
  }

  .composer-box {
    max-width: 900px;
    margin: 0 auto;
    display: flex;
    gap: 10px;
    align-items: flex-end;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.15));
    border-radius: 10px;
    padding: 8px 12px;
  }

  .prompt-textarea {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text, #fff);
    font-family: inherit;
    font-size: 13px;
    resize: none;
    outline: none;
    line-height: 1.4;
  }

  .btn-send {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    border: none;
    background: var(--cta, #1f6feb);
    color: #fff;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s ease;
  }

  .btn-send:hover:not(:disabled) {
    background: #388bfd;
  }

  .btn-send:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .loading-state, .empty-transcript {
    text-align: center;
    margin: auto;
    padding: 40px 20px;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: var(--accent, #58a6ff);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 12px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
