<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import {
    type CostSummary,
    type BudgetPolicy,
    type Staff,
    type Project,
    companyRpc,
    onCompanyChanged,
    formatUsd,
    cleanError,
    t,
  } from "$lib/v100/company-api";

  const companyId = $derived(page.params.id ?? "");

  let costs = $state<CostSummary | null>(null);
  let policies = $state<BudgetPolicy[]>([]);
  let staffList = $state<Staff[]>([]);
  let projectList = $state<Project[]>([]);
  let loaded = $state(false);
  let errorMsg = $state("");

  // Dialog state
  let showAddPolicy = $state(false);
  let policyScopeType = $state<"company" | "agent" | "project">("company");
  let policyScopeId = $state<string>("");
  let policyMetric = $state<"usd" | "tokens">("usd");
  let policyWindow = $state<"daily" | "weekly" | "monthly">("monthly");
  let policyAmount = $state<number>(100);
  let policyWarnPct = $state<number>(80);
  let policyAction = $state<"notify" | "pause">("notify");
  let isSavingPolicy = $state(false);
  let dialogError = $state("");

  async function loadData() {
    const id = companyId;
    if (!id) return;
    try {
      const [cData, dash, sData, pData] = await Promise.all([
        companyRpc("costs.summary", { company: id }),
        companyRpc("company.dashboard", { company: id }).catch(() => null),
        companyRpc("staff.list", { company: id }).catch(() => []),
        companyRpc("project.list", { company: id }).catch(() => []),
      ]);

      costs = cData;
      staffList = sData;
      projectList = pData;
      if (dash?.budgets) {
        policies = dash.budgets.map((b: any) => b.policy);
      }
      errorMsg = "";
    } catch (err) {
      errorMsg = cleanError(err);
    } finally {
      loaded = true;
    }
  }

  $effect(() => {
    const id = companyId;
    if (id) {
      loadData();
    }
  });

  onMount(() => {
    const id = companyId;
    if (id) {
      loadData();
      const unsubscribe = onCompanyChanged(id, () => {
        loadData();
      });
      return () => {
        unsubscribe();
      };
    }
  });

  const staffMap = $derived(new Map(staffList.map((s) => [s.id, s.name])));
  const projectMap = $derived(new Map(projectList.map((p) => [p.id, p.name])));

  const modelEntries = $derived(() => {
    if (!costs?.models) return [];
    const entries = Object.entries(costs.models);
    const maxCost = Math.max(...entries.map(([, m]) => m.cost_usd), 0.01);
    return entries.map(([name, data]) => ({
      name,
      ...data,
      pct: (data.cost_usd / maxCost) * 100,
    }));
  });

  const agentEntries = $derived(() => {
    if (!costs?.agents) return [];
    const entries = Object.entries(costs.agents);
    const maxCost = Math.max(...entries.map(([, a]) => a.cost_usd), 0.01);
    return entries.map(([name, data]) => ({
      name,
      ...data,
      pct: (data.cost_usd / maxCost) * 100,
    }));
  });

  function openPolicyDialog() {
    showAddPolicy = true;
    policyScopeType = "company";
    policyScopeId = "";
    policyMetric = "usd";
    policyWindow = "monthly";
    policyAmount = 100;
    policyWarnPct = 80;
    policyAction = "notify";
    dialogError = "";
  }

  function closePolicyDialog() {
    if (isSavingPolicy) return;
    showAddPolicy = false;
  }

  async function handleSavePolicy() {
    if (policyAmount <= 0) return;
    isSavingPolicy = true;
    dialogError = "";
    try {
      const newPolicy: BudgetPolicy = {
        id: `b-${Date.now()}`,
        company_id: companyId,
        scope_type: policyScopeType,
        scope_id: policyScopeId || null,
        metric: policyMetric,
        window: policyWindow,
        amount: policyAmount,
        warn_pct: policyWarnPct,
        action: policyAction,
        enabled: true,
      };

      await companyRpc("budget.save", { company: companyId, policy: newPolicy });
      policies = [...policies, newPolicy];
      showAddPolicy = false;
    } catch (err) {
      dialogError = cleanError(err);
    } finally {
      isSavingPolicy = false;
    }
  }

  function getScopeLabel(p: BudgetPolicy): string {
    if (p.scope_type === "company") return t("company.costs.scope_company", "Entire Organization");
    if (p.scope_type === "agent") return staffMap.get(p.scope_id ?? "") ?? p.scope_id ?? "Agent";
    return projectMap.get(p.scope_id ?? "") ?? p.scope_id ?? "Project";
  }
</script>

<svelte:head>
  <title>{t("company.costs.title", "Costs & Budgets")}</title>
</svelte:head>

