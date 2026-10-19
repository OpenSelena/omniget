<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspaceRpc,
    t,
    type ThreadItem,
    type ThreadTurn,
  } from "$lib/engine/workspace-api";
  import {
    GitBranch,
    ShieldAlert,
    Check,
    X,
    Terminal,
    Bot,
    User,
    FileDiff,
    Clock,
    Play,
    Pause,
    Plus,
  } from "lucide-svelte";

  let loading = $state(true);
  let threads = $state<ThreadItem[]>([]);
  let selectedThread = $state<ThreadItem | null>(null);

  async function loadThreads() {
    loading = true;
    try {
      const res = await workspaceRpc<ThreadItem[]>("threads.list");
      threads = Array.isArray(res) ? res : [];
      if (threads.length > 0 && !selectedThread) {
        selectedThread = threads[0];
      }
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadThreads();
  });

  function approveItem(approvalId: string) {
    if (!selectedThread) return;
    selectedThread.pendingApprovals = selectedThread.pendingApprovals.filter((a) => a.id !== approvalId);
    selectedThread.pendingApprovalCount = selectedThread.pendingApprovals.length;
  }

  function rejectItem(approvalId: string) {
    if (!selectedThread) return;
    selectedThread.pendingApprovals = selectedThread.pendingApprovals.filter((a) => a.id !== approvalId);
    selectedThread.pendingApprovalCount = selectedThread.pendingApprovals.length;
  }
</script>

