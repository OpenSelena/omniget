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
    buildOrgTree,
    type Staff,
    type StaffTreeNode,
  } from "$lib/v100/company-api";
  import {
    Users,
    UserPlus,
    Network,
    LayoutGrid,
    Search,
    AlertCircle,
    CheckCircle2,
    Edit3,
    Trash2,
    Shield,
    Cpu,
    Check,
    X,
    RefreshCw,
    Play,
    Pause,
    ChevronRight,
  } from "lucide-svelte";

  const companyId = $derived($page.params.id || "omniget-core");

  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state<string | null>(null);

  let staffList = $state<Staff[]>([]);
  let viewMode = $state<"grid" | "tree">("grid");
  let searchQuery = $state("");
  let statusFilter = $state<string>("all");

  // Hire / Edit Modal
  let showModal = $state(false);
  let editingStaffId = $state<string | null>(null);
  let saving = $state(false);
  let formError = $state<string | null>(null);

  let formName = $state("");
  let formRole = $state("");
  let formReportsTo = $state<string>("");
  let formModel = $state("claude-3-7-sonnet");
  let formSystemPrompt = $state("");
  let formCapabilities = $state<string[]>(["terminal", "fs", "git"]);

  const AVAILABLE_CAPABILITIES = [
    { id: "terminal", label: "Terminal Shell" },
    { id: "fs", label: "File System" },
    { id: "git", label: "Git VCS" },
    { id: "browser", label: "Web Browser" },
    { id: "media", label: "Media Sniffer" },
    { id: "telemetry", label: "Telemetry & Logs" },
  ];

  async function loadStaff(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    error = null;
    try {
      const res = await companyRpc<Staff[]>("staff.list", { company: companyId });
      staffList = Array.isArray(res) ? res : [];
    } catch (err: any) {
      error = err?.message || "Failed to load staff roster";
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadStaff();
  });

  const staffMap = $derived(new Map(staffList.map((s) => [s.id, s])));

  const filteredStaff = $derived(
    staffList.filter((s) => {
      if (statusFilter !== "all" && s.status !== statusFilter) return false;
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        s.name.toLowerCase().includes(q) ||
        s.role.toLowerCase().includes(q) ||
        (s.model ?? "").toLowerCase().includes(q) ||
        (s.capabilities ?? []).some((c) => c.toLowerCase().includes(q))
      );
    })
  );

  const orgTree = $derived(buildOrgTree(staffList));

  function openHireModal() {
    editingStaffId = null;
    formName = "";
    formRole = "";
    formReportsTo = "";
    formModel = "claude-3-7-sonnet";
    formSystemPrompt = "";
    formCapabilities = ["terminal", "fs", "git"];
    formError = null;
    showModal = true;
  }

  function openEditModal(staff: Staff) {
    editingStaffId = staff.id;
    formName = staff.name;
    formRole = staff.role;
    formReportsTo = staff.reports_to ?? "";
    formModel = staff.model || "claude-3-7-sonnet";
    formSystemPrompt = staff.system_prompt ?? "";
    formCapabilities = staff.capabilities ?? ["terminal", "fs"];
    formError = null;
    showModal = true;
  }

  function toggleCapability(capId: string) {
    if (formCapabilities.includes(capId)) {
      formCapabilities = formCapabilities.filter((c) => c !== capId);
    } else {
      formCapabilities = [...formCapabilities, capId];
    }
  }

  async function handleSaveStaff(e: SubmitEvent) {
    e.preventDefault();
    if (!formName.trim() || !formRole.trim()) {
      formError = "Name and Role are required";
      return;
    }
    saving = true;
    formError = null;

    try {
      if (editingStaffId) {
        const updated = await companyRpc<Staff>("staff.update", {
          company: companyId,
          staff_id: editingStaffId,
          patch: {
            name: formName.trim(),
            role: formRole.trim(),
            reports_to: formReportsTo || null,
            model: formModel,
            system_prompt: formSystemPrompt.trim(),
            capabilities: formCapabilities,
          },
        });
        const idx = staffList.findIndex((s) => s.id === editingStaffId);
        if (idx >= 0 && updated) staffList[idx] = updated;
      } else {
        const hired = await companyRpc<Staff>("staff.hire", {
          company: companyId,
          staff: {
            name: formName.trim(),
            role: formRole.trim(),
            reports_to: formReportsTo || null,
            model: formModel,
            system_prompt: formSystemPrompt.trim(),
            capabilities: formCapabilities,
          },
        });
        if (hired) staffList.push(hired);
      }
      showModal = false;
    } catch (err: any) {
      formError = err?.message || "Failed to save agent";
    } finally {
      saving = false;
    }
  }

  async function toggleAgentStatus(staff: Staff) {
    const nextStatus = staff.status === "running" ? "paused" : "running";
    const prev = staff.status;
    staff.status = nextStatus;
    try {
      await companyRpc("staff.update", {
        company: companyId,
        staff_id: staff.id,
        patch: { status: nextStatus },
      });
    } catch {
      staff.status = prev;
    }
  }

  async function handleFireStaff(staff: Staff) {
    if (!confirm(`Are you sure you want to terminate agent "${staff.name}" (${staff.role})?`)) return;
    try {
      await companyRpc("staff.fire", { company: companyId, staff_id: staff.id });
      staff.status = "terminated";
    } catch (err: any) {
      alert("Failed to terminate agent: " + (err?.message || err));
    }
  }
