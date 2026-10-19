<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspaceRpc,
    t,
    type MissionItem,
  } from "$lib/v100/workspace-api";
  import {
    Compass,
    Plus,
    CheckCircle2,
    Clock,
    AlertCircle,
    Play,
    Pause,
    Bot,
    ChevronRight,
    Terminal,
  } from "lucide-svelte";

  let loading = $state(true);
  let missions = $state<MissionItem[]>([]);
  let selectedMission = $state<MissionItem | null>(null);
  let showCreateModal = $state(false);

  // New Mission inputs
  let newTitle = $state("");
  let newObjective = $state("");
  let newPriority = $state<"low" | "medium" | "high" | "critical">("high");

  async function loadMissions() {
    loading = true;
    try {
      const res = await workspaceRpc<MissionItem[]>("missions.list");
      missions = Array.isArray(res) ? res : [];
      if (missions.length > 0 && !selectedMission) {
        selectedMission = missions[0];
      }
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadMissions();
  });

  async function createMission() {
    if (!newTitle.trim() || !newObjective.trim()) return;
    const m = await workspaceRpc<MissionItem>("missions.create", {
      title: newTitle,
      objective: newObjective,
      priority: newPriority,
    });
    missions = [m, ...missions];
    selectedMission = m;
    showCreateModal = false;
    newTitle = "";
    newObjective = "";
  }

  function toggleTask(taskId: string) {
    if (!selectedMission) return;
    const t = selectedMission.tasks.find((x) => x.id === taskId);
    if (t) {
      t.done = !t.done;
      const doneCount = selectedMission.tasks.filter((x) => x.done).length;
      selectedMission.progress_pct = Math.round((doneCount / selectedMission.tasks.length) * 100);
    }
  }
</script>

