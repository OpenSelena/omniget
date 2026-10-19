<script lang="ts">
  import { page } from "$app/stores";
  import { onMount } from "svelte";
  import {
    companyRpc,
    t,
    avatarColor,
    getInitials,
    formatRelativeTime,
    type Issue,
    type Staff,
  } from "$lib/engine/company-api";
  import {
    Kanban,
    List,
    Plus,
    Search,
    Filter,
    AlertCircle,
    CheckCircle2,
    Clock,
    CircleDot,
    X,
    ChevronDown,
    User,
    Tag,
    RefreshCw,
  } from "lucide-svelte";

  const companyId = $derived($page.params.id || "omniget-core");

  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state<string | null>(null);

  let issues = $state<Issue[]>([]);
  let staffList = $state<Staff[]>([]);

  let viewMode = $state<"board" | "list">("board");
  let searchQuery = $state("");
  let statusFilter = $state<string>("all");
  let priorityFilter = $state<string>("all");
  let assigneeFilter = $state<string>("all");

  // New Issue Modal
  let showNewModal = $state(false);
  let creating = $state(false);
  let newTitle = $state("");
  let newDesc = $state("");
  let newPriority = $state<"low" | "medium" | "high" | "urgent">("medium");
  let newStatus = $state<"backlog" | "todo" | "in_progress">("todo");
  let newAssignee = $state<string>("");
  let newTags = $state<string>("");
  let formError = $state<string | null>(null);

  const COLUMNS: { id: Issue["status"]; label: string; color: string }[] = [
    { id: "backlog", label: "Backlog", color: "var(--text-muted, #8b949e)" },
    { id: "todo", label: "To Do", color: "var(--primary, #3b82f6)" },
    { id: "in_progress", label: "In Progress", color: "#f59e0b" },
    { id: "in_review", label: "In Review", color: "#8b5cf6" },
    { id: "blocked", label: "Blocked", color: "#ef4444" },
    { id: "done", label: "Done", color: "#10b981" },
  ];

  async function loadData(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    error = null;
    try {
      const [resIssues, resStaff] = await Promise.all([
        companyRpc<Issue[]>("issue.list", { company: companyId }),
        companyRpc<Staff[]>("staff.list", { company: companyId }),
      ]);
      issues = Array.isArray(resIssues) ? resIssues : [];
      staffList = Array.isArray(resStaff) ? resStaff : [];
    } catch (err: any) {
      error = err?.message || "Failed to load issues";
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadData();
  });

  const staffMap = $derived(
    new Map(staffList.map((s) => [s.id, s]))
  );

  const filteredIssues = $derived(
    issues.filter((item) => {
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const matches =
          item.identifier.toLowerCase().includes(q) ||
          item.title.toLowerCase().includes(q) ||
          (item.description ?? "").toLowerCase().includes(q) ||
          (item.tags ?? []).some((t) => t.toLowerCase().includes(q));
        if (!matches) return false;
      }
      if (statusFilter !== "all" && item.status !== statusFilter) return false;
      if (priorityFilter !== "all" && item.priority !== priorityFilter) return false;
      if (assigneeFilter !== "all") {
        if (assigneeFilter === "unassigned" && item.assignee_id) return false;
        if (assigneeFilter !== "unassigned" && item.assignee_id !== assigneeFilter) return false;
      }
      return true;
    })
  );

  async function updateStatus(issue: Issue, newStatus: Issue["status"]) {
    const oldStatus = issue.status;
    issue.status = newStatus;
    try {
      await companyRpc("issue.update", {
        company: companyId,
        id: issue.id,
        identifier: issue.identifier,
        patch: { status: newStatus },
      });
    } catch {
      issue.status = oldStatus;
    }
  }

  async function handleCreateIssue(e: SubmitEvent) {
    e.preventDefault();
    if (!newTitle.trim()) {
      formError = "Issue title is required";
      return;
    }
    creating = true;
    formError = null;
    try {
      const tagsArray = newTags
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);

      const created = await companyRpc<Issue>("issue.create", {
        company: companyId,
        issue: {
          title: newTitle.trim(),
          description: newDesc.trim(),
          status: newStatus,
          priority: newPriority,
          assignee_id: newAssignee || null,
          tags: tagsArray,
        },
      });

      if (created) {
        issues.unshift(created);
        showNewModal = false;
        newTitle = "";
        newDesc = "";
        newTags = "";
      }
    } catch (err: any) {
      formError = err?.message || "Failed to create issue";
    } finally {
      creating = false;
    }
  }
