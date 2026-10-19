<script lang="ts">
  import { page } from "$app/stores";
  import { onMount } from "svelte";
  import {
    companyRpc,
    t,
    avatarColor,
    getInitials,
    formatRelativeTime,
    statusColor,
    CRON_PRESETS,
    type Routine,
    type Staff,
  } from "$lib/v100/company-api";
  import {
    Repeat,
    Play,
    Plus,
    Clock,
    Search,
    AlertCircle,
    CheckCircle2,
    Trash2,
    Edit3,
    Terminal,
    RefreshCw,
    X,
    Check,
    Calendar,
    ChevronDown,
  } from "lucide-svelte";

  const companyId = $derived($page.params.id || "omniget-core");

  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state<string | null>(null);

  let routines = $state<Routine[]>([]);
  let staffList = $state<Staff[]>([]);
  let searchQuery = $state("");

  // Triggering state
  let triggeringId = $state<string | null>(null);
  let triggerNotice = $state<string | null>(null);

  // New Routine Modal
  let showModal = $state(false);
  let editingId = $state<string | null>(null);
  let saving = $state(false);
  let formError = $state<string | null>(null);

  let formName = $state("");
  let formDesc = $state("");
  let formCron = $state("0 9 * * *");
  let formPreset = $state("daily");
  let formAgent = $state("");
  let formPrompt = $state("");
  let formEnabled = $state(true);

  async function loadData(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    error = null;
    try {
      const [resRoutines, resStaff] = await Promise.all([
        companyRpc<Routine[]>("routine.list", { company: companyId }),
        companyRpc<Staff[]>("staff.list", { company: companyId }),
      ]);
      routines = Array.isArray(resRoutines) ? resRoutines : [];
      staffList = Array.isArray(resStaff) ? resStaff : [];
    } catch (err: any) {
      error = err?.message || "Failed to load scheduled routines";
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadData();
  });

  const staffMap = $derived(new Map(staffList.map((s) => [s.id, s])));

  const filteredRoutines = $derived(
    routines.filter((r) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        r.name.toLowerCase().includes(q) ||
        (r.description ?? "").toLowerCase().includes(q) ||
        r.cron.toLowerCase().includes(q)
      );
    })
  );

  async function toggleRoutine(routine: Routine) {
    const prev = routine.enabled;
    routine.enabled = !prev;
    try {
      await companyRpc("routine.update", {
        company: companyId,
        id: routine.id,
        patch: { enabled: routine.enabled },
      });
    } catch {
      routine.enabled = prev;
    }
  }

  async function handleTriggerNow(routine: Routine) {
    triggeringId = routine.id;
    triggerNotice = null;
    try {
      await companyRpc("routine.trigger", {
        company: companyId,
        routine_id: routine.id,
        staff_id: routine.agent_id,
      });
      triggerNotice = t("company.routines.triggered_success", `Run dispatched for "${routine.name}".`);
      routine.last_run_at = Date.now();
      routine.last_run_status = "running";
      setTimeout(() => {
        triggerNotice = null;
      }, 4000);
    } catch (err: any) {
      alert("Trigger failed: " + (err?.message || err));
    } finally {
      triggeringId = null;
    }
  }

  function openCreateModal() {
    editingId = null;
    formName = "";
    formDesc = "";
    formCron = "0 9 * * *";
    formPreset = "daily";
    formAgent = staffList[0]?.id || "";
    formPrompt = "";
    formEnabled = true;
    formError = null;
    showModal = true;
  }

  function openEditModal(routine: Routine) {
    editingId = routine.id;
    formName = routine.name;
    formDesc = routine.description ?? "";
    formCron = routine.cron;
    const match = CRON_PRESETS.find((p) => p.cron === routine.cron);
    formPreset = match ? match.id : "custom";
    formAgent = routine.agent_id ?? "";
    formPrompt = routine.prompt ?? "";
    formEnabled = routine.enabled;
    formError = null;
    showModal = true;
  }

  function handlePresetChange(presetId: string) {
    formPreset = presetId;
    const match = CRON_PRESETS.find((p) => p.id === presetId);
    if (match) {
      formCron = match.cron;
    }
  }

  async function handleSaveRoutine(e: SubmitEvent) {
    e.preventDefault();
    if (!formName.trim()) {
      formError = "Routine name is required";
      return;
    }
    saving = true;
    formError = null;

    try {
      if (editingId) {
        const updated = await companyRpc<Routine>("routine.update", {
          company: companyId,
          id: editingId,
          patch: {
            name: formName.trim(),
            description: formDesc.trim(),
            cron: formCron.trim(),
            agent_id: formAgent || null,
            prompt: formPrompt.trim(),
            enabled: formEnabled,
          },
        });
        const idx = routines.findIndex((r) => r.id === editingId);
        if (idx >= 0 && updated) routines[idx] = updated;
      } else {
        const created = await companyRpc<Routine>("routine.create", {
          company: companyId,
          routine: {
            name: formName.trim(),
            description: formDesc.trim(),
            cron: formCron.trim(),
            agent_id: formAgent || null,
            prompt: formPrompt.trim(),
            enabled: formEnabled,
          },
        });
        if (created) routines.unshift(created);
      }
      showModal = false;
    } catch (err: any) {
      formError = err?.message || "Failed to save routine";
    } finally {
      saving = false;
    }
  }

  async function handleDelete(routine: Routine) {
    if (!confirm(`Are you sure you want to delete routine "${routine.name}"?`)) return;
    try {
      await companyRpc("routine.delete", { company: companyId, id: routine.id });
      routines = routines.filter((r) => r.id !== routine.id);
    } catch (err: any) {
      alert("Failed to delete routine: " + (err?.message || err));
    }
  }
