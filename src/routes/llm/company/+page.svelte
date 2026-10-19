<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import {
    type Company,
    type Template,
    type ImportPlan,
    companyRpc,
    onCompanyChanged,
    avatarColor,
    cleanError,
    t,
  } from "$lib/v100/company-api";

  interface CompanyRowItem {
    company: Company;
    agents: number;
    running: number;
  }

  let companies = $state<CompanyRowItem[]>([]);
  let loaded = $state(false);
  let errorMsg = $state("");
  let templates = $state<Template[]>([]);

  // Dialog state
  let showCreate = $state(false);
  let selectedTemplate = $state("blank");
  let inputName = $state("");
  let inputMission = $state("");
  let inputWorkspace = $state<string | null>(null);
  let planPreview = $state<ImportPlan | null>(null);
  let packageJson = $state<any>(null);
  let packageFileName = $state("");
  let isSubmitting = $state(false);
  let dialogError = $state("");

  let fileInputEl: HTMLInputElement | null = null;

  async function loadData() {
    try {
      const list: Company[] = await companyRpc("company.list");
      const staffResults = await Promise.all(
        list.map((c) => companyRpc("staff.list", { company: c.id }).catch(() => []))
      );

      companies = list.map((c, i) => {
        const staff = staffResults[i] || [];
        return {
          company: c,
          agents: staff.filter((s: any) => s.status !== "terminated").length,
          running: staff.filter((s: any) => s.status === "running").length,
        };
      });
      errorMsg = "";
    } catch (err) {
      errorMsg = cleanError(err);
    } finally {
      loaded = true;
    }
  }

  onMount(() => {
    loadData();
    companyRpc("company.templates")
      .then((tpls) => {
        if (Array.isArray(tpls)) templates = tpls;
      })
      .catch(() => {});

    const unsubscribe = onCompanyChanged(null, () => {
      loadData();
    });

    return () => {
      unsubscribe();
    };
  });

  const activeCompanies = $derived(
    companies.filter((c) => c.company.status !== "archived")
  );
  const archivedCompanies = $derived(
    companies.filter((c) => c.company.status === "archived")
  );

  function openCreateDialog() {
    showCreate = true;
    selectedTemplate = "blank";
    inputName = "";
    inputMission = "";
    inputWorkspace = null;
    planPreview = null;
    packageJson = null;
    dialogError = "";
  }

  function closeCreateDialog() {
    if (isSubmitting) return;
    showCreate = false;
    planPreview = null;
    packageJson = null;
  }

  function chooseTemplate(id: string) {
    selectedTemplate = id;
    planPreview = null;
    dialogError = "";
  }

  function getCreationOptions() {
    return {
      name: inputName.trim() || null,
      mission: inputMission.trim() || null,
      workspace: inputWorkspace,
    };
  }

  function getSourcePayload() {
    return packageJson ? { package: packageJson } : { template: selectedTemplate };
  }

  async function handleReview() {
    isSubmitting = true;
    dialogError = "";
    try {
      planPreview = await companyRpc("import.preview", {
        ...getSourcePayload(),
        options: getCreationOptions(),
      });
    } catch (err) {
      dialogError = cleanError(err);
    } finally {
      isSubmitting = false;
    }
  }

  async function handleApply() {
    if (!planPreview) return;
    isSubmitting = true;
    dialogError = "";
    try {
      const created = await companyRpc("import.apply", {
        ...getSourcePayload(),
        options: getCreationOptions(),
        plan_hash: planPreview.plan_hash,
      });
      showCreate = false;
      await goto(`/llm/company/${created.id}`);
    } catch (err) {
      dialogError = cleanError(err);
    } finally {
      isSubmitting = false;
    }
  }

  async function handleCreateBlank() {
    if (!inputName.trim()) return;
    isSubmitting = true;
    dialogError = "";
    try {
      const created = await companyRpc("company.create", {
        company: {
          name: inputName.trim(),
          mission: inputMission.trim(),
          workspace: inputWorkspace,
        },
      });
      showCreate = false;
      await goto(`/llm/company/${created.id}`);
    } catch (err) {
      dialogError = cleanError(err);
    } finally {
      isSubmitting = false;
    }
  }

  async function onFileSelected(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;

    openCreateDialog();
    selectedTemplate = "package";
    packageFileName = file.name;

    try {
      packageJson = JSON.parse(await file.text());
    } catch {
      packageJson = null;
      dialogError = t("company.list.import_bad_json", "Invalid JSON file");
      return;
    }
    await handleReview();
  }

  async function pickWorkspaceFolder() {
    try {
      const w = typeof window !== "undefined" ? (window as any) : null;
      if (w?.__TAURI_INTERNALS__?.invoke) {
        const selected = await w.__TAURI_INTERNALS__.invoke("plugin:dialog|open", {
          directory: true,
          multiple: false,
        });
        if (typeof selected === "string") {
          inputWorkspace = selected;
          planPreview = null;
        }
      }
    } catch {
      // Fallback
    }
  }