</script>

<svelte:head>
  <title>{t("company.issues.title", "Issues & Kanban")} - OmniGet</title>
</svelte:head>

<div class="issues-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Kanban size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("company.issues.header", "Issue Tracker")}</h1>
          <span class="count-pill">{filteredIssues.length}</span>
        </div>
        <p class="subtitle">
          {t("company.issues.sub", "Plan, track, and assign automated task workloads to agents.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadData(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>

      <button class="btn btn-primary btn-sm" onclick={() => (showNewModal = true)}>
        <Plus size={15} />
        <span>{t("company.issues.new", "New Issue")}</span>
      </button>
    </div>
  </header>

  <!-- Filter & View Controls -->
  <div class="controls-bar">
    <div class="filters-left">
      <div class="search-wrap">
        <Search size={14} class="search-icon" />
        <input
          type="search"
          placeholder={t("company.issues.search_placeholder", "Search identifier, title, tags...")}
          bind:value={searchQuery}
        />
      </div>

      <select bind:value={statusFilter} class="filter-select">
        <option value="all">{t("company.issues.all_statuses", "All Statuses")}</option>
        <option value="backlog">Backlog</option>
        <option value="todo">To Do</option>
        <option value="in_progress">In Progress</option>
        <option value="in_review">In Review</option>
        <option value="blocked">Blocked</option>
        <option value="done">Done</option>
        <option value="cancelled">Cancelled</option>
      </select>

      <select bind:value={priorityFilter} class="filter-select">
        <option value="all">{t("company.issues.all_priorities", "All Priorities")}</option>
        <option value="urgent">Urgent</option>
        <option value="high">High</option>
        <option value="medium">Medium</option>
        <option value="low">Low</option>
      </select>

      <select bind:value={assigneeFilter} class="filter-select">
        <option value="all">{t("company.issues.all_assignees", "All Assignees")}</option>
        <option value="unassigned">{t("company.issues.unassigned", "Unassigned")}</option>
        {#each staffList as s}
          <option value={s.id}>{s.name} ({s.role})</option>
        {/each}
      </select>
    </div>

    <div class="view-switcher">
      <button
        class="view-btn"
        class:active={viewMode === "board"}
        onclick={() => (viewMode = "board")}
        title="Kanban Board"
      >
        <Kanban size={16} />
      </button>
      <button
        class="view-btn"
        class:active={viewMode === "list"}
        onclick={() => (viewMode = "list")}
        title="Table List"
      >
        <List size={16} />
      </button>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading issues...")}</p>
    </div>
  {:else if error}
    <div class="error-card">
      <AlertCircle size={20} class="text-danger" />
      <div>
        <h4>{t("company.issues.failed", "Failed to Load Issues")}</h4>
        <p>{error}</p>
      </div>
      <button class="btn btn-secondary btn-sm" onclick={() => loadData()}>
        {t("common.retry", "Retry")}
      </button>
    </div>
  {:else if viewMode === "board"}
    <!-- Kanban Board -->
    <div class="kanban-board">
      {#each COLUMNS as col (col.id)}
        {@const colIssues = filteredIssues.filter((i) => i.status === col.id)}
        <div class="kanban-col">
          <div class="col-header">
            <div class="col-title">
              <span class="col-dot" style="background: {col.color};"></span>
              <h4>{col.label}</h4>
            </div>
            <span class="col-count">{colIssues.length}</span>
          </div>

          <div class="col-cards">
            {#each colIssues as issue (issue.id)}
              {@const assignee = issue.assignee_id ? staffMap.get(issue.assignee_id) : null}
              <div class="kanban-card">
                <div class="card-head">
                  <a
                    href="/llm/company/{companyId}/issues/{issue.identifier}"
                    class="issue-key"
                  >
                    {issue.identifier}
                  </a>

                  {#if issue.priority}
                    <span class="priority-pill priority-{issue.priority}">
                      {issue.priority}
                    </span>
                  {/if}
                </div>

                <a
                  href="/llm/company/{companyId}/issues/{issue.identifier}"
                  class="card-title-link"
                >
                  <h5>{issue.title}</h5>
                </a>

                {#if issue.tags && issue.tags.length > 0}
                  <div class="card-tags">
                    {#each issue.tags as tag}
                      <span class="tag-pill">#{tag}</span>
                    {/each}
                  </div>
                {/if}

                <div class="card-footer">
                  <div class="assignee-box">
                    {#if assignee}
                      <span
                        class="avatar-circle"
                        style="background: {avatarColor(assignee.name)};"
                        title="{assignee.name} ({assignee.role})"
                      >
                        {getInitials(assignee.name)}
                      </span>
                      <span class="assignee-name">{assignee.name}</span>
                    {:else}
                      <span class="unassigned-text">{t("company.issues.unassigned", "Unassigned")}</span>
                    {/if}
                  </div>

                  <!-- Quick Status Dropdown -->
                  <select
                    value={issue.status}
                    class="status-select-sm"
                    onchange={(e) => updateStatus(issue, e.currentTarget.value as any)}
                  >
                    <option value="backlog">Backlog</option>
                    <option value="todo">To Do</option>
                    <option value="in_progress">In Progress</option>
                    <option value="in_review">In Review</option>
                    <option value="blocked">Blocked</option>
                    <option value="done">Done</option>
                  </select>
                </div>
              </div>
            {/each}

            {#if colIssues.length === 0}
              <div class="col-empty">
                <span>{t("company.issues.col_empty", "No issues")}</span>
              </div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <!-- List / Table View -->
    <div class="table-container">
      <table class="issues-table">
        <thead>
          <tr>
            <th style="width: 100px;">{t("company.issues.th_id", "ID")}</th>
            <th>{t("company.issues.th_title", "Title")}</th>
            <th style="width: 110px;">{t("company.issues.th_priority", "Priority")}</th>
            <th style="width: 140px;">{t("company.issues.th_status", "Status")}</th>
            <th style="width: 160px;">{t("company.issues.th_assignee", "Assignee")}</th>
            <th style="width: 120px;">{t("company.issues.th_created", "Created")}</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredIssues as issue (issue.id)}
            {@const assignee = issue.assignee_id ? staffMap.get(issue.assignee_id) : null}
            <tr>
              <td>
                <a
                  href="/llm/company/{companyId}/issues/{issue.identifier}"
                  class="issue-key"
                >
                  {issue.identifier}
                </a>
              </td>
              <td>
                <a
                  href="/llm/company/{companyId}/issues/{issue.identifier}"
                  class="table-title-link"
                >
                  {issue.title}
                </a>
                {#if issue.tags && issue.tags.length > 0}
                  <span class="table-tags">
                    {#each issue.tags as tg}
                      <span class="tag-pill-sm">#{tg}</span>
                    {/each}
                  </span>
                {/if}
              </td>
              <td>
                {#if issue.priority}
                  <span class="priority-pill priority-{issue.priority}">
                    {issue.priority}
                  </span>
                {:else}
                  <span class="text-muted">-</span>
                {/if}
              </td>
              <td>
                <select
                  value={issue.status}
                  class="status-select-sm"
                  onchange={(e) => updateStatus(issue, e.currentTarget.value as any)}
                >
                  <option value="backlog">Backlog</option>
                  <option value="todo">To Do</option>
                  <option value="in_progress">In Progress</option>
                  <option value="in_review">In Review</option>
                  <option value="blocked">Blocked</option>
                  <option value="done">Done</option>
                  <option value="cancelled">Cancelled</option>
                </select>
              </td>
              <td>
                {#if assignee}
                  <div class="table-assignee">
                    <span
                      class="avatar-circle-sm"
                      style="background: {avatarColor(assignee.name)};"
                    >
                      {getInitials(assignee.name)}
                    </span>
                    <span>{assignee.name}</span>
                  </div>
                {:else}
                  <span class="text-muted">{t("company.issues.unassigned", "Unassigned")}</span>
                {/if}
              </td>
              <td>
                <span class="text-muted text-sm">
                  {formatRelativeTime(issue.created_at, t)}
                </span>
              </td>
            </tr>
          {/each}

          {#if filteredIssues.length === 0}
            <tr>
              <td colspan="6" class="table-empty">
                {t("company.issues.none_found", "No issues match the search criteria.")}
              </td>
            </tr>
          {/if}
        </tbody>
      </table>
    </div>
  {/if}

  <!-- New Issue Modal Dialog -->
  {#if showNewModal}
    <div class="modal-backdrop" onclick={() => (showNewModal = false)}>
      <div class="modal-box" onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3>{t("company.issues.modal_new_title", "Create New Issue")}</h3>
          <button class="icon-btn" onclick={() => (showNewModal = false)}>
            <X size={18} />
          </button>
        </div>

        {#if formError}
          <div class="form-error">
            <AlertCircle size={15} />
            <span>{formError}</span>
          </div>
        {/if}

        <form onsubmit={handleCreateIssue} class="modal-form">
          <div class="form-group">
            <label for="iss-title">{t("company.issues.form_title", "Title *")}</label>
            <input
              id="iss-title"
              type="text"
              placeholder={t("company.issues.title_placeholder", "e.g. Scrape playlist metadata")}
              bind:value={newTitle}
              required
            />
          </div>

          <div class="form-group">
            <label for="iss-desc">{t("company.issues.form_desc", "Description")}</label>
            <textarea
              id="iss-desc"
              rows="4"
              placeholder={t("company.issues.desc_placeholder", "Detailed requirements, context or reproduction steps...")}
              bind:value={newDesc}
            ></textarea>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="iss-status">{t("company.issues.form_status", "Status")}</label>
              <select id="iss-status" bind:value={newStatus}>
                <option value="todo">To Do</option>
                <option value="backlog">Backlog</option>
                <option value="in_progress">In Progress</option>
              </select>
            </div>

            <div class="form-group">
              <label for="iss-priority">{t("company.issues.form_priority", "Priority")}</label>
              <select id="iss-priority" bind:value={newPriority}>
                <option value="urgent">Urgent</option>
                <option value="high">High</option>
                <option value="medium">Medium</option>
                <option value="low">Low</option>
              </select>
            </div>
          </div>

          <div class="form-group">
            <label for="iss-assignee">{t("company.issues.form_assignee", "Assignee")}</label>
            <select id="iss-assignee" bind:value={newAssignee}>
              <option value="">{t("company.issues.unassigned", "Unassigned")}</option>
              {#each staffList as s}
                <option value={s.id}>{s.name} ({s.role})</option>
              {/each}
            </select>
          </div>

          <div class="form-group">
            <label for="iss-tags">{t("company.issues.form_tags", "Tags (comma separated)")}</label>
            <input
              id="iss-tags"
              type="text"
              placeholder="e.g. core, downloader, engine"
              bind:value={newTags}
            />
          </div>

          <div class="modal-footer">
            <button
              type="button"
              class="btn btn-secondary"
              disabled={creating}
              onclick={() => (showNewModal = false)}
            >
              {t("common.cancel", "Cancel")}
            </button>
            <button type="submit" class="btn btn-primary" disabled={creating}>
              {creating ? t("common.saving", "Creating...") : t("company.issues.create_btn", "Create Issue")}
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .issues-page {
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
    min-width: 220px;
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

  /* Kanban Board */
  .kanban-board {
    display: flex;
    gap: 16px;
    overflow-x: auto;
    padding-bottom: 16px;
  }

  .kanban-col {
    flex: 0 0 280px;
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 240px);
  }

  .col-header {
    padding: 12px 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-color, #2d3340);
  }

  .col-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .col-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
  }

  .col-title h4 {
    margin: 0;
    font-size: 0.88rem;
    font-weight: 600;
  }

  .col-count {
    font-size: 0.75rem;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--bg-surface, #21262d);
    color: var(--text-muted, #8b949e);
  }

  .col-cards {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
    flex: 1;
  }

  .kanban-card {
    background: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 6px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition: transform 0.15s ease, border-color 0.15s ease;
  }

  .kanban-card:hover {
    border-color: var(--primary, #3b82f6);
    transform: translateY(-1px);
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .issue-key {
    font-size: 0.78rem;
    font-weight: 700;
    color: var(--primary, #3b82f6);
    text-decoration: none;
  }

  .issue-key:hover {
    text-decoration: underline;
  }

  .priority-pill {
    font-size: 0.68rem;
    padding: 1px 6px;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 700;
  }

  .priority-urgent {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .priority-high {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .priority-medium {
    background: rgba(59, 130, 246, 0.2);
    color: #3b82f6;
  }

  .priority-low {
    background: rgba(139, 148, 158, 0.2);
    color: #8b949e;
  }

  .card-title-link {
    text-decoration: none;
    color: inherit;
  }

  .card-title-link h5 {
    margin: 0;
    font-size: 0.88rem;
    font-weight: 500;
    line-height: 1.35;
  }

  .card-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .tag-pill {
    font-size: 0.7rem;
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-muted, #8b949e);
  }

  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 4px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    padding-top: 6px;
  }

  .assignee-box {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .avatar-circle {
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

  .assignee-name {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
  }

  .unassigned-text {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
    font-style: italic;
  }

  .status-select-sm {
    font-size: 0.72rem;
    padding: 2px 4px;
    border-radius: 4px;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-muted, #8b949e);
  }

  .col-empty {
    text-align: center;
    padding: 24px 0;
    color: var(--text-muted, #8b949e);
    font-size: 0.8rem;
    font-style: italic;
  }

  /* Table / List View */
  .table-container {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    overflow-x: auto;
  }

  .issues-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .issues-table th {
    text-align: left;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-color, #2d3340);
    color: var(--text-muted, #8b949e);
    font-weight: 600;
  }

  .issues-table td {
    padding: 10px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .table-title-link {
    text-decoration: none;
    color: inherit;
    font-weight: 500;
  }

  .table-title-link:hover {
    text-decoration: underline;
  }

  .table-tags {
    margin-left: 8px;
    display: inline-flex;
    gap: 4px;
  }

  .tag-pill-sm {
    font-size: 0.68rem;
    padding: 0 4px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-muted, #8b949e);
  }

  .table-assignee {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .avatar-circle-sm {
    width: 18px;
    height: 18px;
    border-radius: 999px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.6rem;
    font-weight: 700;
    color: #fff;
  }

  .table-empty {
    text-align: center;
    padding: 30px;
    color: var(--text-muted, #8b949e);
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

  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-secondary {
    background: var(--bg-surface-elevated, #21262d);
    border-color: var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--border-color, #2d3340);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .text-primary {
    color: var(--primary, #3b82f6);
  }

  .text-danger {
    color: #ef4444;
  }

  .text-muted {
    color: var(--text-muted, #8b949e);
  }

  .text-sm {
    font-size: 0.75rem;
  }

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
