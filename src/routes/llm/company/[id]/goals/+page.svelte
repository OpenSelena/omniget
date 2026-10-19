<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import {
    type Goal,
    type Project,
    type Staff,
    type Company,
    companyRpc,
    onCompanyChanged,
    statusColor,
    cleanError,
    t,
  } from "$lib/v100/company-api";

  const companyId = $derived(page.params.id ?? "");

  let company = $state<Company | null>(null);
  let goals = $state<Goal[]>([]);
  let projects = $state<Project[]>([]);
  let staffList = $state<Staff[]>([]);
  let loaded = $state(false);
  let errorMsg = $state("");

  // Goal Dialog
  let showGoalDialog = $state(false);
  let goalTitle = $state("");
  let goalDesc = $state("");
  let goalLevel = $state<"objective" | "milestone" | "task">("milestone");
  let goalParentId = $state<string>("");
  let goalOwnerId = $state<string>("");
  let isSavingGoal = $state(false);
  let goalDialogError = $state("");

  // Project Dialog
  let showProjDialog = $state(false);
  let projName = $state("");
  let projKey = $state("");
  let projDesc = $state("");
  let projColor = $state("var(--blue)");
  let projFolder = $state<string | null>(null);
  let isSavingProj = $state(false);
  let projDialogError = $state("");

  const PROJECT_COLORS = [
    "var(--blue)",
    "var(--green)",
    "var(--orange)",
    "var(--purple)",
    "var(--teal)",
    "var(--red)",
  ];

  async function loadData() {
    const id = companyId;
    if (!id) return;
    try {
      const [c, g, p, s] = await Promise.all([
        companyRpc("company.get", { company: id }),
        companyRpc("goal.list", { company: id }),
        companyRpc("project.list", { company: id }),
        companyRpc("staff.list", { company: id }),
      ]);
      company = c;
      goals = g;
      projects = p;
      staffList = s;
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

  function openNewGoal() {
    showGoalDialog = true;
    goalTitle = "";
    goalDesc = "";
    goalLevel = "milestone";
    goalParentId = "";
    goalOwnerId = "";
    goalDialogError = "";
  }

  function closeGoalDialog() {
    if (isSavingGoal) return;
    showGoalDialog = false;
  }

  async function handleSaveGoal() {
    if (!goalTitle.trim()) return;
    isSavingGoal = true;
    goalDialogError = "";
    try {
      const newGoal = await companyRpc("goal.create", {
        company: companyId,
        goal: {
          title: goalTitle.trim(),
          description: goalDesc.trim(),
          level: goalLevel,
          parent_goal_id: goalParentId || null,
          owner_staff_id: goalOwnerId || null,
        },
      });
      goals = [...goals, newGoal];
      showGoalDialog = false;
    } catch (err) {
      goalDialogError = cleanError(err);
    } finally {
      isSavingGoal = false;
    }
  }

  async function toggleGoalStatus(g: Goal) {
    const newStatus = g.status === "achieved" ? "active" : "achieved";
    try {
      await companyRpc("goal.update", {
        id: g.id,
        patch: { status: newStatus },
      });
      g.status = newStatus;
    } catch (err) {
      errorMsg = cleanError(err);
    }
  }

  async function handleDeleteGoal(id: string) {
    try {
      await companyRpc("goal.delete", { id });
      goals = goals.filter((g) => g.id !== id);
    } catch (err) {
      errorMsg = cleanError(err);
    }
  }

  function openNewProject() {
    showProjDialog = true;
    projName = "";
    projKey = "";
    projDesc = "";
    projColor = "var(--blue)";
    projFolder = null;
    projDialogError = "";
  }

  function closeProjDialog() {
    if (isSavingProj) return;
    showProjDialog = false;
  }

  async function pickProjectFolder() {
    try {
      const w = typeof window !== "undefined" ? (window as any) : null;
      if (w?.__TAURI_INTERNALS__?.invoke) {
        const selected = await w.__TAURI_INTERNALS__.invoke("plugin:dialog|open", {
          directory: true,
          multiple: false,
        });
        if (typeof selected === "string") {
          projFolder = selected;
        }
      }
    } catch {
      // Fallback
    }
  }

  async function handleSaveProject() {
    if (!projName.trim()) return;
    isSavingProj = true;
    projDialogError = "";
    try {
      const newProj = await companyRpc("project.create", {
        company: companyId,
        project: {
          name: projName.trim(),
          key: (projKey.trim() || projName.slice(0, 4)).toUpperCase(),
          description: projDesc.trim(),
          color: projColor,
          workspace_folder: projFolder,
        },
      });
      projects = [...projects, newProj];
      showProjDialog = false;
    } catch (err) {
      projDialogError = cleanError(err);
    } finally {
      isSavingProj = false;
    }
  }
</script>

<svelte:head>
  <title>{t("company.goals.title", "Strategic Goals & Projects")}</title>
</svelte:head>

<div class="goals-page">
  {#if errorMsg}
    <p class="field-error">{errorMsg}</p>
  {/if}

  {#if company?.mission}
    <div class="group">
      <div class="goal-row mission-row">
        <span class="tag tag-success">{t("company.goals.mission_tag", "Mission")}</span>
        <span class="goal-title">{company.mission}</span>
      </div>
    </div>
  {/if}

  <!-- Strategic Objectives & Milestones Section -->
  <section class="section">
    <div class="section-head">
      <h2 class="section-title">{t("company.goals.objectives", "Objectives & Milestones")}</h2>
      <button type="button" class="button active" onclick={openNewGoal}>
        {t("company.goals.new_goal", "+ New Goal")}
      </button>
    </div>

    <div class="group">
      {#if !loaded}
        <div class="skeleton" style="height: 80px"></div>
      {:else if goals.length === 0}
        <div class="group-row">
          <span class="hint">{t("company.goals.no_goals", "No strategic objectives recorded. Define company milestones to align staff.")}</span>
        </div>
      {:else}
        {#each goals as g (g.id)}
          <div class="goal-row" class:done={g.status === "achieved"}>
            <span class="tag">{t(`company.common.goal_level.${g.level}`, g.level)}</span>
            <span class="goal-title" title={g.title}>{g.title}</span>
            {#if g.owner_staff_id}
              <span class="dim">{staffMap.get(g.owner_staff_id) ?? ""}</span>
            {/if}
            <span class="tag {statusColor(g.status)}">
              {t(`company.common.status.${g.status}`, g.status)}
            </span>
            <div class="row-actions">
              <button
                type="button"
                class="link"
                onclick={() => toggleGoalStatus(g)}
              >
                {g.status === "achieved" ? t("company.goals.reopen", "Reopen") : t("company.goals.mark_achieved", "Mark Achieved")}
              </button>
              <button
                type="button"
                class="link danger"
                onclick={() => handleDeleteGoal(g.id)}
              >
                {t("company.common.delete", "Delete")}
              </button>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </section>

  <!-- Projects Section -->
  <section class="section">
    <div class="section-head">
      <h2 class="section-title">{t("company.goals.projects", "Workspace Projects")}</h2>
      <button type="button" class="button active" onclick={openNewProject}>
        {t("company.goals.new_project", "+ New Project")}
      </button>
    </div>

    <div class="group">
      {#if !loaded}
        <div class="skeleton" style="height: 80px"></div>
      {:else if projects.length === 0}
        <div class="group-row">
          <span class="hint">{t("company.goals.no_projects", "No sub-projects configured.")}</span>
        </div>
      {:else}
        {#each projects as pr (pr.id)}
          <div class="goal-row project-row">
            <span class="swatch" style="background: {pr.color || 'var(--blue)'}"></span>
            <code>{pr.key}</code>
            <span class="goal-title">{pr.name}</span>
            {#if pr.workspace_folder}
              <code class="dim">{pr.workspace_folder}</code>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </section>
</div>

<!-- New Goal Modal -->
{#if showGoalDialog}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="dialog-backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && closeGoalDialog()}
    onkeydown={(e) => e.key === "Escape" && closeGoalDialog()}
  >
    <div
      class="dialog-card wide"
      role="dialog"
      aria-modal="true"
      aria-labelledby="goal-dialog-title"
    >
      <h2 class="dialog-title" id="goal-dialog-title">
        {t("company.goals.new_goal_title", "Create Goal or Milestone")}
      </h2>

      <div class="dialog-form">
        <div class="field">
          <span class="field-label">{t("company.goals.level", "Goal Level")}</span>
          <div class="segmented">
            <button
              type="button"
              class="segmented-btn"
              class:active={goalLevel === "objective"}
              onclick={() => (goalLevel = "objective")}
            >
              Objective
            </button>
            <button
              type="button"
              class="segmented-btn"
              class:active={goalLevel === "milestone"}
              onclick={() => (goalLevel = "milestone")}
            >
              Milestone
            </button>
            <button
              type="button"
              class="segmented-btn"
              class:active={goalLevel === "task"}
              onclick={() => (goalLevel = "task")}
            >
              Task
            </button>
          </div>
        </div>

        {#if goalLevel !== "objective" && goals.length > 0}
          <label class="field">
            <span class="field-label">{t("company.goals.parent_goal", "Parent Objective")}</span>
            <select class="input" bind:value={goalParentId}>
              <option value="">{t("company.goals.none", "None (Top Level)")}</option>
              {#each goals.filter((x) => x.level === "objective") as parent (parent.id)}
                <option value={parent.id}>{parent.title}</option>
              {/each}
            </select>
          </label>
        {/if}

        <label class="field">
          <span class="field-label">{t("company.goals.goal_title", "Title")}</span>
          <input class="input" type="text" bind:value={goalTitle} />
        </label>

        <label class="field">
          <span class="field-label">{t("company.goals.description", "Description")}</span>
          <textarea class="input" rows="2" bind:value={goalDesc}></textarea>
        </label>

        <label class="field">
          <span class="field-label">{t("company.goals.owner", "Owner Staff")}</span>
          <select class="input" bind:value={goalOwnerId}>
            <option value="">{t("company.goals.unassigned", "Unassigned")}</option>
            {#each staffList as s (s.id)}
              <option value={s.id}>{s.name} ({s.role})</option>
            {/each}
          </select>
        </label>

        {#if goalDialogError}
          <p class="field-error">{goalDialogError}</p>
        {/if}

        <div class="dialog-actions">
          <button type="button" class="button" disabled={isSavingGoal} onclick={closeGoalDialog}>
            {t("company.common.cancel", "Cancel")}
          </button>
          <button
            type="button"
            class="button active"
            disabled={isSavingGoal || !goalTitle.trim()}
            onclick={handleSaveGoal}
          >
            {t("company.goals.save_goal", "Save Goal")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- New Project Modal -->
{#if showProjDialog}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="dialog-backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && closeProjDialog()}
    onkeydown={(e) => e.key === "Escape" && closeProjDialog()}
  >
    <div
      class="dialog-card wide"
      role="dialog"
      aria-modal="true"
      aria-labelledby="proj-dialog-title"
    >
      <h2 class="dialog-title" id="proj-dialog-title">
        {t("company.goals.new_project_title", "Create Workspace Project")}
      </h2>

      <div class="dialog-form">
        <label class="field">
          <span class="field-label">{t("company.goals.proj_name", "Project Name")}</span>
          <input class="input" type="text" bind:value={projName} />
        </label>

        <label class="field">
          <span class="field-label">{t("company.goals.proj_key", "Key Prefix")}</span>
          <input
            class="input"
            type="text"
            placeholder={projName.slice(0, 4).toUpperCase()}
            bind:value={projKey}
          />
        </label>

        <label class="field">
          <span class="field-label">{t("company.goals.proj_desc", "Description")}</span>
          <textarea class="input" rows="2" bind:value={projDesc}></textarea>
        </label>

        <div class="field">
          <span class="field-label">{t("company.goals.color", "Project Color")}</span>
          <div class="colors">
            {#each PROJECT_COLORS as c}
              <button
                type="button"
                class="color"
                style="background: {c}"
                aria-checked={projColor === c}
                onclick={() => (projColor = c)}
              ></button>
            {/each}
          </div>
        </div>

        <div class="field">
          <span class="field-label">{t("company.goals.workspace_folder", "Project Workspace Folder")}</span>
          <div class="folder">
            <button type="button" class="button" onclick={pickProjectFolder}>
              {t("company.common.pick_folder", "Select Folder")}
            </button>
            {#if projFolder}
              <code class="path" title={projFolder}>{projFolder}</code>
              <button type="button" class="button" onclick={() => (projFolder = null)}>
                {t("company.common.clear", "Clear")}
              </button>
            {/if}
          </div>
        </div>

        {#if projDialogError}
          <p class="field-error">{projDialogError}</p>
        {/if}

        <div class="dialog-actions">
          <button type="button" class="button" disabled={isSavingProj} onclick={closeProjDialog}>
            {t("company.common.cancel", "Cancel")}
          </button>
          <button
            type="button"
            class="button active"
            disabled={isSavingProj || !projName.trim()}
            onclick={handleSaveProject}
          >
            {t("company.goals.save_project", "Create Project")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .goals-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
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
  .goal-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 40px;
    padding: 6px var(--space-3);
    font-size: var(--text-sm);
    position: relative;
  }
  .goal-row + .goal-row {
    border-top: var(--hairline) solid var(--separator);
  }
  .mission-row {
    background: var(--fill-1);
    font-weight: 600;
  }
  .goal-row.done .goal-title {
    color: var(--text-dim);
    text-decoration: line-through;
  }
  .goal-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-actions {
    display: flex;
    gap: var(--space-3);
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .goal-row:hover .row-actions,
  .goal-row:focus-within .row-actions {
    opacity: 1;
  }
  @media (hover: none) {
    .row-actions {
      opacity: 1;
    }
  }
  .link {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    font-size: var(--text-xs);
    color: var(--accent-hi);
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .link.danger {
    color: var(--red);
  }
  .dim {
    font-size: var(--text-xs);
    color: var(--text-dim);
    flex-shrink: 0;
  }
  .hint {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-dim);
  }
  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 4px;
    flex-shrink: 0;
  }
  .project-row code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }
  .dialog-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .wide {
    width: min(560px, 100%);
    max-height: calc(100vh - 64px);
    overflow-y: auto;
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
  .colors {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    padding-top: 4px;
  }
  .color {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 0;
    cursor: pointer;
  }
  .color[aria-checked="true"] {
    box-shadow: 0 0 0 2px var(--surface), 0 0 0 4px var(--accent);
  }
  .color:focus-visible {
    outline: var(--focus-ring);
  }
</style>