<div class="costs-page">
  {#if errorMsg}
    <p class="field-error">{errorMsg}</p>
  {/if}

  {#if !loaded || !costs}
    <div class="stats">
      {#each [0, 1, 2, 3] as _}
        <div class="stat skeleton" style="height: 72px"></div>
      {/each}
    </div>
  {:else}
    <!-- Top Cost Stats -->
    <div class="stats">
      <div class="stat surface-card">
        <span class="stat-label">{t("company.costs.today", "Today")}</span>
        <span class="stat-value">US$ {formatUsd(costs.spend_today_usd)}</span>
        <span class="stat-hint">{t("company.costs.resets_daily", "Resets at midnight UTC")}</span>
      </div>

      <div class="stat surface-card">
        <span class="stat-label">{t("company.costs.this_week", "This Week")}</span>
        <span class="stat-value">US$ {formatUsd(costs.spend_this_week_usd)}</span>
        <span class="stat-hint">{t("company.costs.week_to_date", "Current calendar week")}</span>
      </div>

      <div class="stat surface-card">
        <span class="stat-label">{t("company.costs.this_month", "This Month")}</span>
        <span class="stat-value">US$ {formatUsd(costs.spend_this_month_usd ?? costs.cost_usd)}</span>
        <span class="stat-hint">{t("company.costs.month_to_date", "Current billing cycle")}</span>
      </div>

      <div class="stat surface-card">
        <span class="stat-label">{t("company.costs.total", "All Time")}</span>
        <span class="stat-value">US$ {formatUsd(costs.total_usd)}</span>
        <span class="stat-hint">{t("company.costs.cumulative", "Lifetime workspace token spend")}</span>
      </div>
    </div>

    <!-- Models Breakdown -->
    <div class="section">
      <div class="section-head">
        <h2 class="section-title">{t("company.costs.models_breakdown", "Model Usage Breakdown")}</h2>
      </div>

      <div class="group">
        {#if modelEntries().length === 0}
          <div class="group-row">
            <span class="hint">{t("company.costs.no_model_data", "No model consumption data recorded.")}</span>
          </div>
        {:else}
          {#each modelEntries() as m (m.name)}
            <div class="group-row cost-row">
              <span class="name" title={m.name}>{m.name}</span>
              <div class="track">
                <div class="fill" style="width: {m.pct}%"></div>
              </div>
              <span class="num">{m.tokens.toLocaleString()} tok</span>
              <span class="num money">US$ {formatUsd(m.cost_usd)}</span>
            </div>
          {/each}
        {/if}
      </div>
    </div>

    <!-- Agents Breakdown -->
    <div class="section">
      <div class="section-head">
        <h2 class="section-title">{t("company.costs.agents_breakdown", "Staff & Agent Breakdown")}</h2>
      </div>

      <div class="group">
        {#if agentEntries().length === 0}
          <div class="group-row">
            <span class="hint">{t("company.costs.no_agent_data", "No individual staff consumption data recorded.")}</span>
          </div>
        {:else}
          {#each agentEntries() as a (a.name)}
            <div class="group-row cost-row">
              <span class="name" title={a.name}>{a.name}</span>
              <div class="track">
                <div class="fill" style="width: {a.pct}%"></div>
              </div>
              <span class="num">{a.tokens.toLocaleString()} tok</span>
              <span class="num money">US$ {formatUsd(a.cost_usd)}</span>
            </div>
          {/each}
        {/if}
      </div>
    </div>

    <!-- Budget Policies -->
    <div class="section">
      <div class="section-head">
        <h2 class="section-title">{t("company.costs.budget_policies", "Spend Guardrails & Policies")}</h2>
        <button type="button" class="button active" onclick={openPolicyDialog}>
          {t("company.costs.new_policy", "+ New Budget Policy")}
        </button>
      </div>

      <div class="group">
        {#if policies.length === 0}
          <div class="group-row">
            <span class="hint">{t("company.costs.no_policies", "No budget limits currently configured. Agents can operate without hard spending caps.")}</span>
          </div>
        {:else}
          {#each policies as p (p.id)}
            <div class="group-row policy">
              <div class="group-row-content">
                <div class="group-row-title">
                  <strong>{getScopeLabel(p)}</strong>
                  <span class="tag">{t(`company.overview.window.${p.window}`, p.window)}</span>
                  <span class="tag">{p.action === "pause" ? t("company.costs.action_pause", "Hard Stop") : t("company.costs.action_notify", "Alert Only")}</span>
                </div>
                <div class="dim">
                  {p.metric === "usd" ? `US$ ${formatUsd(p.amount)}` : `${p.amount.toLocaleString()} tokens`} max cap · Warn at {p.warn_pct}%
                </div>
              </div>
              <span class="num">{p.enabled ? t("company.costs.active", "Active") : t("company.costs.disabled", "Disabled")}</span>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>

{#if showAddPolicy}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="dialog-backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && closePolicyDialog()}
    onkeydown={(e) => e.key === "Escape" && closePolicyDialog()}
  >
    <div
      class="dialog-card"
      role="dialog"
      aria-modal="true"
      aria-labelledby="policy-title"
    >
      <h2 class="dialog-title" id="policy-title">
        {t("company.costs.create_policy_title", "New Budget Guardrail")}
      </h2>

      <div class="field">
        <span class="field-label">{t("company.costs.scope", "Scope Target")}</span>
        <div class="segmented">
          <button
            type="button"
            class="segmented-btn"
            class:active={policyScopeType === "company"}
            onclick={() => (policyScopeType = "company")}
          >
            {t("company.costs.scope_company", "Organization")}
          </button>
          <button
            type="button"
            class="segmented-btn"
            class:active={policyScopeType === "agent"}
            onclick={() => (policyScopeType = "agent")}
          >
            {t("company.costs.scope_agent", "Agent")}
          </button>
          <button
            type="button"
            class="segmented-btn"
            class:active={policyScopeType === "project"}
            onclick={() => (policyScopeType = "project")}
          >
            {t("company.costs.scope_project", "Project")}
          </button>
        </div>
      </div>

      {#if policyScopeType === "agent"}
        <label class="field">
          <span class="field-label">{t("company.costs.select_agent", "Target Agent")}</span>
          <select class="input" bind:value={policyScopeId}>
            <option value="">{t("company.costs.choose_agent", "Select staff member...")}</option>
            {#each staffList as s (s.id)}
              <option value={s.id}>{s.name} ({s.role})</option>
            {/each}
          </select>
        </label>
      {:else if policyScopeType === "project"}
        <label class="field">
          <span class="field-label">{t("company.costs.select_project", "Target Project")}</span>
          <select class="input" bind:value={policyScopeId}>
            <option value="">{t("company.costs.choose_project", "Select project...")}</option>
            {#each projectList as pr (pr.id)}
              <option value={pr.id}>{pr.name}</option>
            {/each}
          </select>
        </label>
      {/if}

      <div class="grid">
        <label class="field">
          <span class="field-label">{t("company.costs.metric", "Metric")}</span>
          <select class="input" bind:value={policyMetric}>
            <option value="usd">USD ($)</option>
            <option value="tokens">Tokens</option>
          </select>
        </label>

        <label class="field">
          <span class="field-label">{t("company.costs.window", "Window")}</span>
          <select class="input" bind:value={policyWindow}>
            <option value="daily">{t("company.overview.window.daily", "Daily")}</option>
            <option value="weekly">{t("company.overview.window.weekly", "Weekly")}</option>
            <option value="monthly">{t("company.overview.window.monthly", "Monthly")}</option>
          </select>
        </label>
      </div>

      <div class="grid">
        <label class="field">
          <span class="field-label">{t("company.costs.amount", "Budget Cap Amount")}</span>
          <input class="input" type="number" min="1" bind:value={policyAmount} />
        </label>

        <label class="field">
          <span class="field-label">{t("company.costs.warn_pct", "Warning Alert %")}</span>
          <input class="input" type="number" min="1" max="100" bind:value={policyWarnPct} />
        </label>
      </div>

      <div class="field">
        <span class="field-label">{t("company.costs.action", "Action when Exceeded")}</span>
        <div class="check">
          <label>
            <input
              type="radio"
              name="action"
              value="notify"
              checked={policyAction === "notify"}
              onchange={() => (policyAction = "notify")}
            />
            {t("company.costs.action_notify_label", "Notify & Alert (Non-blocking)")}
          </label>
          <label>
            <input
              type="radio"
              name="action"
              value="pause"
              checked={policyAction === "pause"}
              onchange={() => (policyAction = "pause")}
            />
            {t("company.costs.action_pause_label", "Hard Pause Operations (Freeze Runs)")}
          </label>
        </div>
      </div>

      {#if dialogError}
        <p class="field-error">{dialogError}</p>
      {/if}

      <div class="dialog-actions">
        <button type="button" class="button" disabled={isSavingPolicy} onclick={closePolicyDialog}>
          {t("company.common.cancel", "Cancel")}
        </button>
        <button
          type="button"
          class="button active"
          disabled={isSavingPolicy || policyAmount <= 0}
          onclick={handleSavePolicy}
        >
          {t("company.costs.save_policy", "Create Policy")}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .costs-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 1000px;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .section-title {
    margin: 0;
    font-size: var(--text-md);
    font-weight: 600;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--space-3);
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-3) var(--space-4);
  }
  .stat-label {
    font-size: var(--text-sm);
    color: var(--text-dim);
  }
  .stat-value {
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: 600;
    letter-spacing: var(--track-tight);
    font-variant-numeric: tabular-nums;
  }
  .stat-hint {
    font-size: var(--text-xs);
    color: var(--text-dim);
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .cost-row {
    min-height: var(--row-compact);
    gap: var(--space-3);
    display: flex;
    align-items: center;
  }
  .name {
    width: 180px;
    flex-shrink: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-sm);
  }
  .track {
    flex: 1;
    min-width: 60px;
    height: 6px;
    border-radius: var(--radius-full);
    background: var(--fill-2);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: var(--radius-full);
    background: var(--accent);
  }
  .num {
    flex-shrink: 0;
    min-width: 64px;
    text-align: right;
    font-size: var(--text-xs);
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .money {
    color: var(--text);
    min-width: 84px;
  }
  .hint {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-dim);
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
  }
  .check {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .dim {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-dim);
  }
  .policy .group-row-content {
    gap: 6px;
  }
  .group-row-title {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
</style>
