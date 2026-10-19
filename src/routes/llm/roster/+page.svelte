<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspaceRpc,
    t,
    type RosterAgent,
  } from "$lib/v100/workspace-api";
  import {
    Bot,
    Plus,
    Shield,
    Cpu,
    Sliders,
    Link,
    Wrench,
    CheckCircle2,
    Lock,
    Unlock,
  } from "lucide-svelte";

  let loading = $state(true);
  let agents = $state<RosterAgent[]>([]);
  let selectedAgent = $state<RosterAgent | null>(null);

  async function loadRoster() {
    loading = true;
    try {
      const res = await workspaceRpc<RosterAgent[]>("roster.list");
      agents = Array.isArray(res) ? res : [];
      if (agents.length > 0 && !selectedAgent) {
        selectedAgent = agents[0];
      }
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadRoster();
  });
</script>

<div class="roster-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("roster.title", "Autonomous Agent Roster")}</h1>
      <p class="page-subtitle">
        {t("roster.subtitle", "Configure agent personas, model fallback chains, MCP tool bindings, and security policies.")}
      </p>
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{t("common.loading", "Loading agent squad roster...")}</p>
    </div>
  {:else if agents.length === 0}
    <div class="empty-state">
      <Bot size={48} class="text-muted" />
      <h3>{t("roster.none", "No agents defined in squad")}</h3>
    </div>
  {:else}
    <div class="roster-layout">
      <!-- Agent Squad Column -->
      <aside class="agents-sidebar">
        {#each agents as agent (agent.id)}
          <button
            type="button"
            class="agent-card"
            class:active={selectedAgent?.id === agent.id}
            onclick={() => (selectedAgent = agent)}
          >
            <div class="avatar" style="background: {agent.avatar_color}">
              <Bot size={18} />
            </div>
            <div class="agent-summary">
              <span class="agent-name">{agent.name}</span>
              <span class="agent-role">{agent.role}</span>
              <span class="model-pill">{agent.primary_model}</span>
            </div>
          </button>
        {/each}
      </aside>

      <!-- Agent Details & Policy Config -->
      {#if selectedAgent}
        <main class="agent-details">
          <div class="panel persona-panel">
            <div class="panel-head">
              <Bot size={18} class="text-blue-400" />
              <h2>Agent Persona & System Directives</h2>
            </div>
            <div class="form-row">
              <div class="form-group">
                <label for="agent-name-input">Display Name</label>
                <input
                  id="agent-name-input"
                  type="text"
                  bind:value={selectedAgent.name}
                  class="form-input"
                />
              </div>
              <div class="form-group">
                <label for="agent-role-input">Role & Domain</label>
                <input
                  id="agent-role-input"
                  type="text"
                  bind:value={selectedAgent.role}
                  class="form-input"
                />
              </div>
            </div>
            <div class="form-group full-width">
              <label for="agent-prompt-input">System Instructions (Prompt)</label>
              <textarea
                id="agent-prompt-input"
                rows="4"
                bind:value={selectedAgent.system_prompt}
                class="form-input mono"
              ></textarea>
            </div>
          </div>

          <!-- Model Fallback Chain -->
          <div class="panel chain-panel">
            <div class="panel-head">
              <Cpu size={18} class="text-green-400" />
              <h2>Inference Model & Fallback Chain</h2>
            </div>
            <p class="panel-hint">
              If the primary model is rate-limited or unavailable, inference seamlessly steps down the candidate chain.
            </p>
            <div class="chain-box">
              <div class="chain-item primary">
                <span class="badge primary">Primary</span>
                <span class="model-name">{selectedAgent.primary_model}</span>
              </div>
              {#each selectedAgent.fallback_chain as fb, i}
                <div class="chain-arrow">&darr;</div>
                <div class="chain-item fallback">
                  <span class="badge fallback">Fallback {i + 1}</span>
                  <span class="model-name">{fb}</span>
                </div>
              {/each}
            </div>
          </div>

          <!-- Policy & Guardrails -->
          <div class="panel policy-panel">
            <div class="panel-head">
              <Shield size={18} class="text-orange-400" />
              <h2>Execution Guardrails & Security Policy</h2>
            </div>
            <div class="policy-grid">
              <div class="policy-card">
                <div class="policy-icon">
                  {#if selectedAgent.policy.require_approval_on_write}
                    <Lock size={16} class="text-orange-400" />
                  {:else}
                    <Unlock size={16} class="text-green-400" />
                  {/if}
                </div>
                <div class="policy-desc">
                  <strong>Require Human Approval on Disk Writes</strong>
                  <span class="hint">Blocks automatic filesystem modification until explicitly confirmed.</span>
                </div>
                <input
                  type="checkbox"
                  bind:checked={selectedAgent.policy.require_approval_on_write}
                  class="policy-toggle"
                />
              </div>

              <div class="policy-card">
                <div class="policy-desc">
                  <strong>Turn Token Cap</strong>
                  <span class="hint">Maximum tokens per reasoning turn.</span>
                </div>
                <span class="policy-val">{selectedAgent.policy.max_tokens_per_turn.toLocaleString()} tok</span>
              </div>

              <div class="policy-card">
                <div class="policy-desc">
                  <strong>Monthly Spend Guardrail</strong>
                  <span class="hint">Auto-pause if cost exceeds budget limit.</span>
                </div>
                <span class="policy-val text-green">${selectedAgent.policy.budget_limit_usd.toFixed(2)}</span>
              </div>
            </div>
          </div>

          <!-- Active MCP Tools -->
          <div class="panel tools-panel">
            <div class="panel-head">
              <Wrench size={18} class="text-purple-400" />
              <h2>Allowed Tool Capabilities & MCP Servers</h2>
            </div>
            <div class="chips-row">
              {#each selectedAgent.allowed_tools as tool}
                <span class="tool-chip">
                  <Wrench size={11} class="mr-1 inline" />
                  <code>{tool}</code>
                </span>
              {/each}
            </div>
          </div>
        </main>
      {/if}
    </div>
  {/if}
</div>

<style>
  .roster-page {
    padding: 24px 32px;
    max-width: 1400px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
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

  .roster-layout {
    display: grid;
    grid-template-columns: 320px 1fr;
    gap: 20px;
  }

  .agents-sidebar {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .agent-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
    color: var(--text, #fff);
  }

  .agent-card:hover {
    border-color: rgba(255, 255, 255, 0.25);
  }

  .agent-card.active {
    border-color: var(--accent, #58a6ff);
    background: rgba(56, 139, 253, 0.06);
  }

  .avatar {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    flex-shrink: 0;
  }

  .agent-summary {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .agent-name {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .agent-role {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 4px;
  }

  .model-pill {
    align-self: flex-start;
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    font-size: 10px;
    font-family: var(--font-mono, monospace);
    color: #58a6ff;
  }

  .agent-details {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .panel {
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    padding: 20px;
  }

  .panel-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }

  .panel-head h2 {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
  }

  .panel-hint {
    font-size: 12px;
    color: var(--text-muted, #8b949e);
    margin: 0 0 16px;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-bottom: 12px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 12px;
    color: var(--text-muted, #8b949e);
  }

  .form-input {
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 13px;
    outline: none;
  }

  .form-input.mono {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    line-height: 1.5;
  }

  .chain-box {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 400px;
  }

  .chain-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: 8px;
    background: var(--surface-mut, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
  }

  .chain-item.primary {
    border-color: rgba(63, 185, 80, 0.3);
  }

  .badge {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .badge.primary { background: rgba(63, 185, 80, 0.2); color: #3fb950; }
  .badge.fallback { background: rgba(139, 148, 158, 0.2); color: #8b949e; }

  .model-name {
    font-family: var(--font-mono, monospace);
    font-size: 13px;
    color: var(--text, #fff);
  }

  .chain-arrow {
    text-align: center;
    color: var(--text-muted, #8b949e);
    font-size: 12px;
  }

  .policy-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .policy-card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
    background: var(--fill-1, rgba(255, 255, 255, 0.03));
    border-radius: 8px;
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.06));
  }

  .policy-desc {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .policy-desc strong {
    font-size: 13px;
  }

  .policy-desc .hint {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .policy-val {
    font-size: 13px;
    font-weight: 600;
  }

  .text-green { color: #3fb950; }

  .chips-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tool-chip {
    padding: 4px 10px;
    border-radius: 6px;
    background: var(--fill-1, rgba(255, 255, 255, 0.06));
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    font-size: 12px;
  }

  .tool-chip code {
    font-family: var(--font-mono, monospace);
    color: #e6edf3;
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
    .roster-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
