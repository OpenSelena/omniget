<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type ProviderConfig,
  } from "$lib/engine/llm-engine-api";
  import {
    CreditCard,
    DollarSign,
    ShieldCheck,
    AlertTriangle,
    RefreshCw,
    TrendingUp,
    Lock,
    ExternalLink,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let providers = $state<ProviderConfig[]>([]);

  interface AccountBalance {
    id: string;
    name: string;
    balance_usd: number;
    used_month_usd: number;
    limit_usd: number;
    tier: string;
    rpm_limit: number;
    tpm_limit: number;
  }

  let accounts = $state<AccountBalance[]>([
    { id: "p-anthropic", name: "Anthropic Commercial", balance_usd: 142.5, used_month_usd: 16.2, limit_usd: 150.0, tier: "Scale Tier 4", rpm_limit: 4000, tpm_limit: 400000 },
    { id: "p-openai", name: "OpenAI Platform", balance_usd: 88.0, used_month_usd: 7.15, limit_usd: 100.0, tier: "Usage Tier 3", rpm_limit: 5000, tpm_limit: 800000 },
    { id: "p-google", name: "Google Cloud Vertex", balance_usd: 250.0, used_month_usd: 0.9, limit_usd: 300.0, tier: "Enterprise PayG", rpm_limit: 2000, tpm_limit: 1000000 },
    { id: "p-deepseek", name: "DeepSeek API", balance_usd: 48.0, used_month_usd: 0.6, limit_usd: 50.0, tier: "Standard Tier", rpm_limit: 1000, tpm_limit: 200000 },
  ]);

  async function loadData(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<ProviderConfig[]>("providers.list");
      providers = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadData();
  });

  const totalUsed = $derived(
    accounts.reduce((acc, a) => acc + a.used_month_usd, 0)
  );
  const totalBalance = $derived(
    accounts.reduce((acc, a) => acc + a.balance_usd, 0)
  );
</script>

<svelte:head>
  <title>{t("llm.accounts.title", "Inference Accounts & Quotas")} - OmniGet</title>
</svelte:head>

<div class="accounts-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <CreditCard size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.accounts.header", "Provider Accounts & Quotas")}</h1>
          <span class="count-pill">{accounts.length} Accounts</span>
        </div>
        <p class="subtitle">
          {t("llm.accounts.sub", "Billing tiers, credit balances, rate limit windows, and spend caps across external providers.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadData(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>
    </div>
  </header>

  <!-- Summary Cards -->
  <div class="summary-cards">
    <div class="kpi-card">
      <span class="kpi-label">Available Prepaid Credits</span>
      <strong class="kpi-val text-success">${totalBalance.toFixed(2)} USD</strong>
      <span class="kpi-sub">Across 4 active provider accounts</span>
    </div>

    <div class="kpi-card">
      <span class="kpi-label">This Month's Incurred Spend</span>
      <strong class="kpi-val text-primary">${totalUsed.toFixed(2)} USD</strong>
      <span class="kpi-sub">Within projected $300 monthly ceiling</span>
    </div>
  </div>

  <!-- Accounts List -->
  <div class="accounts-grid">
    {#each accounts as acc (acc.id)}
      {@const pct = Math.min(100, Math.round((acc.used_month_usd / acc.limit_usd) * 100))}
      <div class="account-card">
        <div class="card-head">
          <div>
            <h3 class="account-name">{acc.name}</h3>
            <span class="tier-badge">{acc.tier}</span>
          </div>

          <div class="balance-box">
            <span class="balance-label">Credit Balance</span>
            <strong class="balance-val">${acc.balance_usd.toFixed(2)}</strong>
          </div>
        </div>

        <div class="spend-section">
          <div class="spend-head">
            <span class="text-muted">Monthly Cap: ${acc.limit_usd.toFixed(0)}</span>
            <strong>${acc.used_month_usd.toFixed(2)} used ({pct}%)</strong>
          </div>
          <div class="progress-wrap">
            <div class="progress-bar" style="width: {pct}%;"></div>
          </div>
        </div>

        <div class="rate-limits-grid">
          <div class="limit-item">
            <span class="limit-label">RPM Window</span>
            <strong>{acc.rpm_limit.toLocaleString()} req/min</strong>
          </div>
          <div class="limit-item">
            <span class="limit-label">TPM Window</span>
            <strong>{acc.tpm_limit.toLocaleString()} tok/min</strong>
          </div>
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .accounts-page {
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

  .subtitle {
    margin: 4px 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
  }

  .summary-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 16px;
  }

  .kpi-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .kpi-label {
    font-size: 0.8rem;
    color: var(--text-muted, #8b949e);
  }

  .kpi-val {
    font-size: 1.6rem;
    font-weight: 800;
  }

  .kpi-sub {
    font-size: 0.75rem;
    color: var(--text-muted, #8b949e);
  }

  .accounts-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 16px;
  }

  .account-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }

  .account-name {
    margin: 0 0 4px 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .tier-badge {
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    font-size: 0.72rem;
    color: var(--primary, #3b82f6);
    font-weight: 600;
  }

  .balance-box {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .balance-label {
    font-size: 0.7rem;
    color: var(--text-muted, #8b949e);
  }

  .balance-val {
    font-size: 1.15rem;
    font-weight: 700;
    color: #10b981;
  }

  .spend-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .spend-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.8rem;
  }

  .progress-wrap {
    height: 6px;
    background: var(--bg-surface, #0d1117);
    border-radius: 999px;
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    background: var(--primary, #3b82f6);
    border-radius: 999px;
  }

  .rate-limits-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    padding: 10px;
    background: var(--bg-surface, #0d1117);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.8rem;
  }

  .limit-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .limit-label {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
  }

  /* Utilities */
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
  }

  .btn-sm {
    padding: 5px 10px;
    font-size: 0.78rem;
  }

  .btn-secondary {
    background: var(--bg-surface-elevated, #21262d);
    border-color: var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
  }

  .text-primary { color: var(--primary, #3b82f6); }
  .text-success { color: #10b981; }
  .text-muted { color: var(--text-muted, #8b949e); }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }
</style>