</script>

<svelte:head>
  <title>{t("company.list.title", "Companies")}</title>
</svelte:head>

<input
  bind:this={fileInputEl}
  type="file"
  accept=".json,application/json"
  hidden
  onchange={onFileSelected}
/>

<div class="page company-list-page">
  <header class="page-head">
    <div>
      <h1 class="page-title">{t("company.list.title", "Companies")}</h1>
      <p class="page-lede">{t("company.list.lede", "Autonomous organizations, multi-agent workspaces, and strategic telemetry.")}</p>
    </div>
    {#if companies.length > 0}
      <div class="head-actions">
        <button type="button" class="button" onclick={() => fileInputEl?.click()}>
          {t("company.list.import", "Import Package")}
        </button>
        <button type="button" class="button active" onclick={openCreateDialog}>
          {t("company.list.new", "+ New Company")}
        </button>
      </div>
    {/if}
  </header>

  {#if errorMsg}
    <p class="field-error">{errorMsg}</p>
  {/if}

  {#if !loaded}
    <div class="group">
      {#each [0, 1] as _}
        <div class="group-row">
          <div class="skeleton" style="height: 36px; flex: 1"></div>
        </div>
      {/each}
    </div>
  {:else if companies.length === 0}
    <div class="empty-state">
      <img
        class="empty-state-art"
        src="/emoji/paperclip.png"
        alt=""
        width="72"
        height="72"
        draggable="false"
      />
      <div class="empty-state-title">{t("company.list.empty_title", "No Companies Found")}</div>
      <p class="empty-state-hint">{t("company.list.empty_hint", "Create your first autonomous organization or import a company workspace blueprint.")}</p>
      <div class="empty-actions">
        <button type="button" class="button active" onclick={openCreateDialog}>
          {t("company.list.empty_cta", "Create Company")}
        </button>
        <button type="button" class="button" onclick={() => fileInputEl?.click()}>
          {t("company.list.import", "Import Package")}
        </button>
      </div>
    </div>
  {:else}
    <div class="group">
      {#each activeCompanies as item (item.company.id)}
        <a class="group-row company-row" href={`/llm/company/${item.company.id}`}>
          <span
            class="tile"
            aria-hidden="true"
            style="background: {avatarColor(item.company.name)}"
          >
            {item.company.issue_prefix}
          </span>
          <span class="group-row-content">
            <span class="group-row-title">{item.company.name}</span>
            <span class="group-row-sub mission">
              {item.company.mission || t("company.list.no_mission", "No mission specified")}
            </span>
          </span>
          <span class="group-row-trailing">
            {#if item.running > 0}
              <span class="tag tag-success">
                <span class="live"></span>
                {t("company.list.running", { count: item.running }) || `${item.running} running`}
              </span>
            {/if}
            <span class="meta">
              {t("company.list.agents", { count: item.agents }) || `${item.agents} agents`}
            </span>
            {#if item.company.status !== "active"}
              <span class="tag {item.company.status === 'paused' ? 'tag-warning' : ''}">
                {t(`company.common.company_status.${item.company.status}`, item.company.status)}
              </span>
            {/if}
            <span class="chevron" aria-hidden="true">›</span>
          </span>
        </a>
      {/each}
    </div>

    {#if archivedCompanies.length > 0}
      <h2 class="group-label archived-label">{t("company.list.archived", "Archived Companies")}</h2>
      <div class="group">
        {#each archivedCompanies as item (item.company.id)}
          <a class="group-row company-row" href={`/llm/company/${item.company.id}`}>
            <span
              class="tile"
              aria-hidden="true"
              style="background: {avatarColor(item.company.name)}"
            >
              {item.company.issue_prefix}
            </span>
            <span class="group-row-content">
              <span class="group-row-title">{item.company.name}</span>
              <span class="group-row-sub mission">{item.company.mission || ""}</span>
            </span>
            <span class="group-row-trailing">
              <span class="tag">{t("company.common.company_status.archived", "Archived")}</span>
              <span class="chevron" aria-hidden="true">›</span>
            </span>
          </a>
        {/each}
      </div>
    {/if}
  {/if}
</div>

{#if showCreate}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="dialog-backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && closeCreateDialog()}
    onkeydown={(e) => e.key === "Escape" && closeCreateDialog()}
  >
    <div
      class="dialog-card create-card"
      role="dialog"
      aria-modal="true"
      aria-labelledby="create-title"
    >
      <h2 class="dialog-title" id="create-title">
        {selectedTemplate === "package"
          ? t("company.list.import_title", { file: packageFileName }) || `Import ${packageFileName}`
          : t("company.list.new_title", "Create Company")}
      </h2>

      {#if selectedTemplate !== "package"}
        <div class="gallery" role="radiogroup" aria-label={t("company.list.templates", "Templates")}>
          <button
            type="button"
            class="tpl"
            role="radio"
            aria-checked={selectedTemplate === "blank"}
            onclick={() => chooseTemplate("blank")}
          >
            <span class="tpl-name">{t("company.list.blank", "Blank Workspace")}</span>
            <span class="tpl-summary">{t("company.list.blank_hint", "Start with an empty organization and assemble staff on demand")}</span>
          </button>
          {#each templates as tpl (tpl.id)}
            <button
              type="button"
              class="tpl"
              role="radio"
              aria-checked={selectedTemplate === tpl.id}
              onclick={() => chooseTemplate(tpl.id)}
            >
              <span class="tpl-name">{tpl.name}</span>
              <span class="tpl-summary">{tpl.summary}</span>
              <span class="tpl-roles">{tpl.roles.join(" · ")}</span>
            </button>
          {/each}
        </div>
      {/if}

      <label class="field">
        <span class="field-label">{t("company.list.name", "Company Name")}</span>
        <input
          class="input"
          type="text"
          placeholder={selectedTemplate === "blank" ? "" : templates.find((x) => x.id === selectedTemplate)?.name ?? ""}
          bind:value={inputName}
          oninput={() => (planPreview = null)}
        />
      </label>

      <label class="field">
        <span class="field-label">{t("company.list.mission", "Mission Statement")}</span>
        <textarea
          class="input"
          rows="2"
          placeholder={t("company.list.mission_placeholder", "What is the strategic objective of this team?")}
          bind:value={inputMission}
          oninput={() => (planPreview = null)}
        ></textarea>
      </label>

      <div class="field">
        <span class="field-label">{t("company.list.workspace", "Workspace Directory")}</span>
        <div class="folder">
          <button type="button" class="button" onclick={pickWorkspaceFolder}>
            {t("company.common.pick_folder", "Select Folder")}
          </button>
          {#if inputWorkspace}
            <code class="path" title={inputWorkspace}>{inputWorkspace}</code>
            <button type="button" class="button" onclick={() => (inputWorkspace = null)}>
              {t("company.common.clear", "Clear")}
            </button>
          {:else}
            <span class="hint">{t("company.list.workspace_hint", "Optional root filesystem folder for workspace tools")}</span>
          {/if}
        </div>
      </div>

      {#if planPreview}
        <div class="plan">
          <div class="plan-head">
            <span class="prefix">{planPreview.issue_prefix}</span>
            <strong>{planPreview.company}</strong>
          </div>
          {#if planPreview.staff.length > 0}
            <div class="field-label">{t("company.list.plan.staff", { count: planPreview.staff.length }) || "Staff"}</div>
            <div class="chips">
              {#each planPreview.staff as staffName}
                <span class="tag">{staffName}</span>
              {/each}
            </div>
          {/if}
          <ul class="counts">
            {#if planPreview.goals > 0}<li><span class="num">{planPreview.goals}</span> {t("company.list.plan.goals", "goals")}</li>{/if}
            {#if planPreview.projects > 0}<li><span class="num">{planPreview.projects}</span> {t("company.list.plan.projects", "projects")}</li>{/if}
            {#if planPreview.routines > 0}<li><span class="num">{planPreview.routines}</span> {t("company.list.plan.routines", "routines")}</li>{/if}
            {#if planPreview.budgets > 0}<li><span class="num">{planPreview.budgets}</span> {t("company.list.plan.budgets", "budgets")}</li>{/if}
            {#if planPreview.issues > 0}<li><span class="num">{planPreview.issues}</span> {t("company.list.plan.issues", "issues")}</li>{/if}
          </ul>
          {#if planPreview.secrets_to_fill.length > 0}
            <div class="notice notice-warning">
              <div class="notice-body">
                <div class="notice-title">{t("company.list.plan.secrets", "Required Secrets")}</div>
                <div class="notice-text">{planPreview.secrets_to_fill.join(", ")}</div>
              </div>
            </div>
          {/if}
          {#each planPreview.warnings as warn}
            <p class="warn">{warn}</p>
          {/each}
        </div>
      {/if}

      {#if dialogError}
        <p class="field-error">{dialogError}</p>
      {/if}

      <div class="dialog-actions">
        <button type="button" class="button" disabled={isSubmitting} onclick={closeCreateDialog}>
          {t("company.common.cancel", "Cancel")}
        </button>
        {#if selectedTemplate === "blank"}
          <button
            type="button"
            class="button active"
            disabled={isSubmitting || !inputName.trim()}
            onclick={handleCreateBlank}
          >
            {t("company.list.create", "Create Organization")}
          </button>
        {:else if planPreview}
          <button
            type="button"
            class="button active"
            disabled={isSubmitting}
            onclick={handleApply}
          >
            {t("company.list.apply", "Apply Blueprint")}
          </button>
        {:else}
          <button
            type="button"
            class="button active"
            disabled={isSubmitting || (selectedTemplate === "package" && !packageJson)}
            onclick={handleReview}
          >
            {t("company.list.review", "Review Blueprint")}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .plan {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--fill-1);
  }
  .plan-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .prefix {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    background: var(--fill-2);
    color: var(--text-muted);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .num {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--text);
  }
  .warn {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--warning);
  }
  .company-list-page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .head-actions,
  .empty-actions {
    display: flex;
    gap: var(--space-2);
  }
  .archived-label {
    margin-top: var(--space-5);
  }
  .company-row {
    text-decoration: none;
    color: inherit;
  }
  @media (hover: hover) {
    .company-row:hover {
      background: var(--fill-1);
    }
  }
  .company-row:focus-visible {
    outline: var(--focus-ring);
    outline-offset: -2px;
  }
  .tile {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    border-radius: 9px;
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.02em;
    font-family: var(--font-mono);
  }
  .mission {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: var(--text-sm);
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .live {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    animation: live 1.6s ease-in-out infinite;
  }
  @keyframes live {
    50% {
      opacity: 0.3;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .live {
      animation: none;
    }
  }
  .create-card {
    width: min(640px, 100%);
    max-height: calc(100vh - 64px);
    overflow-y: auto;
  }
  .gallery {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: var(--space-2);
  }
  .tpl {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: var(--space-3);
    text-align: left;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--fill-1);
    color: inherit;
    font: inherit;
    cursor: pointer;
    box-shadow: inset 0 0 0 var(--hairline) var(--separator);
  }
  @media (hover: hover) {
    .tpl:hover {
      background: var(--fill-2);
    }
  }
  .tpl[aria-checked="true"] {
    box-shadow: inset 0 0 0 2px var(--accent);
    background: var(--accent-soft);
  }
  .tpl:focus-visible {
    outline: var(--focus-ring);
  }
  .tpl-name {
    font-weight: 600;
    font-size: var(--text-sm);
  }
  .tpl-summary {
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.35;
  }
  .tpl-roles {
    font-size: var(--text-xs);
    color: var(--text-dim);
  }
  textarea.input {
    resize: vertical;
    height: auto;
    font-family: inherit;
  }
  .folder {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  .path {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .hint {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-dim);
  }
</style>