<div class="threads-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("threads.title", "Autonomous Threads & Worktrees")}</h1>
      <p class="page-subtitle">
        {t("threads.subtitle", "Track multi-turn agent execution loops, isolated disk worktrees, diff patches, and pending approvals.")}
      </p>
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{t("common.loading", "Loading active threads...")}</p>
    </div>
  {:else if threads.length === 0}
    <div class="empty-state">
      <GitBranch size={44} class="text-muted" />
      <h3>{t("threads.none", "No active agent threads")}</h3>
      <p class="text-muted">Launch a task or routine to start an autonomous thread loop.</p>
    </div>
  {:else}
    <div class="threads-layout">
      <!-- Threads List Sidebar -->
      <aside class="threads-list">
        {#each threads as th (th.threadId)}
          <button
            type="button"
            class="thread-card"
            class:selected={selectedThread?.threadId === th.threadId}
            onclick={() => (selectedThread = th)}
          >
            <div class="th-top">
              <span class="mode-pill {th.runtimeMode}">{th.runtimeMode}</span>
              {#if th.pendingApprovalCount > 0}
                <span class="approval-badge">
                  <ShieldAlert size={11} class="mr-1 inline" />
                  {th.pendingApprovalCount} approval
                </span>
              {/if}
            </div>
            <h3 class="th-title">{th.title}</h3>
            <div class="th-meta">
              <span>{th.turns.length} turns</span>
              <span>{th.model}</span>
            </div>
          </button>
        {/each}
      </aside>

      <!-- Active Thread Inspector -->
      {#if selectedThread}
        <main class="thread-inspector">
          <!-- Header Banner -->
          <div class="th-banner">
            <div>
              <div class="banner-top">
                <span class="mode-pill {selectedThread.runtimeMode}">{selectedThread.runtimeMode}</span>
                <code class="worktree-path">{selectedThread.worktreePath || "Default Workspace"}</code>
              </div>
              <h2>{selectedThread.title}</h2>
            </div>
            <div class="banner-status">
              {#if selectedThread.activeTurnId}
                <span class="status-running">
                  <span class="dot"></span>
                  Running Turn #{selectedThread.activeTurnId}
                </span>
              {:else}
                <span class="status-idle">Idle / Awaiting Step</span>
              {/if}
            </div>
          </div>

          <!-- Pending Approvals Banner -->
          {#if selectedThread.pendingApprovals.length > 0}
            <div class="approvals-card">
              <h3>
                <ShieldAlert size={16} class="text-orange-400 mr-2 inline" />
                Pending Authorizations ({selectedThread.pendingApprovals.length})
              </h3>
              <div class="approvals-list">
                {#each selectedThread.pendingApprovals as app}
                  <div class="approval-row">
                    <div class="app-desc">
                      <strong>{app.action}</strong>
                      <code>{JSON.stringify(app.details)}</code>
                    </div>
                    <div class="app-actions">
                      <button
                        type="button"
                        class="btn-action approve"
                        onclick={() => approveItem(app.id)}
                      >
                        <Check size={14} class="mr-1 inline" /> Approve
                      </button>
                      <button
                        type="button"
                        class="btn-action reject"
                        onclick={() => rejectItem(app.id)}
                      >
                        <X size={14} class="mr-1 inline" /> Deny
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Turns Timeline -->
          <section class="turns-timeline">
            <h3>Execution Turns</h3>
            <div class="turns-list">
              {#each selectedThread.turns as turn (turn.ordinal)}
                <div class="turn-box {turn.role}">
                  <div class="turn-top">
                    <span class="ordinal">Turn #{turn.ordinal}</span>
                    <span class="role-badge">{turn.role}</span>
                    <span class="state-pill {turn.state}">{turn.state}</span>
                    <span class="timestamp">{new Date(turn.timestamp).toLocaleTimeString()}</span>
                  </div>

                  <p class="turn-text">{turn.text}</p>

                  {#if turn.patches && turn.patches.length > 0}
                    <div class="patches-container">
                      <div class="patches-head">
                        <FileDiff size={14} class="mr-1 inline" />
                        <span>Modified Files ({turn.patches.length})</span>
                      </div>
                      {#each turn.patches as patch}
                        <div class="patch-item">
                          <div class="patch-header">
                            <span class="patch-path">{patch.path}</span>
                            <span class="diff-stats">
                              <span class="add">+{patch.additions}</span>
                              <span class="del">-{patch.deletions}</span>
                            </span>
                          </div>
                          <pre class="patch-diff"><code>{patch.patch}</code></pre>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </section>
        </main>
      {/if}
    </div>
  {/if}
</div>

<style>
  .threads-page {
    padding: 24px 32px;
    max-width: 1400px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .page-title {
    font-size: 22px;
    font-weight: 700;
    margin: 0 0 4px;
  }

  .page-subtitle {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 0;
  }

  .threads-layout {
    display: grid;
    grid-template-columns: 340px 1fr;
    gap: 20px;
    min-height: 600px;
  }

  .threads-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .thread-card {
    text-align: left;
    padding: 16px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
    color: var(--text, #fff);
  }

  .thread-card:hover {
    border-color: rgba(255, 255, 255, 0.25);
  }

  .thread-card.selected {
    border-color: var(--accent, #58a6ff);
    background: rgba(56, 139, 253, 0.05);
  }

  .th-top {
    display: flex;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .mode-pill {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    background: rgba(56, 139, 253, 0.2);
    color: #58a6ff;
  }

  .approval-badge {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    background: rgba(240, 136, 62, 0.2);
    color: #f0883e;
  }

  .th-title {
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 8px;
  }

  .th-meta {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .thread-inspector {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .th-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
  }

  .banner-top {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }

  .worktree-path {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .th-banner h2 {
    font-size: 18px;
    font-weight: 700;
    margin: 0;
  }

  .status-running {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 999px;
    background: rgba(63, 185, 80, 0.18);
    color: #3fb950;
    font-size: 12px;
    font-weight: 600;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .status-idle {
    font-size: 12px;
    color: var(--text-muted, #8b949e);
  }

  .approvals-card {
    padding: 16px 20px;
    background: rgba(240, 136, 62, 0.08);
    border: 1px solid rgba(240, 136, 62, 0.3);
    border-radius: 12px;
  }

  .approvals-card h3 {
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 12px;
    color: #f0883e;
  }

  .approvals-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .approval-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    background: var(--surface, #161b22);
    border-radius: 8px;
  }

  .app-desc strong {
    display: block;
    font-size: 13px;
  }

  .app-desc code {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .app-actions {
    display: flex;
    gap: 8px;
  }

  .btn-action {
    display: inline-flex;
    align-items: center;
    padding: 6px 12px;
    border-radius: 6px;
    border: none;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }

  .btn-action.approve {
    background: #238636;
    color: #fff;
  }

  .btn-action.reject {
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    color: #f85149;
  }

  .turns-timeline {
    padding: 20px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
  }

  .turns-timeline h3 {
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 16px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted, #8b949e);
  }

  .turns-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .turn-box {
    padding: 16px;
    background: var(--surface-mut, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    border-radius: 8px;
  }

  .turn-top {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .ordinal {
    font-weight: 700;
    color: var(--text, #fff);
  }

  .role-badge {
    text-transform: capitalize;
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
  }

  .state-pill.completed { color: #3fb950; }
  .state-pill.running { color: #58a6ff; }
  .state-pill.error { color: #f85149; }

  .turn-text {
    font-size: 13px;
    line-height: 1.5;
    margin: 0 0 12px;
  }

  .patches-container {
    padding: 12px;
    background: var(--surface, #161b22);
    border-radius: 6px;
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
  }

  .patches-head {
    font-size: 12px;
    font-weight: 600;
    margin-bottom: 8px;
    color: var(--accent-hi, #58a6ff);
  }

  .patch-item {
    margin-bottom: 8px;
  }

  .patch-header {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    margin-bottom: 4px;
  }

  .patch-path {
    font-family: var(--font-mono, monospace);
  }

  .diff-stats {
    font-family: var(--font-mono, monospace);
  }

  .add { color: #3fb950; margin-right: 4px; }
  .del { color: #f85149; }

  .patch-diff {
    background: #000;
    padding: 8px 12px;
    border-radius: 4px;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    overflow-x: auto;
    margin: 0;
    color: #3fb950;
  }

  .loading-state, .empty-state {
    text-align: center;
    padding: 60px 20px;
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

  @media (max-width: 900px) {
    .threads-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
