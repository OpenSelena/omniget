<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import {
    type DashboardData,
    type Issue,
    type BudgetStatus,
    companyRpc,
    onCompanyChanged,
    formatUsd,
    formatMicroUsd,
    formatDuration,
    formatRelativeTime,
    statusColor,
    cleanError,
    t,
  } from "$lib/engine/company-api";

  const companyId = $derived(page.params.id ?? "");
  const basePath = $derived(`/llm/company/${companyId}`);

  let dashboard = $state<DashboardData | null>(null);
  let issueMap = $state<Map<string, Issue>>(new Map());
  let errorMsg = $state("");
  let now = $state(Date.now());

  const STATUS_KEYS = [
    "backlog",
    "todo",
    "in_progress",
    "in_review",
    "blocked",
    "done",
    "cancelled",
  ] as const;

  async function loadDashboard(id: string) {
    if (!id) return;
    try {
      const [dash, issues]: [DashboardData, Issue[]] = await Promise.all([
        companyRpc("company.dashboard", { company: id }),
        companyRpc("issue.list", {
          company: id,
          filter: { include_closed: true, limit: 500 },
        }).catch(() => []),
      ]);

      dashboard = dash;
      issueMap = new Map(issues.map((i) => [i.id, i]));
      errorMsg = "";
    } catch (err) {
      errorMsg = cleanError(err);
    }
  }

  $effect(() => {
    const id = companyId;
    if (id) {
      loadDashboard(id);
    }
  });

  onMount(() => {
    const id = companyId;
    if (id) {
      loadDashboard(id);
      const unsubscribe = onCompanyChanged(id, () => {
        loadDashboard(id);
      });
      const interval = setInterval(() => {
        now = Date.now();
      }, 15000);

      return () => {
        clearInterval(interval);
        unsubscribe();
      };
    }
  });

  const staffMap = $derived(
    new Map((dashboard?.staff ?? []).map((s) => [s.id, s.name]))
  );
  const activeGoals = $derived(
    (dashboard?.goals ?? [])
      .filter((g) => g.status === "active" && g.level !== "task")
      .slice(0, 6)
  );
  const totalTasks = $derived(
    STATUS_KEYS.reduce((acc, k) => acc + (dashboard?.issue_counts?.[k] ?? 0), 0)
  );
  const openTasks = $derived(
    (dashboard?.issue_counts?.todo ?? 0) +
      (dashboard?.issue_counts?.in_progress ?? 0) +
      (dashboard?.issue_counts?.in_review ?? 0) +
      (dashboard?.issue_counts?.blocked ?? 0)
  );
  const activeStaffCount = $derived(
    (dashboard?.staff ?? []).filter((s) => s.status !== "terminated").length
  );

  function getBudgetScopeName(b: BudgetStatus): string {
    const p = b.policy;
    if (p.scope_type === "company") return t("company.overview.budget_company", "Organization");
    if (p.scope_type === "agent") return (p.scope_id && staffMap.get(p.scope_id)) || p.scope_id || "Agent";
    const proj = dashboard?.projects.find((x) => x.id === p.scope_id);
    return proj?.name || p.scope_id || "Project";
  }

  function formatBudgetMetric(metric: string, val: number): string {
    return metric === "usd" ? `US$ ${formatUsd(val)}` : Math.round(val).toLocaleString();
  }

  function getBudgetFillClass(b: BudgetStatus): string {
    if (b.pct >= 100) return "red";
    if (b.pct >= b.policy.warn_pct) return "orange";
    return "green";
  }
</script>

