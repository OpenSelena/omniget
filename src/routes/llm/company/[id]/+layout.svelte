<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import { page } from "$app/state";
  import {
    type Company,
    companyRpc,
    onCompanyChanged,
    avatarColor,
    cleanError,
    t,
  } from "$lib/v100/company-api";

  let { children }: { children?: Snippet } = $props();

  const companyId = $derived(page.params.id ?? "");
  const basePath = $derived(`/llm/company/${companyId}`);
  const currentPath = $derived(page.url.pathname.replace(/\/$/, ""));

  let company = $state<Company | null>(null);
  let notFound = $state(false);
  let inboxCount = $state(0);
  let isToggling = $state(false);

  const tabs = [
    { seg: "", key: "overview" },
    { seg: "issues", key: "issues" },
    { seg: "inbox", key: "inbox" },
    { seg: "team", key: "team" },
    { seg: "goals", key: "goals" },
    { seg: "routines", key: "routines" },
    { seg: "costs", key: "costs" },
    { seg: "activity", key: "activity" },
    { seg: "settings", key: "settings" },
  ];

  function tabHref(seg: string) {
    return seg ? `${basePath}/${seg}` : basePath;
  }

  function isTabActive(seg: string) {
    const href = tabHref(seg);
    return seg ? currentPath === href || currentPath.startsWith(href + "/") : currentPath === href;
  }

  async function loadCompany(id: string) {
    if (!id) return;
    try {
      company = await companyRpc("company.get", { company: id });
      notFound = false;
    } catch (err) {
      company = null;
      notFound = true;
      console.warn("Failed to load company:", cleanError(err));
    }

    try {
      const inbox = await companyRpc("inbox.list", { company: id });
      if (inbox) {
        inboxCount =
          (inbox.issues?.length || 0) +
          (inbox.approvals?.length || 0) +
          (inbox.incidents?.length || 0);
      }
    } catch {
      // Ignore inbox load errors
    }
  }

  $effect(() => {
    const id = companyId;
    if (id) {
      loadCompany(id);
    }
  });

  onMount(() => {
    const id = companyId;
    if (id) {
      loadCompany(id);
      const unsubscribe = onCompanyChanged(id, () => {
        loadCompany(id);
      });
      return () => {
        unsubscribe();
      };
    }
  });

  async function toggleStatus(newStatus: "active" | "paused") {
    if (!company) return;
    isToggling = true;
    try {
      company = await companyRpc("company.status", {
        company: company.id,
        status: newStatus,
      });
    } catch (err) {
      console.error("Failed to toggle status:", cleanError(err));
    } finally {
      isToggling = false;
    }
  }
</script>

<svelte:head>
  <title>{company?.name ?? t("company.nav", "Company")}</title>
</svelte:head>

<div class="company-shell">
  {#if notFound}
    <div class="empty-state">
      <img
        class="empty-state-art"
        src="/emoji/package.png"
        alt=""
        width="72"
        height="72"
        draggable="false"
      />
      <div class="empty-state-title">{t("company.common.not_found", "Company Not Found")}</div>
      <a class="button" href="/llm/company">{t("company.common.back_to_list", "Back to Companies")}</a>
    </div>
  {:else}
    <header class="company-head">
      <a class="back" href="/llm/company" aria-label={t("company.common.back_to_list", "Back to Companies")}>‹</a>
      {#if company}
        <span
          class="tile"
          aria-hidden="true"
          style="background: {avatarColor(company.name)}"
        >
          {company.issue_prefix}
        </span>
        <div class="titles">
          <div class="name-line">
            <h1 class="company-name">{company.name}</h1>
            <span class="tag {company.status === 'active' ? 'tag-success' : company.status === 'paused' ? 'tag-warning' : ''}">
              {t(`company.common.company_status.${company.status}`, company.status)}
            </span>
          </div>
          {#if company.mission}
            <p class="mission" title={company.mission}>{company.mission}</p>
          {/if}
          {#if company.status === "paused" && company.pause_reason}
            <p class="reason">{company.pause_reason}</p>
          {/if}
        </div>
        <div class="head-actions">
          {#if company.status === "active"}
            <button
              type="button"
              class="button"
              disabled={isToggling}
              onclick={() => toggleStatus("paused")}
            >
              {t("company.shell.pause", "Pause Operations")}
            </button>
          {:else if company.status === "paused"}
            <button
              type="button"
              class="button active"
              disabled={isToggling}
              onclick={() => toggleStatus("active")}
            >
              {t("company.shell.resume", "Resume Operations")}
            </button>
          {/if}
        </div>
      {:else}
        <div class="skeleton" style="height: 36px; flex: 1; max-width: 360px"></div>
      {/if}
    </header>

    <nav class="company-tabs" aria-label={t("company.shell.tabs", "Company Navigation")}>
      {#each tabs as tab (tab.key)}
        <a
          href={tabHref(tab.seg)}
          aria-current={isTabActive(tab.seg) ? "page" : undefined}
        >
          {t(`company.shell.tab.${tab.key}`, tab.key)}
          {#if tab.key === "inbox" && inboxCount > 0}
            <span class="badge">{inboxCount}</span>
          {/if}
        </a>
      {/each}
    </nav>

    <div class="company-content">
      {#if children}
        {@render children()}
      {/if}
    </div>
  {/if}
</div>

<style>
  .company-shell {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .company-head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5) var(--space-3);
    flex-shrink: 0;
  }
  .back {
    font-size: 22px;
    line-height: 1;
    color: var(--text-dim);
    text-decoration: none;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
  }
  .back:hover {
    background: var(--fill-1);
    color: var(--text);
  }
  .tile {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    border-radius: 10px;
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-mono);
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  .name-line {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  .company-name {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mission {
    margin: 2px 0 0;
    font-size: var(--text-sm);
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .reason {
    margin: 2px 0 0;
    font-size: var(--text-xs);
    color: var(--warning);
  }
  .head-actions {
    display: flex;
    gap: var(--space-2);
    flex-shrink: 0;
  }
  .company-tabs {
    display: flex;
    gap: 4px 18px;
    flex-wrap: wrap;
    padding: 0 var(--space-5);
    border-bottom: var(--hairline) solid var(--separator);
    flex-shrink: 0;
  }
  .company-tabs a {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    text-decoration: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .company-tabs a:hover {
    color: var(--text);
  }
  .company-tabs a[aria-current="page"] {
    color: var(--text);
    font-weight: 600;
    border-bottom-color: var(--accent);
  }
  .company-tabs a:focus-visible {
    outline: var(--focus-ring);
    border-radius: var(--radius-xs);
  }
  .badge {
    padding: 0 6px;
    border-radius: var(--radius-full);
    font-size: 11px;
    font-weight: 600;
    background: var(--accent);
    color: var(--accent-text, #fff);
  }
  .company-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  @media (max-width: 600px) {
    .company-head,
    .company-tabs {
      padding-inline: var(--space-3);
    }
  }
</style>
