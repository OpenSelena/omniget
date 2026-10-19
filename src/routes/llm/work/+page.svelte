<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspaceRpc,
    t,
    type WorkItem,
  } from "$lib/engine/workspace-api";
  import {
    Inbox,
    CheckCircle2,
    AlertTriangle,
    ShieldAlert,
    ExternalLink,
    ChevronRight,
    Compass,
    Sparkles,
  } from "lucide-svelte";

  let loading = $state(true);
  let workItems = $state<WorkItem[]>([]);

  async function loadWork() {
    loading = true;
    try {
      const res = await workspaceRpc<WorkItem[]>("work.list");
      workItems = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadWork();
  });

  function resolveItem(id: string) {
    workItems = workItems.filter((w) => w.id !== id);
  }
</script>

<div class="work-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("work.title", "Work Hub & Action Inbox")}</h1>
      <p class="page-subtitle">
        {t("work.subtitle", "Prioritized decisions, pending approvals, and items waiting for your attention.")}
      </p>
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>{t("common.loading", "Checking active work items...")}</p>
    </div>
  {:else if workItems.length === 0}
    <div class="empty-state">
      <CheckCircle2 size={52} class="text-green-400 mb-2" />
      <h2>{t("llm.work.nothing_waits", "Nothing waits for your attention")}</h2>
      <p class="text-muted">
        All autonomous agent loops, background routines, and inference queues are operating normally.
      </p>
    </div>
  {:else}
    <div class="work-grid">
      {#each workItems as item (item.id)}
        <article class="work-card {item.severity}">
          <div class="card-left">
            <div class="severity-icon">
              {#if item.severity === "action_required" || item.severity === "warning"}
                <AlertTriangle size={20} class="text-orange-400" />
              {:else if item.severity === "error"}
                <ShieldAlert size={20} class="text-red-400" />
              {:else}
                <Sparkles size={20} class="text-blue-400" />
              {/if}
            </div>

            <div class="work-info">
              <div class="badge-line">
                <span class="category-pill">{item.category.replace("_", " ")}</span>
                <span class="sev-pill {item.severity}">{item.severity.replace("_", " ")}</span>
              </div>
              <h3 class="work-title">{item.title}</h3>
              <p class="work-desc">{item.description}</p>
            </div>
          </div>

          <div class="card-right">
            <a href={item.target_route} class="btn-navigate">
              Open Workspace
              <ChevronRight size={15} class="ml-1 inline" />
            </a>
            <button
              type="button"
              class="btn-dismiss"
              onclick={() => resolveItem(item.id)}
            >
              Dismiss
            </button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</div>

<style>
  .work-page {
    padding: 24px 32px;
    max-width: 1200px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
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

  .work-grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .work-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    gap: 20px;
  }

  .work-card.action_required {
    border-color: rgba(240, 136, 62, 0.4);
    background: rgba(240, 136, 62, 0.03);
  }

  .work-card.error {
    border-color: rgba(248, 81, 73, 0.4);
    background: rgba(248, 81, 73, 0.03);
  }

  .card-left {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    flex: 1;
    min-width: 0;
  }

  .severity-icon {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    background: var(--fill-1, rgba(255, 255, 255, 0.05));
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .badge-line {
    display: flex;
    gap: 8px;
    margin-bottom: 6px;
  }

  .category-pill {
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    background: var(--fill-2, rgba(255, 255, 255, 0.1));
    color: var(--text-muted, #8b949e);
  }

  .sev-pill {
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .sev-pill.action_required, .sev-pill.warning {
    background: rgba(240, 136, 62, 0.2);
    color: #f0883e;
  }

  .sev-pill.error {
    background: rgba(248, 81, 73, 0.2);
    color: #f85149;
  }

  .sev-pill.info {
    background: rgba(56, 139, 253, 0.2);
    color: #58a6ff;
  }

  .work-title {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 4px;
  }

  .work-desc {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 0;
  }

  .card-right {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .btn-navigate {
    display: inline-flex;
    align-items: center;
    padding: 8px 16px;
    border-radius: 8px;
    background: var(--cta, #1f6feb);
    color: #fff;
    font-size: 13px;
    font-weight: 500;
    text-decoration: none;
    transition: background 0.15s ease;
  }

  .btn-navigate:hover {
    background: #388bfd;
  }

  .btn-dismiss {
    padding: 8px 14px;
    border-radius: 8px;
    background: var(--fill-1, rgba(255, 255, 255, 0.06));
    border: none;
    color: var(--text-muted, #8b949e);
    font-size: 13px;
    cursor: pointer;
  }

  .btn-dismiss:hover {
    color: var(--text, #fff);
    background: rgba(255, 255, 255, 0.12);
  }

  .loading-state, .empty-state {
    text-align: center;
    padding: 80px 20px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
  }

  .empty-state h2 {
    font-size: 20px;
    font-weight: 700;
    margin: 0 0 8px;
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

  @media (max-width: 768px) {
    .work-card {
      flex-direction: column;
      align-items: stretch;
    }
    .card-right {
      justify-content: flex-end;
    }
  }
</style>