<div class="page page-wide overview">
  {#if errorMsg}
    <p class="field-error">{errorMsg}</p>
  {/if}

  {#if !dashboard}
    <div class="stats">
      {#each [0, 1, 2, 3] as _}
        <div class="stat skeleton" style="height: 72px"></div>
      {/each}
    </div>
  {:else}
    <!-- Top Stats Cards -->
    <div class="stats">
      <a class="stat surface-card" href="{basePath}/team">
        <span class="stat-num">
          {dashboard.running}
          <span class="stat-of">/{activeStaffCount}</span>
        </span>
        <span class="stat-label">{t("company.overview.running", "Running Staff")}</span>
      </a>

      <a class="stat surface-card" href="{basePath}/issues">
        <span class="stat-num">{openTasks}</span>
        <span class="stat-label">{t("company.overview.open_tasks", "Open Issues")}</span>
      </a>

      <a class="stat surface-card" href="{basePath}/inbox">
        <span class="stat-num" class:warn={dashboard.open_approvals > 0}>
          {dashboard.open_approvals}
        </span>
        <span class="stat-label">{t("company.overview.approvals", "Pending Approvals")}</span>
      </a>

      <a class="stat surface-card" href="{basePath}/costs">
        <span class="stat-num">US$ {formatUsd(dashboard.costs.cost_usd)}</span>
        <span class="stat-label">{t("company.overview.spend_month", "Spend This Month")}</span>
      </a>
    </div>

    <!-- Mission & Tasks Grid -->
    <div class="grid">
      <section class="block">
        <div class="section-header">
          <h2 class="section-header-title">{t("company.overview.mission", "Mission & Goals")}</h2>
          <a class="section-header-action" href="{basePath}/goals">
            {t("company.overview.see_goals", "View All Goals")}
          </a>
        </div>
        <div class="group pad">
          <p class="mission-text">
            {dashboard.company.mission || t("company.overview.no_mission", "No strategic mission defined.")}
          </p>
          {#if activeGoals.length > 0}
            <ul class="goal-list">
              {#each activeGoals as goal (goal.id)}
                <li>
                  <span class="tag">{t(`company.common.goal_level.${goal.level}`, goal.level)}</span>
                  <span class="goal-title">{goal.title}</span>
                  {#if goal.owner_staff_id}
                    <span class="dim">{staffMap.get(goal.owner_staff_id) ?? ""}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="hint">{t("company.overview.no_goals", "No active strategic objectives yet.")}</p>
          {/if}
        </div>
      </section>

      <section class="block">
        <div class="section-header">
          <h2 class="section-header-title">{t("company.overview.tasks", "Task Breakdown")}</h2>
          <a class="section-header-action" href="{basePath}/issues">
            {t("company.overview.see_tasks", "View Issue Board")}
          </a>
        </div>
        <div class="group pad">
          {#if totalTasks === 0}
            <p class="hint">{t("company.overview.no_tasks", "No issues tracked in this organization.")}</p>
          {:else}
            <div class="stack-bar" role="img" aria-label={t("company.overview.tasks", "Tasks")}>
              {#each STATUS_KEYS as sk}
                {#if (dashboard.issue_counts[sk] ?? 0) > 0}
                  <span
                    class="seg {statusColor(sk)}"
                    title="{t(`company.common.status.${sk}`, sk)}: {dashboard.issue_counts[sk]}"
                    style="flex-grow: {dashboard.issue_counts[sk]}"
                  ></span>
                {/if}
              {/each}
            </div>

            <ul class="legend">
              {#each STATUS_KEYS as sk}
                <li class:zero={!dashboard.issue_counts[sk]}>
                  <span class="sw {statusColor(sk)}"></span>
                  {t(`company.common.status.${sk}`, sk)}
                  <span class="num">{dashboard.issue_counts[sk] ?? 0}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </section>
    </div>

    <!-- Budgets & Approvals Grid -->
    <div class="grid">
      <section class="block">
        <div class="section-header">
          <h2 class="section-header-title">{t("company.overview.budgets", "Budget Policies")}</h2>
          <a class="section-header-action" href="{basePath}/costs">
            {t("company.overview.see_costs", "Cost Details")}
          </a>
        </div>
        <div class="group pad">
          {#if dashboard.budgets.length === 0}
            <p class="hint">{t("company.overview.no_budgets", "No financial budget caps set.")}</p>
          {:else}
            {#each dashboard.budgets as b (b.policy.id)}
              <div class="budget">
                <div class="budget-line">
                  <span class="budget-name">
                    {getBudgetScopeName(b)} · {t(`company.overview.window.${b.policy.window}`, b.policy.window)}
                  </span>
                  <span class="num">
                    {formatBudgetMetric(b.policy.metric, b.used)} / {formatBudgetMetric(b.policy.metric, b.policy.amount)}
                  </span>
                </div>
                <div class="bar">
                  <span
                    class="fill {getBudgetFillClass(b)}"
                    style="width: {Math.min(100, b.pct)}%"
                  ></span>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </section>

      <section class="block">
        <div class="section-header">
          <h2 class="section-header-title">{t("company.overview.open_approvals", "Approvals & Triage")}</h2>
          <a class="section-header-action" href="{basePath}/inbox">
            {t("company.overview.see_inbox", "Open Inbox")}
          </a>
        </div>
        <div class="group">
          {#if (dashboard.recent_runs ?? []).length === 0 && dashboard.open_approvals === 0}
            <div class="group-row">
              <span class="hint">{t("company.overview.no_approvals", "Inbox is clean. No pending approvals.")}</span>
            </div>
          {:else}
            <div class="group-row">
              <span class="hint">{t("company.overview.no_approvals", "Inbox is clean. No pending approvals.")}</span>
            </div>
          {/if}
        </div>
      </section>
    </div>

    <!-- Recent Runs Stream -->
    <section class="block">
      <div class="section-header">
        <h2 class="section-header-title">{t("company.overview.recent_runs", "Recent Executions")}</h2>
        <a class="section-header-action" href="{basePath}/activity">
          {t("company.overview.see_activity", "View Audit Stream")}
        </a>
      </div>
      <div class="group">
        {#if dashboard.recent_runs.length === 0}
          <div class="group-row">
            <span class="hint">{t("company.overview.no_runs", "No recent agent runs recorded.")}</span>
          </div>
        {:else}
          {#each dashboard.recent_runs.slice(0, 12) as run (run.id)}
            {@const matchedIssue = run.issue_id ? issueMap.get(run.issue_id) : undefined}
            <div class="group-row run-row">
              <span class="tag {statusColor(run.state)}">
                {t(`company.common.run_state.${run.state}`, run.state)}
              </span>
              <span class="run-agent">{staffMap.get(run.staff_id) ?? run.staff_id}</span>
              <span class="run-task" title={matchedIssue?.title ?? ""}>
                {#if matchedIssue}
                  <span class="ident">{matchedIssue.identifier}</span>
                  {matchedIssue.title}
                {:else}
                  <span class="dim">{t(`company.overview.source.${run.source}`, run.source)}</span>
                {/if}
              </span>
              <span class="run-cost num">US$ {formatMicroUsd(run.cost_micros)}</span>
              <span class="run-dur num">
                {formatDuration(run.started_ms, run.finished_ms ?? (run.state === "running" ? now : run.started_ms))}
              </span>
              <span class="run-when dim">
                {formatRelativeTime(run.created_ms, t, now)}
              </span>
            </div>
          {/each}
        {/if}
      </div>
    </section>
  {/if}
</div>

<style>
  .overview {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: var(--space-3);
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-3) var(--space-4);
    text-decoration: none;
    color: inherit;
  }
  @media (hover: hover) {
    a.stat:hover {
      background: var(--surface-hi);
    }
  }
  .stat-num {
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .stat-num.warn {
    color: var(--orange);
  }
  .stat-of {
    font-size: var(--text-sm);
    color: var(--text-dim);
    font-weight: 500;
  }
  .stat-label {
    font-size: var(--text-sm);
    color: var(--text-dim);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: var(--space-5);
  }
  .block {
    min-width: 0;
  }
  .pad {
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .mission-text {
    margin: 0;
    font-size: var(--text-base);
    line-height: var(--leading-base);
    white-space: pre-wrap;
  }
  .goal-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .goal-list li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    min-width: 0;
  }
  .goal-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-dim);
  }
  .dim {
    font-size: var(--text-xs);
    color: var(--text-dim);
  }
  .num {
    font-variant-numeric: tabular-nums;
  }
  .stack-bar {
    display: flex;
    height: 10px;
    border-radius: 5px;
    overflow: hidden;
    gap: 2px;
    background: var(--fill-1);
  }
  .seg {
    min-width: 4px;
  }
  .legend {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: 4px var(--space-3);
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }
  .legend li {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
  }
  .legend li.zero {
    opacity: 0.5;
  }
  .legend .num {
    margin-left: auto;
    color: var(--text);
    font-weight: 600;
  }
  .sw {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .budget {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .budget-line {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .budget-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--fill-2);
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    border-radius: 3px;
  }
  .run-row {
    font-size: var(--text-sm);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .run-agent {
    font-weight: 600;
    flex-shrink: 0;
    min-width: 80px;
  }
  .run-task {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ident {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-dim);
  }
  .run-cost,
  .run-dur {
    flex-shrink: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
    min-width: 64px;
    text-align: right;
  }
  .run-when {
    flex-shrink: 0;
    min-width: 72px;
    text-align: right;
  }
  @media (max-width: 720px) {
    .run-cost,
    .run-when {
      display: none;
    }
  }
</style>