</script>

<svelte:head>
  <title>{t("company.routines.title", "Automated Routines")} - OmniGet</title>
</svelte:head>

<div class="routines-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Repeat size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("company.routines.header", "Scheduled Routines")}</h1>
          <span class="count-pill">{routines.length}</span>
        </div>
        <p class="subtitle">
          {t("company.routines.sub", "Configure autonomous cron loops, maintenance workers, and recurring agent workflows.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadData(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>

      <button class="btn btn-primary btn-sm" onclick={openCreateModal}>
        <Plus size={15} />
        <span>{t("company.routines.new", "New Routine")}</span>
      </button>
    </div>
  </header>

  {#if triggerNotice}
    <div class="notice-bar">
      <CheckCircle2 size={16} />
      <span>{triggerNotice}</span>
    </div>
  {/if}

  <!-- Controls Bar -->
  <div class="controls-bar">
    <div class="search-wrap">
      <Search size={14} class="search-icon" />
      <input
        type="search"
        placeholder={t("company.routines.search_placeholder", "Filter routines by name, cron...")}
        bind:value={searchQuery}
      />
    </div>

    <div class="stats-summary">
      <span>{t("company.routines.active_count", "Active:")} <strong>{routines.filter((r) => r.enabled).length}</strong></span>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading scheduled routines...")}</p>
    </div>
  {:else if error}
    <div class="error-card">
      <AlertCircle size={20} class="text-danger" />
      <div>
        <h4>{t("company.routines.failed", "Failed to Load Routines")}</h4>
        <p>{error}</p>
      </div>
      <button class="btn btn-secondary btn-sm" onclick={() => loadData()}>
        {t("common.retry", "Retry")}
      </button>
    </div>
  {:else}
    <div class="routines-grid">
      {#each filteredRoutines as routine (routine.id)}
        {@const agent = routine.agent_id ? staffMap.get(routine.agent_id) : null}
        <div class="routine-card" class:disabled={!routine.enabled}>
          <div class="card-head">
            <div class="head-left">
              <span class="cron-pill">
                <Calendar size={12} />
                <code>{routine.cron}</code>
              </span>
              {#if routine.last_run_status}
                <span class="status-pill status-{statusColor(routine.last_run_status)}">
                  {routine.last_run_status}
                </span>
              {/if}
            </div>

            <div class="head-right">
              <!-- Enabled Toggle -->
              <label class="switch-wrap" title={routine.enabled ? "Disable routine" : "Enable routine"}>
                <input
                  type="checkbox"
                  checked={routine.enabled}
                  onchange={() => toggleRoutine(routine)}
                />
                <span class="slider"></span>
              </label>
            </div>
          </div>

          <div class="card-body">
            <h3 class="routine-name">{routine.name}</h3>
            {#if routine.description}
              <p class="routine-desc">{routine.description}</p>
            {/if}

            {#if routine.prompt}
              <div class="prompt-preview">
                <Terminal size={12} class="prompt-icon" />
                <span>{routine.prompt}</span>
              </div>
            {/if}
          </div>

          <div class="card-meta">
            <div class="agent-info">
              {#if agent}
                <span
                  class="avatar-circle-sm"
                  style="background: {avatarColor(agent.name)};"
                >
                  {getInitials(agent.name)}
                </span>
                <span class="agent-name">{agent.name}</span>
              {:else}
                <span class="text-muted text-sm">{t("company.routines.unassigned", "No agent assigned")}</span>
              {/if}
            </div>

            <div class="last-run-info">
              <Clock size={12} />
              <span>
                {routine.last_run_at
                  ? formatRelativeTime(routine.last_run_at, t)
                  : t("company.common.never", "Never run")}
              </span>
            </div>
          </div>

          <div class="card-footer">
            <div class="action-btns">
              <button
                class="btn btn-secondary btn-sm"
                onclick={() => openEditModal(routine)}
                title="Edit routine"
              >
                <Edit3 size={13} />
                <span>{t("common.edit", "Edit")}</span>
              </button>
              <button
                class="btn btn-danger-outline btn-sm icon-only"
                onclick={() => handleDelete(routine)}
                title="Delete routine"
              >
                <Trash2 size={13} />
              </button>
            </div>

            <button
              class="btn btn-primary btn-sm trigger-btn"
              disabled={triggeringId === routine.id || !routine.enabled}
              onclick={() => handleTriggerNow(routine)}
            >
              <Play size={13} class={triggeringId === routine.id ? "spin" : ""} />
              <span>{triggeringId === routine.id ? t("common.triggering", "Triggering...") : t("company.routines.run_now", "Run Now")}</span>
            </button>
          </div>
        </div>
      {/each}

      {#if filteredRoutines.length === 0}
        <div class="empty-state">
          <Repeat size={36} class="text-muted" />
          <h4>{t("company.routines.no_routines", "No routines found")}</h4>
          <p>{t("company.routines.empty_sub", "Set up autonomous routines to run checks, fetch feeds, or clean disk caches on a schedule.")}</p>
          <button class="btn btn-primary btn-sm" onclick={openCreateModal}>
            <Plus size={14} />
            <span>{t("company.routines.new", "Create Routine")}</span>
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Routine Create / Edit Modal -->
  {#if showModal}
    <div class="modal-backdrop" onclick={() => (showModal = false)}>
      <div class="modal-box" onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3>{editingId ? t("company.routines.edit_title", "Edit Routine") : t("company.routines.new_title", "Create Routine")}</h3>
          <button class="icon-btn" onclick={() => (showModal = false)}>
            <X size={18} />
          </button>
        </div>

        {#if formError}
          <div class="form-error">
            <AlertCircle size={15} />
            <span>{formError}</span>
          </div>
        {/if}

        <form onsubmit={handleSaveRoutine} class="modal-form">
          <div class="form-group">
            <label for="rot-name">{t("company.routines.form_name", "Routine Name *")}</label>
            <input
              id="rot-name"
              type="text"
              placeholder="e.g. Periodic Temp Clean & Queue Check"
              bind:value={formName}
              required
            />
          </div>

          <div class="form-group">
            <label for="rot-desc">{t("company.routines.form_desc", "Description")}</label>
            <input
              id="rot-desc"
              type="text"
              placeholder="e.g. Cleans residual chunks and checks media queues"
              bind:value={formDesc}
            />
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="rot-preset">{t("company.routines.form_schedule_preset", "Schedule Preset")}</label>
              <select
                id="rot-preset"
                bind:value={formPreset}
                onchange={(e) => handlePresetChange(e.currentTarget.value)}
              >
                <option value="every_15">Every 15 Minutes</option>
                <option value="hourly">Hourly</option>
                <option value="daily">Daily at 9:00 AM</option>
                <option value="weekdays">Weekdays at 9:00 AM</option>
                <option value="weekly">Weekly (Monday 9:00 AM)</option>
                <option value="monthly">Monthly (1st of month)</option>
                <option value="custom">Custom Cron Expression</option>
              </select>
            </div>

            <div class="form-group">
              <label for="rot-cron">{t("company.routines.form_cron", "Cron Expression")}</label>
              <input
                id="rot-cron"
                type="text"
                placeholder="* * * * *"
                bind:value={formCron}
                required
              />
            </div>
          </div>

          <div class="form-group">
            <label for="rot-agent">{t("company.routines.form_agent", "Assigned Staff Agent")}</label>
            <select id="rot-agent" bind:value={formAgent}>
              <option value="">{t("company.routines.unassigned", "Unassigned")}</option>
              {#each staffList as s}
                <option value={s.id}>{s.name} ({s.role})</option>
              {/each}
            </select>
          </div>

          <div class="form-group">
            <label for="rot-prompt">{t("company.routines.form_prompt", "Execution Prompt / Instructions")}</label>
            <textarea
              id="rot-prompt"
              rows="4"
              placeholder="Enter instructions for the agent when this routine is triggered..."
              bind:value={formPrompt}
            ></textarea>
          </div>

          <div class="modal-footer">
            <button
              type="button"
              class="btn btn-secondary"
              disabled={saving}
              onclick={() => (showModal = false)}
            >
              {t("common.cancel", "Cancel")}
            </button>
            <button type="submit" class="btn btn-primary" disabled={saving}>
              {saving ? t("common.saving", "Saving...") : t("common.save", "Save Routine")}
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .routines-page {
    padding: 24px;
    max-width: 1400px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .icon-bubble {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    background: var(--bg-surface-elevated, #1e222a);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-color, #2d3340);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  h1 {
    font-size: 1.35rem;
    font-weight: 700;
    margin: 0;
  }

  .count-pill {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--primary, #3b82f6);
    color: #fff;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .subtitle {
    margin: 4px 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .notice-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 6px;
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.3);
    font-size: 0.85rem;
  }

  .controls-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    border-bottom: 1px solid var(--border-color, #2d3340);
    padding-bottom: 12px;
  }

  .search-wrap {
    position: relative;
    min-width: 240px;
  }

  :global(.search-icon) {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted, #8b949e);
  }

  .search-wrap input {
    width: 100%;
    padding: 6px 10px 6px 30px;
    border-radius: 6px;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .stats-summary {
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
  }

  /* Grid */
  .routines-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 16px;
  }

  .routine-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    transition: opacity 0.2s ease, border-color 0.15s ease;
  }

  .routine-card.disabled {
    opacity: 0.65;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .head-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .cron-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .cron-pill code {
    font-family: monospace;
    font-weight: 600;
    color: var(--text-normal, #e6edf3);
  }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .routine-name {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .routine-desc {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
    line-height: 1.4;
  }

  .prompt-preview {
    margin-top: 6px;
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 8px 10px;
    background: #0d1117;
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 6px;
    font-size: 0.78rem;
    color: var(--text-muted, #8b949e);
    line-height: 1.4;
  }

  :global(.prompt-icon) {
    flex-shrink: 0;
    margin-top: 2px;
    color: var(--primary, #3b82f6);
  }

  .card-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.8rem;
  }

  .agent-info {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .avatar-circle-sm {
    width: 20px;
    height: 20px;
    border-radius: 999px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.65rem;
    font-weight: 700;
    color: #fff;
  }

  .agent-name {
    font-weight: 500;
  }

  .last-run-info {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-muted, #8b949e);
  }

  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .action-btns {
    display: flex;
    gap: 6px;
  }

  .icon-only {
    padding: 6px 8px;
  }

  /* Switch */
  .switch-wrap {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
    cursor: pointer;
  }

  .switch-wrap input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    inset: 0;
    background-color: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 20px;
    transition: 0.2s;
  }

  .slider:before {
    position: absolute;
    content: "";
    height: 12px;
    width: 12px;
    left: 3px;
    bottom: 3px;
    background-color: var(--text-muted, #8b949e);
    border-radius: 50%;
    transition: 0.2s;
  }

  input:checked + .slider {
    background-color: var(--primary, #3b82f6);
    border-color: var(--primary, #3b82f6);
  }

  input:checked + .slider:before {
    transform: translateX(16px);
    background-color: #fff;
  }

  /* Empty state */
  .empty-state {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px 20px;
    text-align: center;
    background: var(--bg-surface-elevated, #161b22);
    border: 1px dashed var(--border-color, #2d3340);
    border-radius: 12px;
  }

  .empty-state h4 {
    margin: 0;
    font-size: 1.1rem;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
    max-width: 440px;
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-box {
    background: var(--bg-surface-elevated, #1c2128);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 10px;
    width: 100%;
    max-width: 520px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 600;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #8b949e);
    cursor: pointer;
    padding: 4px;
  }

  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-muted, #8b949e);
  }

  .form-group input,
  .form-group textarea,
  .form-group select {
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .form-error {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    color: #ef4444;
    font-size: 0.82rem;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }

  /* General Utilities */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: all 0.15s ease;
  }

  .btn-sm {
    padding: 5px 10px;
    font-size: 0.78rem;
  }

  .btn-primary {
    background: var(--primary, #3b82f6);
    color: #fff;
  }

  .btn-secondary {
    background: var(--bg-surface-elevated, #21262d);
    border-color: var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
  }

  .btn-danger-outline {
    background: transparent;
    border-color: rgba(239, 68, 68, 0.4);
    color: #ef4444;
  }

  .btn-danger-outline:hover {
    background: rgba(239, 68, 68, 0.1);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .status-pill {
    font-size: 0.7rem;
    padding: 1px 6px;
    border-radius: 4px;
    text-transform: capitalize;
    font-weight: 600;
  }

  .status-green { background: rgba(16, 185, 129, 0.2); color: #10b981; }
  .status-blue { background: rgba(59, 130, 246, 0.2); color: #3b82f6; }
  .status-orange { background: rgba(245, 158, 11, 0.2); color: #f59e0b; }
  .status-red { background: rgba(239, 68, 68, 0.2); color: #ef4444; }
  .status-gray { background: rgba(139, 148, 158, 0.2); color: #8b949e; }
  .status-faint { background: rgba(72, 79, 88, 0.2); color: #484f58; }

  .text-primary { color: var(--primary, #3b82f6); }
  .text-danger { color: #ef4444; }
  .text-muted { color: var(--text-muted, #8b949e); }
  .text-sm { font-size: 0.75rem; }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px 0;
    color: var(--text-muted, #8b949e);
  }

  .error-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 8px;
  }

  .error-card h4 {
    margin: 0;
    font-size: 0.95rem;
    color: #ef4444;
  }

  .error-card p {
    margin: 2px 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
  }
</style>