<div class="missions-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("missions.title", "Autonomous Goal Missions")}</h1>
      <p class="page-subtitle">
        {t("missions.subtitle", "Track multi-agent decomposed campaigns, task steps, and automated milestones.")}
      </p>
    </div>
    <div class="head-actions">
      <button type="button" class="btn btn-primary" onclick={() => (showCreateModal = true)}>
        <Plus size={15} class="mr-1 inline" />
        {t("missions.new_mission", "New Mission")}
      </button>
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{t("common.loading", "Loading autonomous missions...")}</p>
    </div>
  {:else if missions.length === 0}
    <div class="empty-state">
      <Compass size={48} class="text-muted" />
      <h3>{t("missions.none", "No active missions")}</h3>
      <p class="text-muted">{t("missions.none_hint", "Create an autonomous goal mission to assign tasks to agents.")}</p>
      <button type="button" class="btn btn-primary mt-3" onclick={() => (showCreateModal = true)}>
        Launch First Mission
      </button>
    </div>
  {:else}
    <div class="missions-layout">
      <!-- Missions Sidebar List -->
      <aside class="missions-list">
        {#each missions as m (m.id)}
          <button
            type="button"
            class="mission-card"
            class:selected={selectedMission?.id === m.id}
            onclick={() => (selectedMission = m)}
          >
            <div class="card-top">
              <span class="prio-pill {m.priority}">{m.priority}</span>
              <span class="status-pill {m.status}">{m.status}</span>
            </div>
            <h3 class="m-title">{m.title}</h3>
            <p class="m-obj">{m.objective}</p>
            <div class="prog-row">
              <div class="prog-bar-bg">
                <div class="prog-bar-fill" style="width: {m.progress_pct}%"></div>
              </div>
              <span class="pct-text">{m.progress_pct}%</span>
            </div>
          </button>
        {/each}
      </aside>

      <!-- Mission Detail View -->
      {#if selectedMission}
        <main class="mission-detail">
          <div class="detail-header">
            <div class="header-main">
              <div class="badges-row">
                <span class="prio-pill {selectedMission.priority}">{selectedMission.priority} priority</span>
                <span class="status-pill {selectedMission.status}">{selectedMission.status}</span>
              </div>
              <h2>{selectedMission.title}</h2>
              <p class="detail-obj">{selectedMission.objective}</p>
            </div>
          </div>

          <!-- Progress Header Card -->
          <div class="progress-card">
            <div class="prog-stats">
              <div>
                <span class="stat-label">Progress</span>
                <strong class="stat-val">{selectedMission.progress_pct}%</strong>
              </div>
              <div>
                <span class="stat-label">Assigned Agents</span>
                <strong class="stat-val">{selectedMission.assigned_agents.join(", ")}</strong>
              </div>
              <div>
                <span class="stat-label">Tasks</span>
                <strong class="stat-val">
                  {selectedMission.tasks.filter((t) => t.done).length} / {selectedMission.tasks.length}
                </strong>
              </div>
            </div>
            <div class="prog-bar-bg large">
              <div class="prog-bar-fill" style="width: {selectedMission.progress_pct}%"></div>
            </div>
          </div>

          <!-- Tasks Checklist -->
          <section class="section-card">
            <h3>Milestones & Tasks</h3>
            <div class="task-checklist">
              {#each selectedMission.tasks as task (task.id)}
                <div class="task-row" class:done={task.done}>
                  <input
                    type="checkbox"
                    checked={task.done}
                    onchange={() => toggleTask(task.id)}
                    class="task-check"
                  />
                  <div class="task-body">
                    <span class="task-title">{task.title}</span>
                    {#if task.result}
                      <span class="task-result">&rarr; {task.result}</span>
                    {/if}
                  </div>
                  {#if task.agent}
                    <span class="agent-badge">
                      <Bot size={12} class="mr-1 inline" />
                      {task.agent}
                    </span>
                  {/if}
                </div>
              {/each}
            </div>
          </section>

          <!-- Execution Logs -->
          <section class="section-card">
            <h3>Mission Execution Timeline</h3>
            <div class="logs-container">
              {#each selectedMission.logs as log}
                <div class="log-entry {log.level}">
                  <span class="log-time">{new Date(log.ts).toLocaleTimeString()}</span>
                  <span class="log-level">{log.level}</span>
                  <span class="log-msg">{log.message}</span>
                </div>
              {/each}
            </div>
          </section>
        </main>
      {/if}
    </div>
  {/if}

  <!-- Create Modal -->
  {#if showCreateModal}
    <div class="modal-backdrop" onclick={() => (showCreateModal = false)}></div>
    <div class="modal-dialog" role="dialog" aria-modal="true">
      <h2>Launch New Mission</h2>
      <p class="modal-hint">Define high-level objective. Agents will autonomously decompose it into subtasks.</p>

      <div class="form-group">
        <label for="mission-title">Mission Title</label>
        <input
          id="mission-title"
          type="text"
          bind:value={newTitle}
          placeholder="e.g. Audit Dependency Tree for Security Vulnerabilities"
          class="form-input"
        />
      </div>

      <div class="form-group">
        <label for="mission-obj">Primary Objective Charter</label>
        <textarea
          id="mission-obj"
          rows="3"
          bind:value={newObjective}
          placeholder="Specific goal and criteria for success..."
          class="form-input"
        ></textarea>
      </div>

      <div class="form-group">
        <label for="mission-prio">Priority</label>
        <select id="mission-prio" bind:value={newPriority} class="form-input">
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
          <option value="critical">Critical</option>
        </select>
      </div>

      <div class="modal-actions">
        <button type="button" class="btn btn-secondary" onclick={() => (showCreateModal = false)}>
          Cancel
        </button>
        <button type="button" class="btn btn-primary" onclick={createMission}>
          Launch Mission
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .missions-page {
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
    gap: 16px;
    flex-wrap: wrap;
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

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn-primary {
    background: var(--cta, #1f6feb);
    color: #fff;
  }

  .btn-primary:hover {
    background: #388bfd;
  }

  .btn-secondary {
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    color: var(--text, #fff);
  }

  .missions-layout {
    display: grid;
    grid-template-columns: 360px 1fr;
    gap: 20px;
    min-height: 600px;
  }

  .missions-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .mission-card {
    text-align: left;
    padding: 16px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
    color: var(--text, #fff);
  }

  .mission-card:hover {
    border-color: rgba(255, 255, 255, 0.25);
  }

  .mission-card.selected {
    border-color: var(--accent, #58a6ff);
    background: rgba(56, 139, 253, 0.05);
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .prio-pill {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .prio-pill.critical { background: rgba(248, 81, 73, 0.2); color: #f85149; }
  .prio-pill.high { background: rgba(240, 136, 62, 0.2); color: #f0883e; }
  .prio-pill.medium { background: rgba(56, 139, 253, 0.2); color: #58a6ff; }
  .prio-pill.low { background: rgba(139, 148, 158, 0.2); color: #8b949e; }

  .status-pill {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .m-title {
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 4px;
  }

  .m-obj {
    font-size: 12px;
    color: var(--text-muted, #8b949e);
    margin: 0 0 10px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .prog-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .prog-bar-bg {
    flex: 1;
    height: 6px;
    background: var(--fill-1, rgba(255, 255, 255, 0.1));
    border-radius: 3px;
    overflow: hidden;
  }

  .prog-bar-bg.large {
    height: 8px;
  }

  .prog-bar-fill {
    height: 100%;
    background: #3fb950;
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .pct-text {
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono, monospace);
  }

  .mission-detail {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .detail-header {
    padding: 20px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
  }

  .badges-row {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }

  .detail-header h2 {
    font-size: 20px;
    font-weight: 700;
    margin: 0 0 8px;
  }

  .detail-obj {
    font-size: 14px;
    color: var(--text-muted, #8b949e);
    margin: 0;
    line-height: 1.5;
  }

  .progress-card {
    padding: 20px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .prog-stats {
    display: flex;
    justify-content: space-between;
  }

  .stat-label {
    display: block;
    font-size: 11px;
    color: var(--text-muted, #8b949e);
    margin-bottom: 4px;
  }

  .stat-val {
    font-size: 14px;
    font-weight: 600;
  }

  .section-card {
    padding: 20px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
  }

  .section-card h3 {
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 14px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted, #8b949e);
  }

  .task-checklist {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .task-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    background: var(--fill-1, rgba(255, 255, 255, 0.03));
    border-radius: 8px;
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.06));
  }

  .task-row.done {
    opacity: 0.7;
  }

  .task-row.done .task-title {
    text-decoration: line-through;
    color: var(--text-muted, #8b949e);
  }

  .task-check {
    width: 16px;
    height: 16px;
    cursor: pointer;
  }

  .task-body {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .task-title {
    font-size: 13px;
    font-weight: 500;
  }

  .task-result {
    font-size: 11px;
    color: #3fb950;
    font-family: var(--font-mono, monospace);
  }

  .agent-badge {
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 4px;
    background: var(--fill-2, rgba(255, 255, 255, 0.08));
    color: var(--text-muted, #8b949e);
  }

  .logs-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .log-entry {
    display: flex;
    gap: 10px;
    font-size: 12px;
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--surface-mut, #0d1117);
  }

  .log-time {
    color: var(--text-muted, #8b949e);
    font-family: var(--font-mono, monospace);
  }

  .log-level {
    font-weight: 600;
    text-transform: uppercase;
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 3px;
  }

  .log-entry.step .log-level { background: rgba(56, 139, 253, 0.2); color: #58a6ff; }
  .log-entry.info .log-level { background: rgba(139, 148, 158, 0.2); color: #8b949e; }

  .log-msg {
    color: var(--text, #fff);
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 50;
  }

  .modal-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(500px, 90vw);
    background: var(--surface-hi, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.15));
    border-radius: 12px;
    padding: 24px;
    z-index: 51;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-dialog h2 {
    font-size: 18px;
    font-weight: 700;
    margin: 0;
  }

  .modal-hint {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 0;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 12px;
    font-weight: 500;
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

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }

  @media (max-width: 900px) {
    .missions-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
