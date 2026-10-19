<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import {
    type RunRecord,
    type Staff,
    type Issue,
    type AuditEntry,
    companyRpc,
    onCompanyChanged,
    formatMicroUsd,
    formatDuration,
    formatRelativeTime,
    statusColor,
    downloadFile,
    cleanError,
    t,
  } from "$lib/v100/company-api";

  const companyId = $derived(page.params.id ?? "");
  const PAGE_LIMIT = 100;
  const ENTITY_KINDS = [
    "company",
    "staff",
    "issue",
    "approval",
    "budget",
    "goal",
    "project",
    "routine",
    "run",
  ];

  let activeTab = $state<"runs" | "audit">("runs");
  let staffList = $state<Staff[]>([]);
  let issueList = $state<Issue[]>([]);
  let runs = $state<RunRecord[]>([]);
  let agentFilter = $state("");
  let runsLoaded = $state(false);

  // Audit state
  let auditLogs = $state<AuditEntry[]>([]);
  let entityFilter = $state("");
  let actorFilter = $state("");
  let hasMoreAudit = $state(false);
  let auditLoaded = $state(false);
  let verifyResult = $state<{ intact: boolean; text: string } | null>(null);
  let isVerifying = $state(false);
  let expandedSeq = $state<number | null>(null);
  let errorMsg = $state("");
  let now = $state(Date.now());

  async function loadRuns() {
    const id = companyId;
    if (!id) return;
    try {
      const [rList, sList, iList]: [RunRecord[], Staff[], Issue[]] = await Promise.all([
        companyRpc("run.list", {
          company: id,
          staff: agentFilter || null,
          limit: 150,
        }),
        companyRpc("staff.list", { company: id }),
        companyRpc("issue.list", {
          company: id,
          filter: { include_closed: true, limit: 5000 },
        }),
      ]);
      runs = rList;
      staffList = sList;
      issueList = iList;
      errorMsg = "";
    } catch (err) {
      errorMsg = cleanError(err);
    } finally {
      runsLoaded = true;
    }
  }

  async function loadAudit(append = false) {
    const id = companyId;
    if (!id) return;
    const beforeSeq = append && auditLogs.length ? auditLogs[auditLogs.length - 1].seq : null;
    try {
      const res: AuditEntry[] = await companyRpc("audit.list", {
        company: id,
        query: {
          entity_kind: entityFilter || null,
          actor_id: actorFilter || null,
          before_seq: beforeSeq,
          limit: PAGE_LIMIT,
        },
      });

      auditLogs = append ? [...auditLogs, ...res] : res;
      hasMoreAudit = res.length === PAGE_LIMIT;
      if (!staffList.length) {
        staffList = await companyRpc("staff.list", { company: id }).catch(() => []);
      }
      errorMsg = "";
    } catch (err) {
      errorMsg = cleanError(err);
    } finally {
      auditLoaded = true;
    }
  }

  function setTab(tab: "runs" | "audit") {
    activeTab = tab;
    const url = new URL(page.url);
    if (tab === "audit") {
      url.searchParams.set("tab", "audit");
      if (!auditLoaded) loadAudit();
    } else {
      url.searchParams.delete("tab");
      if (!runsLoaded) loadRuns();
    }
    window.history.replaceState(window.history.state, "", url);
  }

  $effect(() => {
    if (page.url.searchParams.get("tab") === "audit") {
      activeTab = "audit";
      loadAudit();
    } else {
      activeTab = "runs";
      loadRuns();
    }
  });

  onMount(() => {
    const id = companyId;
    if (id) {
      const unsubscribe = onCompanyChanged(id, () => {
        if (activeTab === "runs") loadRuns();
        else loadAudit();
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

  const staffMap = $derived(new Map(staffList.map((s) => [s.id, s.name])));
  const issueMap = $derived(new Map(issueList.map((i) => [i.id, i])));
  const runningRunsCount = $derived(
    runs.filter((r) => r.state === "running" || r.state === "queued").length
  );

  const actorOptions = $derived(() => {
    const map = new Map<string, string>();
    for (const e of auditLogs) {
      if (!map.has(e.actor_id)) {
        map.set(
          e.actor_id,
          e.actor_kind === "user"
            ? t("company.common.owner", "Owner")
            : staffMap.get(e.actor_id) || e.actor_id || e.actor_kind
        );
      }
    }
    return [...map.entries()];
  });

  async function handleVerifyChain() {
    isVerifying = true;
    try {
      const res = await companyRpc("audit.verify", { company: companyId });
      verifyResult = {
        intact: res.intact,
        text: res.intact
          ? t("company.activity.chain_ok", { count: res.lines }) || `Cryptographic chain intact (${res.lines} entries)`
          : t("company.activity.chain_broken", { error: res.error }) || "Audit hash mismatch detected",
      };
    } catch (err) {
      verifyResult = {
        intact: false,
        text: cleanError(err),
      };
    } finally {
      isVerifying = false;
    }
  }

  async function handleExportCsv() {
    try {
      const res = await companyRpc("audit.csv", { company: companyId });
      if (res?.csv) {
        downloadFile(`audit-${companyId}.csv`, res.csv, "text/csv");
      }
    } catch (err) {
      errorMsg = cleanError(err);
    }
  }

  function toggleAuditRow(seq: number) {
    expandedSeq = expandedSeq === seq ? null : seq;
  }
</script>

<svelte:head>
  <title>{t("company.activity.title", "Activity & Audit")}</title>
</svelte:head>

<div class="activity-page">
  <div class="bar">
    <div class="segmented" role="tablist" aria-label={t("company.activity.title", "Activity")}>
      <button
        type="button"
        class="segmented-btn"
        role="tab"
        aria-selected={activeTab === "runs"}
        onclick={() => setTab("runs")}
      >
        {t("company.activity.tab_runs", "Active Runs")}
        {#if runningRunsCount > 0}
          <span class="badge">{runningRunsCount}</span>
        {/if}
      </button>
      <button
        type="button"
        class="segmented-btn"
        role="tab"
        aria-selected={activeTab === "audit"}
        onclick={() => setTab("audit")}
      >
        {t("company.activity.tab_audit", "Audit Trail")}
      </button>
    </div>

    <span class="spacer"></span>

    {#if activeTab === "runs"}
      <select
        class="input narrow"
        bind:value={agentFilter}
        aria-label={t("company.activity.agent", "Filter by Agent")}
        onchange={() => loadRuns()}
      >
        <option value="">{t("company.activity.all_agents", "All Agents")}</option>
        {#each staffList as s (s.id)}
          <option value={s.id}>{s.name}</option>
        {/each}
      </select>
    {:else}
      <select
        class="input narrow"
        bind:value={entityFilter}
        aria-label={t("company.activity.entity", "Filter by Entity")}
        onchange={() => loadAudit()}
      >
        <option value="">{t("company.activity.all_entities", "All Entities")}</option>
        {#each ENTITY_KINDS as ek}
          <option value={ek}>{ek}</option>
        {/each}
      </select>

      <select
        class="input narrow"
        bind:value={actorFilter}
        aria-label={t("company.activity.actor", "Filter by Actor")}
        onchange={() => loadAudit()}
      >
        <option value="">{t("company.activity.all_actors", "All Actors")}</option>
        {#each actorOptions() as [actorId, actorLabel]}
          <option value={actorId}>{actorLabel}</option>
        {/each}
      </select>

      <button
        type="button"
        class="btn btn-secondary btn-sm"
        disabled={isVerifying}
        onclick={handleVerifyChain}
      >
        {t("company.activity.verify", "Verify Chain")}
      </button>

      <button
        type="button"
        class="btn btn-secondary btn-sm"
        onclick={handleExportCsv}
      >
        {t("company.activity.export_csv", "Export CSV")}
      </button>
    {/if}
  </div>

  {#if errorMsg}
    <p class="field-error" role="alert">{errorMsg}</p>
  {/if}

  {#if activeTab === "runs"}
    {#if !runsLoaded}
      <div class="skeleton" style="height: 120px"></div>
    {:else if runs.length === 0}
      <div class="empty-state">
        <img
          class="empty-state-art"
          src="/emoji/hourglass_not_done.png"
          alt=""
          width="72"
          height="72"
          draggable="false"
        />
        <p class="empty-state-title">{t("company.activity.no_runs", "No Runs Found")}</p>
        <p class="empty-state-hint">{t("company.activity.no_runs_hint", "Agent work sessions and automated tasks will appear here.")}</p>
      </div>
    {:else}
      <div class="group">
        {#each runs as run (run.id)}
          {@const matchedIssue = run.issue_id ? issueMap.get(run.issue_id) : undefined}
          <div class="group-row run-row">
            <span class="tag {statusColor(run.state)}">
              {t(`company.common.run_state.${run.state}`, run.state)}
            </span>
            <span class="run-agent">{staffMap.get(run.staff_id) ?? run.staff_id}</span>
            <span class="run-task" title={matchedIssue?.title ?? ""}>
              {#if matchedIssue}
                <a
                  class="ident-link"
                  href={`/llm/company/${companyId}/issues/${matchedIssue.identifier}`}
                >
                  <span class="ident">{matchedIssue.identifier}</span>
                  {matchedIssue.title}
                </a>
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
      </div>
    {/if}
  {:else}
    <!-- Audit Tab -->
    {#if verifyResult}
      <div class="notice {verifyResult.intact ? 'notice-success' : 'notice-warning'}" role="status">
        <div class="notice-body">
          <span class="notice-title">
            {verifyResult.intact ? t("company.activity.chain_intact", "Chain Verified") : t("company.activity.chain_bad", "Integrity Warning")}
          </span>
          <span class="notice-text">{verifyResult.text}</span>
        </div>
      </div>
    {/if}

    {#if !auditLoaded}
      <div class="skeleton" style="height: 120px"></div>
    {:else if auditLogs.length === 0}
      <div class="empty-state">
        <img
          class="empty-state-art"
          src="/emoji/books.png"
          alt=""
          width="72"
          height="72"
          draggable="false"
        />
        <p class="empty-state-title">{t("company.activity.no_audit", "No Audit Logs Recorded")}</p>
      </div>
    {:else}
      <div class="group">
        {#each auditLogs as log (log.seq)}
          {@const isExpanded = expandedSeq === log.seq}
          <div class="audit">
            <button
              type="button"
              class="list-row interactive compact line"
              aria-expanded={isExpanded}
              onclick={() => toggleAuditRow(log.seq)}
            >
              <span class="seq">#{log.seq}</span>
              <span class="action">{log.action}</span>
              <span class="ent">{log.entity_kind}</span>
              <span class="who">
                {log.actor_kind === "user" ? t("company.common.owner", "Owner") : staffMap.get(log.actor_id) || log.actor_id}
              </span>
              <span class="chan">{log.channel}</span>
              <time class="when">{formatRelativeTime(log.ts_ms, t, now)}</time>
            </button>

            {#if isExpanded}
              <div class="audit-detail">
                <dl>
                  <dt>{t("company.activity.entity", "Entity")}</dt>
                  <dd class="mono">{log.entity_kind} · {log.entity_id}</dd>
                  <dt>{t("company.activity.actor", "Actor")}</dt>
                  <dd class="mono">{log.actor_kind} · {log.actor_id}</dd>
                  {#if log.run_id}
                    <dt>{t("company.activity.run", "Run ID")}</dt>
                    <dd class="mono">{log.run_id}</dd>
                  {/if}
                  <dt>{t("company.activity.hash", "Cryptographic Hash")}</dt>
                  <dd class="mono">{log.hash}</dd>
                  <dt>{t("company.activity.when", "Timestamp")}</dt>
                  <dd>{new Date(log.ts_ms).toLocaleString()}</dd>
                </dl>
                {#if log.details}
                  <pre class="pre">{JSON.stringify(log.details, null, 2)}</pre>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>

      {#if hasMoreAudit}
        <div class="more">
          <button
            type="button"
            class="btn btn-secondary btn-sm"
            onclick={() => loadAudit(true)}
          >
            {t("company.activity.load_more", "Load More Entries")}
          </button>
        </div>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .activity-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 0;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
  .spacer {
    flex: 1;
  }
  .narrow {
    width: auto;
    max-width: 180px;
  }
  .badge {
    margin-left: 4px;
    padding: 0 6px;
    border-radius: var(--radius-full);
    font-size: 11px;
    background: var(--accent);
    color: var(--accent-text, #fff);
  }
  .line {
    border-radius: 0;
    gap: var(--space-3);
    font-size: var(--text-sm);
    width: 100%;
    display: flex;
    align-items: center;
    text-align: left;
    padding: var(--space-2) var(--space-3);
    background: none;
    border: 0;
    cursor: pointer;
  }
  .line:hover {
    background: var(--fill-1);
  }
  .seq,
  .mono {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .seq {
    min-width: 48px;
    flex-shrink: 0;
  }
  .action {
    flex: 1;
    min-width: 0;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ent,
  .chan {
    flex-shrink: 0;
    padding: 0 6px;
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    color: var(--text-muted);
    background: var(--fill-2);
  }
  .who {
    flex-shrink: 0;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .when {
    flex-shrink: 0;
    min-width: 72px;
    text-align: right;
    font-size: var(--text-xs);
    color: var(--text-dim);
  }
  .audit-detail {
    padding: 0 var(--space-3) var(--space-3);
    background: var(--fill-1);
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px var(--space-3);
    margin: 0 0 var(--space-2);
    font-size: var(--text-sm);
  }
  dt {
    color: var(--text-dim);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .pre {
    margin: 0;
    padding: var(--space-2);
    max-height: 260px;
    overflow: auto;
    border-radius: var(--radius-sm);
    background: var(--fill-2);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    white-space: pre-wrap;
    user-select: text;
  }
  .more {
    display: flex;
    justify-content: center;
    margin-top: var(--space-3);
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
  .ident-link {
    text-decoration: none;
    color: inherit;
  }
  .ident-link:hover {
    text-decoration: underline;
  }
  .ident {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-dim);
    margin-right: 4px;
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
</style>
