<script lang="ts">
  import { page } from "$app/stores";
  import { onMount } from "svelte";
  import {
    companyRpc,
    t,
    formatRelativeTime,
    formatUsd,
    avatarColor,
    getInitials,
    type InboxData,
    type ApprovalItem,
    type IncidentItem,
    type InboxTaskItem,
  } from "$lib/v100/company-api";
  import {
    Inbox,
    CheckCircle2,
    XCircle,
    AlertTriangle,
    Clock,
    Filter,
    Search,
    Sparkles,
    RefreshCw,
    FileCode,
    DollarSign,
    UserPlus,
    Activity,
    ChevronRight,
    Check,
  } from "lucide-svelte";

  const companyId = $derived($page.params.id || "omniget-core");

  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state<string | null>(null);
  let inboxData = $state<InboxData>({ approvals: [], incidents: [], issues: [] });
  let activeTab = $state<"all" | "approvals" | "incidents" | "issues">("all");
  let searchQuery = $state("");
  let processingId = $state<string | null>(null);
  let notice = $state<{ type: "success" | "error"; text: string } | null>(null);

  async function loadInbox(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    error = null;
    try {
      const res = await companyRpc<InboxData>("inbox.list", { company: companyId });
      inboxData = {
        approvals: res?.approvals ?? [],
        incidents: res?.incidents ?? [],
        issues: res?.issues ?? [],
      };
    } catch (err: any) {
      error = err?.message || "Failed to load inbox items";
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadInbox();
  });

  function showNotice(text: string, type: "success" | "error" = "success") {
    notice = { type, text };
    setTimeout(() => {
      if (notice?.text === text) notice = null;
    }, 4000);
  }

  async function handleApprove(item: ApprovalItem) {
    processingId = item.id;
    try {
      await companyRpc("inbox.approve", { company: companyId, id: item.id });
      inboxData.approvals = inboxData.approvals.filter((a) => a.id !== item.id);
      showNotice(t("company.inbox.approved_msg", "Approval granted successfully."));
    } catch (err: any) {
      showNotice(err?.message || "Action failed", "error");
    } finally {
      processingId = null;
    }
  }

  async function handleReject(item: ApprovalItem) {
    processingId = item.id;
    try {
      await companyRpc("inbox.reject", { company: companyId, id: item.id });
      inboxData.approvals = inboxData.approvals.filter((a) => a.id !== item.id);
      showNotice(t("company.inbox.rejected_msg", "Request was rejected."));
    } catch (err: any) {
      showNotice(err?.message || "Action failed", "error");
    } finally {
      processingId = null;
    }
  }

  async function handleResolveIncident(item: IncidentItem) {
    processingId = item.id;
    try {
      await companyRpc("incident.resolve", { company: companyId, id: item.id });
      inboxData.incidents = inboxData.incidents.filter((i) => i.id !== item.id);
      showNotice(t("company.inbox.incident_resolved", "Incident marked resolved."));
    } catch (err: any) {
      showNotice(err?.message || "Action failed", "error");
    } finally {
      processingId = null;
    }
  }

  const filteredApprovals = $derived(
    inboxData.approvals.filter((a) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return a.title.toLowerCase().includes(q) || (a.note ?? "").toLowerCase().includes(q);
    })
  );

  const filteredIncidents = $derived(
    inboxData.incidents.filter((i) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return i.title.toLowerCase().includes(q) || (i.details ?? "").toLowerCase().includes(q);
    })
  );

  const filteredIssues = $derived(
    inboxData.issues.filter((iss) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return iss.title.toLowerCase().includes(q) || iss.identifier.toLowerCase().includes(q);
    })
  );

  const totalCount = $derived(
    inboxData.approvals.length + inboxData.incidents.length + inboxData.issues.length
  );
</script>

<svelte:head>
  <title>{t("company.inbox.title", "Inbox & Approvals")} - OmniGet</title>
</svelte:head>