</script>

<svelte:head>
  <title>{t("company.team.title", "Team & Staff")} - OmniGet</title>
</svelte:head>

<div class="team-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Users size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("company.team.header", "Staff Roster")}</h1>
          <span class="count-pill">{staffList.length}</span>
        </div>
        <p class="subtitle">
          {t("company.team.sub", "Autonomous agent staff members, role hierarchy, system prompts, and tool permissions.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadStaff(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>

      <button class="btn btn-primary btn-sm" onclick={openHireModal}>
        <UserPlus size={15} />
        <span>{t("company.team.hire", "Hire Agent")}</span>
      </button>
    </div>
  </header>

  <!-- Controls Bar -->
  <div class="controls-bar">
    <div class="filters-left">
      <div class="search-wrap">
        <Search size={14} class="search-icon" />
        <input
          type="search"
          placeholder={t("company.team.search_placeholder", "Filter by name, role, model...")}
          bind:value={searchQuery}
        />
      </div>

      <select bind:value={statusFilter} class="filter-select">
        <option value="all">{t("company.team.all_statuses", "All Statuses")}</option>
        <option value="running">Running</option>
        <option value="idle">Idle</option>
        <option value="paused">Paused</option>
        <option value="terminated">Terminated</option>
      </select>
    </div>

    <div class="view-switcher">
      <button
        class="view-btn"
        class:active={viewMode === "grid"}
        onclick={() => (viewMode = "grid")}
        title="Grid View"
      >
        <LayoutGrid size={16} />
      </button>
      <button
        class="view-btn"
        class:active={viewMode === "tree"}
        onclick={() => (viewMode = "tree")}
        title="Org Chart Tree"
      >
        <Network size={16} />
      </button>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading team roster...")}</p>
    </div>
  {:else if error}
    <div class="error-card">
      <AlertCircle size={20} class="text-danger" />
      <div>
        <h4>{t("company.team.failed", "Failed to Load Team")}</h4>
        <p>{error}</p>
      </div>
      <button class="btn btn-secondary btn-sm" onclick={() => loadStaff()}>
        {t("common.retry", "Retry")}
      </button>
    </div>
  {:else if viewMode === "grid"}
    <!-- Grid View -->
    <div class="team-grid">
      {#each filteredStaff as staff (staff.id)}
        {@const supervisor = staff.reports_to ? staffMap.get(staff.reports_to) : null}
        <div class="staff-card" class:terminated={staff.status === "terminated"}>
          <div class="card-head">
            <div class="avatar-box">
              <div
                class="avatar-circle"
                style="background: {avatarColor(staff.name)};"
              >
                {getInitials(staff.name)}
              </div>
              <div>
                <h3 class="staff-name">{staff.name}</h3>
                <span class="staff-role">{staff.role}</span>
              </div>
            </div>

            <div class="status-indicator">
              <span class="dot dot-{statusColor(staff.status)}"></span>
              <span class="status-text">{staff.status}</span>
            </div>
          </div>

          <div class="card-body">
            {#if supervisor}
              <div class="meta-item">
                <span class="meta-label">{t("company.team.reports_to", "Reports to:")}</span>
                <span class="meta-val">{supervisor.name} ({supervisor.role})</span>
              </div>
            {/if}

            {#if staff.model}
              <div class="meta-item">
                <span class="meta-label">{t("company.team.model", "Model:")}</span>
                <span class="model-badge">
                  <Cpu size={11} />
                  <code>{staff.model}</code>
                </span>
              </div>
            {/if}

            {#if staff.capabilities && staff.capabilities.length > 0}
              <div class="caps-wrap">
                {#each staff.capabilities as cap}
                  <span class="cap-pill">
                    <Shield size={10} />
                    <span>{cap}</span>
                  </span>
                {/each}
              </div>
            {/if}
          </div>

          <div class="card-footer">
            <div class="action-btns">
              <button
                class="btn btn-secondary btn-sm"
                onclick={() => openEditModal(staff)}
                title="Edit agent"
              >
                <Edit3 size={13} />
                <span>{t("common.edit", "Edit")}</span>
              </button>

              {#if staff.status !== "terminated"}
                <button
                  class="btn btn-secondary btn-sm"
                  onclick={() => toggleAgentStatus(staff)}
                  title={staff.status === "running" ? "Pause agent" : "Resume agent"}
                >
                  {#if staff.status === "running"}
                    <Pause size={13} />
                  {:else}
                    <Play size={13} />
                  {/if}
                </button>

                <button
                  class="btn btn-danger-outline btn-sm icon-only"
                  onclick={() => handleFireStaff(staff)}
                  title="Terminate agent"
                >
                  <Trash2 size={13} />
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/each}

      {#if filteredStaff.length === 0}
        <div class="empty-state">
          <Users size={36} class="text-muted" />
          <h4>{t("company.team.no_agents", "No agents found")}</h4>
          <p>{t("company.team.empty_sub", "Hire autonomous agents to orchestrate and execute software engineering and operational tasks.")}</p>
          <button class="btn btn-primary btn-sm" onclick={openHireModal}>
            <UserPlus size={14} />
            <span>{t("company.team.hire", "Hire Agent")}</span>
          </button>
        </div>
      {/if}
    </div>
  {:else}
    <!-- Org Hierarchy Tree View -->
    <div class="tree-container">
      <h3>{t("company.team.org_chart", "Reporting Hierarchy")}</h3>
      <div class="tree-root">
        {#each orgTree as node (node.staff.id)}
          <div class="tree-node-block">
            <div class="tree-card root-node">
              <div
                class="avatar-circle-sm"
                style="background: {avatarColor(node.staff.name)};"
              >
                {getInitials(node.staff.name)}
              </div>
              <div class="node-info">
                <strong>{node.staff.name}</strong>
                <span>{node.staff.role}</span>
              </div>
              <span class="status-pill status-{statusColor(node.staff.status)}">
                {node.staff.status}
              </span>
            </div>

            {#if node.reports.length > 0}
              <div class="child-branches">
                {#each node.reports as child (child.staff.id)}
                  <div class="tree-card child-node">
                    <ChevronRight size={14} class="branch-arrow" />
                    <div
                      class="avatar-circle-sm"
                      style="background: {avatarColor(child.staff.name)};"
                    >
                      {getInitials(child.staff.name)}
                    </div>
                    <div class="node-info">
                      <strong>{child.staff.name}</strong>
                      <span>{child.staff.role}</span>
                    </div>
                    <span class="status-pill status-{statusColor(child.staff.status)}">
                      {child.staff.status}
                    </span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Hire / Edit Agent Modal -->
  {#if showModal}
    <div class="modal-backdrop" onclick={() => (showModal = false)}>
      <div class="modal-box" onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3>{editingStaffId ? t("company.team.modal_edit", "Edit Staff Member") : t("company.team.modal_hire", "Hire New Agent")}</h3>
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

        <form onsubmit={handleSaveStaff} class="modal-form">
          <div class="form-row">
            <div class="form-group">
              <label for="agent-name">{t("company.team.form_name", "Agent Name *")}</label>
              <input
                id="agent-name"
                type="text"
                placeholder="e.g. Lead Architect"
                bind:value={formName}
                required
              />
            </div>

            <div class="form-group">
              <label for="agent-role">{t("company.team.form_role", "Role / Title *")}</label>
              <input
                id="agent-role"
                type="text"
                placeholder="e.g. Systems Engineer"
                bind:value={formRole}
                required
              />
            </div>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="agent-reports">{t("company.team.form_reports", "Reports To (Supervisor)")}</label>
              <select id="agent-reports" bind:value={formReportsTo}>
                <option value="">{t("company.team.no_supervisor", "None (Top Orchestrator)")}</option>
                {#each staffList.filter((s) => s.id !== editingStaffId) as s}
                  <option value={s.id}>{s.name} ({s.role})</option>
                {/each}
              </select>
            </div>

            <div class="form-group">
              <label for="agent-model">{t("company.team.form_model", "Inference Model")}</label>
              <select id="agent-model" bind:value={formModel}>
                <option value="claude-3-7-sonnet">Claude 3.7 Sonnet</option>
                <option value="gpt-4o">GPT-4o</option>
                <option value="deepseek-r1">DeepSeek-R1 (Local/API)</option>
                <option value="ollama-llama3">Llama 3.3 (Local Ollama)</option>
              </select>
            </div>
          </div>

          <div class="form-group">
            <label for="agent-prompt">{t("company.team.form_system_prompt", "System Instructions / Prompt")}</label>
            <textarea
              id="agent-prompt"
              rows="3"
              placeholder="Primary directive, persona, constraints, and operational scope..."
              bind:value={formSystemPrompt}
            ></textarea>
          </div>

          <div class="form-group">
            <span class="caps-label">{t("company.team.form_capabilities", "Tool Capabilities")}</span>
            <div class="caps-selector">
              {#each AVAILABLE_CAPABILITIES as cap}
                <button
                  type="button"
                  class="cap-toggle-btn"
                  class:active={formCapabilities.includes(cap.id)}
                  onclick={() => toggleCapability(cap.id)}
                >
                  <span class="check-box" class:checked={formCapabilities.includes(cap.id)}>
                    {#if formCapabilities.includes(cap.id)}
                      <Check size={12} />
                    {/if}
                  </span>
                  <span>{cap.label}</span>
                </button>
              {/each}
            </div>
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
              {saving ? t("common.saving", "Saving...") : t("common.save", "Save Agent")}
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .team-page {
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

  .controls-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    border-bottom: 1px solid var(--border-color, #2d3340);
    padding-bottom: 12px;
  }

  .filters-left {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
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

  .filter-select {
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .view-switcher {
    display: flex;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 6px;
    padding: 2px;
  }

  .view-btn {
    background: transparent;
    border: none;
    padding: 6px 10px;
    border-radius: 4px;
    color: var(--text-muted, #8b949e);
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .view-btn.active {
    background: var(--bg-surface-elevated, #21262d);
    color: var(--text-normal, #e6edf3);
  }

  /* Team Grid */
  .team-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
  }

  .staff-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    transition: transform 0.15s ease, border-color 0.15s ease;
  }

  .staff-card.terminated {
    opacity: 0.6;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .avatar-box {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .avatar-circle {
    width: 40px;
    height: 40px;
    border-radius: 999px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    font-size: 0.85rem;
    font-weight: 700;
  }

  .staff-name {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .staff-role {
    font-size: 0.78rem;
    color: var(--text-muted, #8b949e);
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
    text-transform: capitalize;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
  }

  .dot-green { background: #10b981; }
  .dot-blue { background: #3b82f6; }
  .dot-orange { background: #f59e0b; }
  .dot-faint { background: #484f58; }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    font-size: 0.82rem;
  }

  .meta-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .meta-label {
    color: var(--text-muted, #8b949e);
  }

  .meta-val {
    font-weight: 500;
  }

  .model-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: var(--bg-surface, #21262d);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.72rem;
  }

  .caps-wrap {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }

  .cap-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(59, 130, 246, 0.1);
    color: var(--primary, #3b82f6);
    font-size: 0.7rem;
    font-weight: 500;
  }

  .card-footer {
    display: flex;
    justify-content: flex-end;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    padding-top: 10px;
  }

  .action-btns {
    display: flex;
    gap: 6px;
  }

  .icon-only {
    padding: 6px 8px;
  }

  /* Tree View */
  .tree-container {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .tree-container h3 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .tree-root {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .tree-node-block {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .tree-card {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 6px;
    background: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    max-width: 400px;
  }

  .avatar-circle-sm {
    width: 24px;
    height: 24px;
    border-radius: 999px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    font-size: 0.65rem;
    font-weight: 700;
  }

  .node-info {
    display: flex;
    flex-direction: column;
    font-size: 0.8rem;
  }

  .node-info span {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
  }

  .child-branches {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-left: 24px;
    border-left: 2px dashed var(--border-color, #30363d);
    margin-left: 12px;
  }

  :global(.branch-arrow) {
    color: var(--text-muted, #8b949e);
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
    max-width: 560px;
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

  .caps-label {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-muted, #8b949e);
  }

  .caps-selector {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .cap-toggle-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-muted, #8b949e);
    cursor: pointer;
    font-size: 0.8rem;
    transition: all 0.15s ease;
  }

  .cap-toggle-btn.active {
    border-color: var(--primary, #3b82f6);
    color: var(--text-normal, #e6edf3);
  }

  .check-box {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1px solid var(--border-color, #30363d);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .check-box.checked {
    background: var(--primary, #3b82f6);
    border-color: var(--primary, #3b82f6);
    color: #fff;
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
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: capitalize;
    font-weight: 600;
  }

  .status-green { background: rgba(16, 185, 129, 0.2); color: #10b981; }
  .status-blue { background: rgba(59, 130, 246, 0.2); color: #3b82f6; }
  .status-orange { background: rgba(245, 158, 11, 0.2); color: #f59e0b; }
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
</style>