<div class="inbox-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Inbox size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("company.inbox.header", "Company Inbox")}</h1>
          <span class="count-pill" class:empty={totalCount === 0}>{totalCount}</span>
        </div>
        <p class="subtitle">
          {t("company.inbox.sub", "Review human-in-the-loop approvals, anomaly alerts, and assigned work.")}
        </p>
      </div>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadInbox(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>
    </div>
  </header>

  {#if notice}
    <div class="notice-bar notice-{notice.type}">
      {#if notice.type === "success"}
        <Check size={16} />
      {:else}
        <AlertTriangle size={16} />
      {/if}
      <span>{notice.text}</span>
    </div>
  {/if}

  <!-- Controls Bar -->
  <div class="controls-bar">
    <div class="filter-tabs">
      <button
        class="tab-btn"
        class:active={activeTab === "all"}
        onclick={() => (activeTab = "all")}
      >
        <span>{t("company.inbox.all", "All Items")}</span>
        <span class="tab-badge">{totalCount}</span>
      </button>

      <button
        class="tab-btn"
        class:active={activeTab === "approvals"}
        onclick={() => (activeTab = "approvals")}
      >
        <span>{t("company.inbox.approvals", "Approvals")}</span>
        {#if inboxData.approvals.length > 0}
          <span class="tab-badge badge-warning">{inboxData.approvals.length}</span>
        {/if}
      </button>

      <button
        class="tab-btn"
        class:active={activeTab === "incidents"}
        onclick={() => (activeTab = "incidents")}
      >
        <span>{t("company.inbox.incidents", "Incidents")}</span>
        {#if inboxData.incidents.length > 0}
          <span class="tab-badge badge-danger">{inboxData.incidents.length}</span>
        {/if}
      </button>

      <button
        class="tab-btn"
        class:active={activeTab === "issues"}
        onclick={() => (activeTab = "issues")}
      >
        <span>{t("company.inbox.triage", "Triage Issues")}</span>
        {#if inboxData.issues.length > 0}
          <span class="tab-badge">{inboxData.issues.length}</span>
        {/if}
      </button>
    </div>

    <div class="search-wrap">
      <Search size={14} class="search-icon" />
      <input
        type="search"
        placeholder={t("company.inbox.search_placeholder", "Filter items...")}
        bind:value={searchQuery}
      />
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading inbox items...")}</p>
    </div>
  {:else if error}
    <div class="error-card">
      <AlertTriangle size={20} class="text-danger" />
      <div>
        <h4>{t("company.inbox.failed_title", "Failed to Load Inbox")}</h4>
        <p>{error}</p>
      </div>
      <button class="btn btn-secondary btn-sm" onclick={() => loadInbox()}>
        {t("common.retry", "Retry")}
      </button>
    </div>
  {:else if totalCount === 0}
    <!-- Inbox Zero state -->
    <div class="zero-state">
      <div class="zero-mascot">
        <Sparkles size={40} class="text-primary pulse" />
      </div>
      <h3>{t("company.inbox.zero_title", "Inbox Zero Reached")}</h3>
      <p>{t("company.inbox.zero_desc", "MosCat is purring peacefully. No pending approvals, incidents, or triage tasks require attention.")}</p>
    </div>
  {:else}
    <div class="inbox-sections">
      <!-- Approvals Section -->
      {#if (activeTab === "all" || activeTab === "approvals") && filteredApprovals.length > 0}
        <section class="item-group">
          <div class="group-header">
            <h3>{t("company.inbox.approvals_heading", "Pending Approvals")}</h3>
            <span class="badge badge-warning">{filteredApprovals.length}</span>
          </div>

          <div class="cards-grid">
            {#each filteredApprovals as item (item.id)}
              <div class="approval-card">
                <div class="card-top">
                  <div class="kind-tag">
                    {#if item.kind === "spend"}
                      <DollarSign size={13} class="text-warning" />
                      <span>{t("company.inbox.kind_spend", "Budget Spend")}</span>
                    {:else if item.kind === "diff"}
                      <FileCode size={13} class="text-primary" />
                      <span>{t("company.inbox.kind_diff", "Code Diff")}</span>
                    {:else if item.kind === "hire"}
                      <UserPlus size={13} class="text-success" />
                      <span>{t("company.inbox.kind_hire", "Staff Hire")}</span>
                    {:else}
                      <Activity size={13} />
                      <span>{item.kind}</span>
                    {/if}
                  </div>
                  <div class="card-time">
                    <Clock size={12} />
                    <span>{formatRelativeTime(item.created_ms, t)}</span>
                  </div>
                </div>

                <h4 class="card-title">{item.title}</h4>

                {#if item.note}
                  <p class="card-note">{item.note}</p>
                {/if}

                {#if item.amount_usd}
                  <div class="amount-pill">
                    <span>{t("company.inbox.requested_amount", "Requested:")}</span>
                    <strong>${formatUsd(item.amount_usd)} USD</strong>
                  </div>
                {/if}

                {#if item.diff}
                  <div class="diff-preview">
                    <pre><code>{item.diff}</code></pre>
                  </div>
                {/if}

                <div class="card-actions">
                  <button
                    class="btn btn-secondary btn-sm"
                    disabled={processingId === item.id}
                    onclick={() => handleReject(item)}
                  >
                    <XCircle size={14} />
                    <span>{t("company.inbox.reject", "Reject")}</span>
                  </button>
                  <button
                    class="btn btn-primary btn-sm"
                    disabled={processingId === item.id}
                    onclick={() => handleApprove(item)}
                  >
                    <CheckCircle2 size={14} />
                    <span>{t("company.inbox.approve", "Approve")}</span>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <!-- Incidents Section -->
      {#if (activeTab === "all" || activeTab === "incidents") && filteredIncidents.length > 0}
        <section class="item-group">
          <div class="group-header">
            <h3>{t("company.inbox.incidents_heading", "Active Incidents")}</h3>
            <span class="badge badge-danger">{filteredIncidents.length}</span>
          </div>

          <div class="incidents-list">
            {#each filteredIncidents as inc (inc.id)}
              <div class="incident-row" class:error={inc.severity === "error"}>
                <div class="inc-icon">
                  <AlertTriangle size={18} />
                </div>
                <div class="inc-content">
                  <div class="inc-head">
                    <span class="severity-pill {inc.severity}">{inc.severity}</span>
                    <span class="inc-kind">{inc.kind}</span>
                    <span class="inc-time">{formatRelativeTime(inc.created_ms, t)}</span>
                  </div>
                  <h4 class="inc-title">{inc.title}</h4>
                  {#if inc.details}
                    <p class="inc-details">{inc.details}</p>
                  {/if}
                  {#if inc.raised_by}
                    <span class="inc-raised">{t("company.inbox.raised_by", "Raised by:")} {inc.raised_by}</span>
                  {/if}
                </div>
                <div class="inc-actions">
                  <button
                    class="btn btn-secondary btn-sm"
                    disabled={processingId === inc.id}
                    onclick={() => handleResolveIncident(inc)}
                  >
                    <CheckCircle2 size={14} />
                    <span>{t("company.inbox.resolve", "Resolve")}</span>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <!-- Triage Issues Section -->
      {#if (activeTab === "all" || activeTab === "issues") && filteredIssues.length > 0}
        <section class="item-group">
          <div class="group-header">
            <h3>{t("company.inbox.triage_heading", "Assigned & Triage Issues")}</h3>
            <span class="badge badge-secondary">{filteredIssues.length}</span>
          </div>

          <div class="triage-list">
            {#each filteredIssues as issue (issue.id)}
              <a href="/llm/company/{companyId}/issues/{issue.identifier}" class="triage-row">
                <div class="triage-left">
                  <span class="issue-id">{issue.identifier}</span>
                  <span class="issue-title">{issue.title}</span>
                </div>
                <div class="triage-right">
                  <span class="why-badge">{issue.why}</span>
                  <span class="issue-time">{formatRelativeTime(issue.created_ms, t)}</span>
                  <ChevronRight size={16} class="chevron" />
                </div>
              </a>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .inbox-page {
    padding: 24px;
    max-width: 1200px;
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

  .count-pill.empty {
    background: var(--bg-surface-elevated, #2d3340);
    color: var(--text-muted, #8b949e);
  }

  .subtitle {
    margin: 4px 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
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

  .filter-tabs {
    display: flex;
    gap: 8px;
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 6px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-muted, #8b949e);
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-btn:hover {
    color: var(--text-normal, #e6edf3);
    background: var(--bg-surface-elevated, #1e222a);
  }

  .tab-btn.active {
    color: var(--text-normal, #e6edf3);
    background: var(--bg-surface-elevated, #1e222a);
    border-color: var(--border-color, #2d3340);
    font-weight: 600;
  }

  .tab-badge {
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 0.7rem;
    background: var(--bg-surface, #21262d);
  }

  .tab-badge.badge-warning {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .tab-badge.badge-danger {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .search-wrap {
    position: relative;
    min-width: 200px;
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

  .notice-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .notice-success {
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .notice-error {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .zero-state {
    text-align: center;
    padding: 60px 20px;
    background: var(--bg-surface-elevated, #161b22);
    border: 1px dashed var(--border-color, #2d3340);
    border-radius: 12px;
  }

  .zero-mascot {
    margin-bottom: 16px;
  }

  .zero-state h3 {
    font-size: 1.15rem;
    font-weight: 600;
    margin: 0 0 6px 0;
  }

  .zero-state p {
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
    max-width: 460px;
    margin: 0 auto;
  }

  .item-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .group-header h3 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
  }

  .cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
  }

  .approval-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .kind-tag {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
  }

  .card-time {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .card-title {
    font-size: 0.95rem;
    font-weight: 600;
    margin: 0;
  }

  .card-note {
    font-size: 0.82rem;
    color: var(--text-muted, #8b949e);
    margin: 0;
  }

  .amount-pill {
    font-size: 0.82rem;
    padding: 4px 8px;
    border-radius: 4px;
    background: rgba(245, 158, 11, 0.1);
    color: #f59e0b;
    display: inline-block;
  }

  .diff-preview {
    background: #0d1117;
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 4px;
    padding: 8px;
    overflow-x: auto;
    font-size: 0.75rem;
    max-height: 120px;
  }

  .diff-preview pre {
    margin: 0;
    font-family: monospace;
  }

  .card-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }

  .incidents-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .incident-row {
    display: flex;
    align-items: center;
    gap: 16px;
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid rgba(245, 158, 11, 0.3);
    border-radius: 8px;
    padding: 14px 16px;
  }

  .incident-row.error {
    border-color: rgba(239, 68, 68, 0.3);
  }

  .inc-icon {
    color: #f59e0b;
  }

  .incident-row.error .inc-icon {
    color: #ef4444;
  }

  .inc-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .inc-head {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.75rem;
  }

  .severity-pill {
    padding: 1px 6px;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 700;
  }

  .severity-pill.warning {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .severity-pill.error {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .inc-kind {
    color: var(--text-muted, #8b949e);
  }

  .inc-time {
    color: var(--text-muted, #8b949e);
  }

  .inc-title {
    font-size: 0.92rem;
    font-weight: 600;
    margin: 0;
  }

  .inc-details {
    font-size: 0.8rem;
    color: var(--text-muted, #8b949e);
    margin: 0;
  }

  .inc-raised {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
  }

  .triage-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .triage-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-radius: 8px;
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    text-decoration: none;
    color: inherit;
    transition: all 0.15s ease;
  }

  .triage-row:hover {
    background: var(--bg-surface, #1f242c);
    border-color: var(--primary, #3b82f6);
  }

  .triage-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .issue-id {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--primary, #3b82f6);
  }

  .issue-title {
    font-size: 0.88rem;
  }

  .triage-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .why-badge {
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    color: var(--text-muted, #8b949e);
    text-transform: capitalize;
  }

  .issue-time {
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .chevron {
    color: var(--text-muted, #8b949e);
  }

  /* General utilities */
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

  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 600;
  }

  .badge-warning {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .badge-danger {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .badge-secondary {
    background: var(--bg-surface, #21262d);
    color: var(--text-muted, #8b949e);
  }

  .text-primary {
    color: var(--primary, #3b82f6);
  }

  .text-warning {
    color: #f59e0b;
  }

  .text-danger {
    color: #ef4444;
  }

  .text-success {
    color: #10b981;
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }

  .pulse {
    animation: pulse 2s infinite ease-in-out;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.7; transform: scale(0.95); }
    50% { opacity: 1; transform: scale(1.05); }
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
