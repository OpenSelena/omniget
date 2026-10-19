<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import {
    companyRpc,
    t,
    avatarColor,
    getInitials,
    formatRelativeTime,
    formatMicroUsd,
    formatDuration,
    statusColor,
    ISSUE_TRANSITIONS,
    type Issue,
    type Staff,
    type Goal,
    type Project,
    type RunRecord,
  } from "$lib/engine/company-api";
  import {
    ArrowLeft,
    CheckCircle2,
    Clock,
    AlertCircle,
    User,
    Tag,
    Trash2,
    Send,
    Play,
    Terminal,
    DollarSign,
    RefreshCw,
    X,
    Plus,
    Check,
  } from "lucide-svelte";

  const companyId = $derived($page.params.id || "omniget-core");
  const issueParam = $derived($page.params.issue || "");

  let loading = $state(true);
  let error = $state<string | null>(null);

  let issue = $state<Issue | null>(null);
  let staffList = $state<Staff[]>([]);
  let goals = $state<Goal[]>([]);
  let projects = $state<Project[]>([]);
  let runs = $state<RunRecord[]>([]);
  let comments = $state<{ id: string; author: string; text: string; created_at: number }[]>([]);

  // Editing state
  let editTitle = $state(false);
  let titleDraft = $state("");
  let editDesc = $state(false);
  let descDraft = $state("");

  // New Comment
  let commentDraft = $state("");
  let submittingComment = $state(false);

  // New Tag
  let newTagInput = $state("");
  let addingTag = $state(false);

  // Delete
  let confirmDelete = $state(false);
  let deleting = $state(false);

  async function loadIssueData() {
    loading = true;
    error = null;
    try {
      const [resIssue, resStaff, resGoals, resProjects, resRuns] = await Promise.all([
        companyRpc<Issue>("issue.get", { company: companyId, identifier: issueParam, id: issueParam }),
        companyRpc<Staff[]>("staff.list", { company: companyId }),
        companyRpc<Goal[]>("goal.list", { company: companyId }),
        companyRpc<Project[]>("project.list", { company: companyId }),
        companyRpc<RunRecord[]>("run.list", { company: companyId }),
      ]);

      issue = resIssue;
      staffList = Array.isArray(resStaff) ? resStaff : [];
      goals = Array.isArray(resGoals) ? resGoals : [];
      projects = Array.isArray(resProjects) ? resProjects : [];

      if (issue) {
        titleDraft = issue.title;
        descDraft = issue.description ?? "";
        // Load comments
        const resComments = await companyRpc<any[]>("issue.comments", { issue_id: issue.id });
        comments = Array.isArray(resComments) ? resComments : [];

        // Filter associated runs
        const allRuns = Array.isArray(resRuns) ? resRuns : [];
        runs = allRuns.filter((r) => r.issue_id === issue?.id);
      }
    } catch (err: any) {
      error = err?.message || "Failed to load issue details";
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadIssueData();
  });

  const staffMap = $derived(new Map(staffList.map((s) => [s.id, s])));

  async function patchIssue(patch: Partial<Issue>) {
    if (!issue) return;
    Object.assign(issue, patch);
    try {
      await companyRpc("issue.update", {
        company: companyId,
        id: issue.id,
        identifier: issue.identifier,
        patch,
      });
    } catch (err: any) {
      console.error("Failed to update issue:", err);
    }
  }

  function saveTitle() {
    if (!titleDraft.trim() || !issue) return;
    patchIssue({ title: titleDraft.trim() });
    editTitle = false;
  }

  function saveDescription() {
    if (!issue) return;
    patchIssue({ description: descDraft.trim() });
    editDesc = false;
  }

  async function handleAddComment(e: SubmitEvent) {
    e.preventDefault();
    if (!commentDraft.trim() || !issue) return;
    submittingComment = true;
    try {
      const newC = await companyRpc("issue.add_comment", {
        issue_id: issue.id,
        author: "User",
        text: commentDraft.trim(),
      });
      if (newC) {
        comments.push(newC);
        commentDraft = "";
      }
    } catch (err: any) {
      console.error("Failed to post comment:", err);
    } finally {
      submittingComment = false;
    }
  }

  function addTag() {
    if (!newTagInput.trim() || !issue) return;
    const tag = newTagInput.trim().toLowerCase();
    const currentTags = issue.tags ?? [];
    if (!currentTags.includes(tag)) {
      const nextTags = [...currentTags, tag];
      patchIssue({ tags: nextTags });
    }
    newTagInput = "";
    addingTag = false;
  }

  function removeTag(tagToRemove: string) {
    if (!issue || !issue.tags) return;
    const nextTags = issue.tags.filter((t) => t !== tagToRemove);
    patchIssue({ tags: nextTags });
  }

  async function handleDeleteIssue() {
    if (!issue) return;
    deleting = true;
    try {
      await companyRpc("issue.delete", { company: companyId, id: issue.id });
      goto(`/llm/company/${companyId}/issues`);
    } catch (err: any) {
      alert("Failed to delete issue: " + (err?.message || err));
      deleting = false;
    }
  }

  const allowedTransitions = $derived(
    issue ? (ISSUE_TRANSITIONS[issue.status] ?? []) : []
  );
</script>

<svelte:head>
  <title>{issue ? `${issue.identifier}: ${issue.title}` : "Issue Details"} - OmniGet</title>
</svelte:head>

<div class="issue-detail-page">
  <!-- Top Navigation & Breadcrumb -->
  <div class="nav-bar">
    <a href="/llm/company/{companyId}/issues" class="back-link">
      <ArrowLeft size={16} />
      <span>{t("company.issues.back_to_list", "All Issues")}</span>
    </a>

    {#if issue}
      <div class="badge-row">
        <span class="id-pill">{issue.identifier}</span>
        <span class="status-pill status-{statusColor(issue.status)}">{issue.status}</span>
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading issue details...")}</p>
    </div>
  {:else if error || !issue}
    <div class="error-card">
      <AlertCircle size={20} class="text-danger" />
      <div>
        <h4>{t("company.issues.not_found", "Issue Not Found")}</h4>
        <p>{error || "The requested issue does not exist in this organization."}</p>
      </div>
      <a href="/llm/company/{companyId}/issues" class="btn btn-secondary btn-sm">
        {t("company.issues.back_to_list", "Back to Issues")}
      </a>
    </div>
  {:else}
    <div class="content-layout">
      <!-- Main Content Column -->
      <div class="main-column">
        <!-- Title Header -->
        <div class="title-block">
          {#if editTitle}
            <div class="edit-title-row">
              <input
                type="text"
                bind:value={titleDraft}
                class="title-input"
                onkeydown={(e) => e.key === "Enter" && saveTitle()}
              />
              <button class="btn btn-primary btn-sm" onclick={saveTitle}>
                <Check size={14} />
              </button>
              <button class="btn btn-secondary btn-sm" onclick={() => (editTitle = false)}>
                <X size={14} />
              </button>
            </div>
          {:else}
            <h1 class="issue-title" onclick={() => (editTitle = true)} title="Click to edit title">
              {issue.title}
            </h1>
          {/if}

          <!-- Lifecycle Transition Buttons -->
          <div class="transitions-bar">
            <span class="trans-label">{t("company.issues.transition_to", "Transition to:")}</span>
            <div class="trans-btns">
              {#each allowedTransitions as targetStatus}
                <button
                  class="btn btn-secondary btn-sm status-trans-btn"
                  onclick={() => patchIssue({ status: targetStatus as any })}
                >
                  <span class="dot dot-{statusColor(targetStatus)}"></span>
                  <span class="capitalize">{targetStatus.replace("_", " ")}</span>
                </button>
              {/each}
            </div>
          </div>
        </div>

        <!-- Description Block -->
        <div class="section-card">
          <div class="section-head">
            <h3>{t("company.issues.desc_heading", "Description")}</h3>
            {#if !editDesc}
              <button class="link-btn" onclick={() => (editDesc = true)}>
                {t("common.edit", "Edit")}
              </button>
            {/if}
          </div>

          {#if editDesc}
            <div class="edit-desc-wrap">
              <textarea
                rows="6"
                bind:value={descDraft}
                class="desc-textarea"
                placeholder={t("company.issues.desc_placeholder", "Add details, reproduction steps, or agent prompt requirements...")}
              ></textarea>
              <div class="edit-actions">
                <button class="btn btn-secondary btn-sm" onclick={() => (editDesc = false)}>
                  {t("common.cancel", "Cancel")}
                </button>
                <button class="btn btn-primary btn-sm" onclick={saveDescription}>
                  {t("common.save", "Save Description")}
                </button>
              </div>
            </div>
          {:else if issue.description}
            <div class="desc-content">
              <p>{issue.description}</p>
            </div>
          {:else}
            <p class="empty-text" onclick={() => (editDesc = true)}>
              {t("company.issues.no_desc", "No description provided. Click here to add one.")}
            </p>
          {/if}
        </div>

        <!-- Associated Runs Block -->
        <div class="section-card">
          <div class="section-head">
            <h3>{t("company.issues.associated_runs", "Agent Executions")}</h3>
            <span class="badge badge-secondary">{runs.length}</span>
          </div>

          {#if runs.length > 0}
            <div class="runs-list">
              {#each runs as run (run.id)}
                {@const staff = staffMap.get(run.staff_id)}
                <div class="run-row">
                  <div class="run-left">
                    <Terminal size={15} class="text-primary" />
                    <span class="run-id">{run.id}</span>
                    <span class="run-agent">{staff?.name ?? run.staff_id}</span>
                  </div>
                  <div class="run-right">
                    <span class="status-pill status-{statusColor(run.state)}">{run.state}</span>
                    <span class="run-cost">${formatMicroUsd(run.cost_micros)}</span>
                    <span class="run-time">{formatDuration(run.started_ms, run.finished_ms)}</span>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <p class="empty-text">
              {t("company.issues.no_runs", "No agent runs currently linked to this issue.")}
            </p>
          {/if}
        </div>

        <!-- Comments & Activity Thread -->
        <div class="section-card">
          <div class="section-head">
            <h3>{t("company.issues.activity_heading", "Activity & Comments")}</h3>
            <span class="badge badge-secondary">{comments.length}</span>
          </div>

          <div class="comments-thread">
            {#each comments as c (c.id)}
              <div class="comment-item">
                <div
                  class="comment-avatar"
                  style="background: {avatarColor(c.author)};"
                >
                  {getInitials(c.author)}
                </div>
                <div class="comment-bubble">
                  <div class="comment-meta">
                    <strong>{c.author}</strong>
                    <span class="comment-time">{formatRelativeTime(c.created_at, t)}</span>
                  </div>
                  <p class="comment-body">{c.text}</p>
                </div>
              </div>
            {/each}

            {#if comments.length === 0}
              <p class="empty-text">{t("company.issues.no_comments", "No comments yet.")}</p>
            {/if}
          </div>

          <!-- Add Comment Box -->
          <form onsubmit={handleAddComment} class="comment-input-form">
            <textarea
              rows="3"
              placeholder={t("company.issues.comment_placeholder", "Leave a comment or prompt instruction for the agent...")}
              bind:value={commentDraft}
            ></textarea>
            <div class="comment-footer">
              <button
                type="submit"
                class="btn btn-primary btn-sm"
                disabled={submittingComment || !commentDraft.trim()}
              >
                <Send size={13} />
                <span>{submittingComment ? t("common.sending", "Posting...") : t("company.issues.post_comment", "Post Comment")}</span>
              </button>
            </div>
          </form>
        </div>
      </div>

      <!-- Sidebar Column -->
      <aside class="sidebar-column">
        <!-- Properties Panel -->
        <div class="panel-card">
          <h4>{t("company.issues.properties", "Properties")}</h4>

          <div class="prop-group">
            <label for="prop-status">{t("company.issues.status", "Status")}</label>
            <select
              id="prop-status"
              value={issue.status}
              onchange={(e) => patchIssue({ status: e.currentTarget.value as any })}
            >
              <option value="backlog">Backlog</option>
              <option value="todo">To Do</option>
              <option value="in_progress">In Progress</option>
              <option value="in_review">In Review</option>
              <option value="blocked">Blocked</option>
              <option value="done">Done</option>
              <option value="cancelled">Cancelled</option>
            </select>
          </div>

          <div class="prop-group">
            <label for="prop-priority">{t("company.issues.priority", "Priority")}</label>
            <select
              id="prop-priority"
              value={issue.priority ?? "medium"}
              onchange={(e) => patchIssue({ priority: e.currentTarget.value as any })}
            >
              <option value="urgent">Urgent</option>
              <option value="high">High</option>
              <option value="medium">Medium</option>
              <option value="low">Low</option>
            </select>
          </div>

          <div class="prop-group">
            <label for="prop-assignee">{t("company.issues.assignee", "Assignee")}</label>
            <select
              id="prop-assignee"
              value={issue.assignee_id ?? ""}
              onchange={(e) => patchIssue({ assignee_id: e.currentTarget.value || null })}
            >
              <option value="">{t("company.issues.unassigned", "Unassigned")}</option>
              {#each staffList as s}
                <option value={s.id}>{s.name} ({s.role})</option>
              {/each}
            </select>
          </div>

          <div class="prop-group">
            <label for="prop-project">{t("company.issues.project", "Project")}</label>
            <select
              id="prop-project"
              value={issue.project_id ?? ""}
              onchange={(e) => patchIssue({ project_id: e.currentTarget.value || null })}
            >
              <option value="">{t("company.issues.no_project", "None")}</option>
              {#each projects as p}
                <option value={p.id}>{p.name} ({p.key})</option>
              {/each}
            </select>
          </div>

          <div class="prop-group">
            <label for="prop-goal">{t("company.issues.goal", "Strategic Goal")}</label>
            <select
              id="prop-goal"
              value={issue.goal_id ?? ""}
              onchange={(e) => patchIssue({ goal_id: e.currentTarget.value || null })}
            >
              <option value="">{t("company.issues.no_goal", "None")}</option>
              {#each goals as g}
                <option value={g.id}>{g.title}</option>
              {/each}
            </select>
          </div>
        </div>

        <!-- Tags Panel -->
        <div class="panel-card">
          <div class="panel-head">
            <h4>{t("company.issues.tags", "Tags")}</h4>
            {#if !addingTag}
              <button class="link-btn" onclick={() => (addingTag = true)}>
                <Plus size={13} />
                <span>{t("common.add", "Add")}</span>
              </button>
            {/if}
          </div>

          <div class="tags-wrapper">
            {#each issue.tags ?? [] as tag}
              <span class="tag-pill">
                #{tag}
                <button class="tag-remove-btn" onclick={() => removeTag(tag)}>
                  <X size={11} />
                </button>
              </span>
            {/each}

            {#if addingTag}
              <div class="tag-input-row">
                <input
                  type="text"
                  placeholder="tag-name"
                  bind:value={newTagInput}
                  onkeydown={(e) => e.key === "Enter" && addTag()}
                />
                <button class="btn btn-primary btn-sm" onclick={addTag}>
                  <Check size={13} />
                </button>
                <button class="btn btn-secondary btn-sm" onclick={() => (addingTag = false)}>
                  <X size={13} />
                </button>
              </div>
            {/if}

            {#if (!issue.tags || issue.tags.length === 0) && !addingTag}
              <span class="text-muted text-sm">{t("company.issues.no_tags", "No tags assigned.")}</span>
            {/if}
          </div>
        </div>

        <!-- Meta Details Panel -->
        <div class="panel-card meta-card">
          <h4>{t("company.issues.details", "Details")}</h4>
          <div class="meta-row">
            <span>{t("company.issues.created", "Created:")}</span>
            <strong>{formatRelativeTime(issue.created_at, t)}</strong>
          </div>
          {#if issue.creator}
            <div class="meta-row">
              <span>{t("company.issues.creator", "Creator:")}</span>
              <strong>{issue.creator}</strong>
            </div>
          {/if}
        </div>

        <!-- Danger Zone -->
        <div class="panel-card danger-card">
          <h4>{t("company.common.danger_zone", "Danger Zone")}</h4>
          {#if confirmDelete}
            <div class="confirm-delete-box">
              <p>{t("company.issues.confirm_delete", "Permanently delete this issue?")}</p>
              <div class="confirm-actions">
                <button class="btn btn-secondary btn-sm" onclick={() => (confirmDelete = false)}>
                  {t("common.cancel", "Cancel")}
                </button>
                <button
                  class="btn btn-danger btn-sm"
                  disabled={deleting}
                  onclick={handleDeleteIssue}
                >
                  <Trash2 size={13} />
                  <span>{deleting ? t("common.deleting", "Deleting...") : t("common.confirm", "Delete")}</span>
                </button>
              </div>
            </div>
          {:else}
            <button class="btn btn-danger-outline btn-sm" onclick={() => (confirmDelete = true)}>
              <Trash2 size={13} />
              <span>{t("company.issues.delete", "Delete Issue")}</span>
            </button>
          {/if}
        </div>
      </aside>
    </div>
  {/if}
</div>

<style>
  .issue-detail-page {
    padding: 24px;
    max-width: 1400px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .nav-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border-color, #2d3340);
  }

  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted, #8b949e);
    text-decoration: none;
    font-size: 0.85rem;
    font-weight: 500;
    transition: color 0.15s ease;
  }

  .back-link:hover {
    color: var(--text-normal, #e6edf3);
  }

  .badge-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .id-pill {
    font-size: 0.82rem;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 6px;
    background: var(--bg-surface-elevated, #21262d);
    color: var(--primary, #3b82f6);
    border: 1px solid var(--border-color, #30363d);
  }

  .content-layout {
    display: grid;
    grid-template-columns: 1fr 340px;
    gap: 24px;
    align-items: start;
  }

  @media (max-width: 960px) {
    .content-layout {
      grid-template-columns: 1fr;
    }
  }

  .main-column {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .title-block {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .issue-title {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    line-height: 1.3;
    cursor: pointer;
  }

  .issue-title:hover {
    color: var(--primary, #3b82f6);
  }

  .edit-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .title-input {
    flex: 1;
    font-size: 1.25rem;
    font-weight: 700;
    padding: 6px 12px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
  }

  .transitions-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .trans-label {
    font-size: 0.8rem;
    color: var(--text-muted, #8b949e);
  }

  .trans-btns {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .status-trans-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
  }

  .dot-green { background: #10b981; }
  .dot-blue { background: #3b82f6; }
  .dot-orange { background: #f59e0b; }
  .dot-red { background: #ef4444; }
  .dot-gray { background: #8b949e; }
  .dot-faint { background: #484f58; }

  .capitalize {
    text-transform: capitalize;
  }

  .section-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .section-head h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .link-btn {
    background: transparent;
    border: none;
    color: var(--primary, #3b82f6);
    font-size: 0.82rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .link-btn:hover {
    text-decoration: underline;
  }

  .desc-content p {
    margin: 0;
    font-size: 0.9rem;
    line-height: 1.6;
    color: var(--text-normal, #e6edf3);
    white-space: pre-wrap;
  }

  .desc-textarea {
    width: 100%;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.88rem;
    font-family: inherit;
  }

  .edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }

  .empty-text {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
    font-style: italic;
    cursor: pointer;
  }

  /* Runs */
  .runs-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .run-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-radius: 6px;
    background: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    font-size: 0.82rem;
  }

  .run-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .run-id {
    font-weight: 600;
    color: var(--primary, #3b82f6);
  }

  .run-agent {
    color: var(--text-muted, #8b949e);
  }

  .run-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .run-cost {
    font-weight: 600;
  }

  .run-time {
    color: var(--text-muted, #8b949e);
  }

  /* Comments */
  .comments-thread {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .comment-item {
    display: flex;
    gap: 12px;
  }

  .comment-avatar {
    width: 32px;
    height: 32px;
    border-radius: 999px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    font-size: 0.75rem;
    font-weight: 700;
    flex-shrink: 0;
  }

  .comment-bubble {
    flex: 1;
    background: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 8px;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .comment-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.8rem;
  }

  .comment-time {
    color: var(--text-muted, #8b949e);
    font-size: 0.75rem;
  }

  .comment-body {
    margin: 0;
    font-size: 0.85rem;
    line-height: 1.45;
  }

  .comment-input-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
  }

  .comment-input-form textarea {
    width: 100%;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
    font-family: inherit;
  }

  .comment-footer {
    display: flex;
    justify-content: flex-end;
  }

  /* Sidebar */
  .sidebar-column {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .panel-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .panel-card h4 {
    margin: 0;
    font-size: 0.88rem;
    font-weight: 600;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .prop-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .prop-group label {
    font-size: 0.78rem;
    color: var(--text-muted, #8b949e);
  }

  .prop-group select {
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.82rem;
  }

  .tags-wrapper {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tag-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    color: var(--text-muted, #8b949e);
    border: 1px solid var(--border-color, #30363d);
  }

  .tag-remove-btn {
    background: transparent;
    border: none;
    color: inherit;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .tag-input-row {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
  }

  .tag-input-row input {
    flex: 1;
    padding: 4px 8px;
    border-radius: 4px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.8rem;
  }

  .meta-card {
    gap: 8px;
  }

  .meta-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.8rem;
  }

  .meta-row span {
    color: var(--text-muted, #8b949e);
  }

  .danger-card {
    border-color: rgba(239, 68, 68, 0.3);
  }

  .danger-card h4 {
    color: #ef4444;
  }

  .confirm-delete-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 0.82rem;
  }

  .confirm-delete-box p {
    margin: 0;
    color: #ef4444;
  }

  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
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

  .btn-secondary {
    background: var(--bg-surface-elevated, #21262d);
    border-color: var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
  }

  .btn-danger {
    background: #ef4444;
    color: #fff;
  }

  .btn-danger-outline {
    background: transparent;
    border-color: rgba(239, 68, 68, 0.4);
    color: #ef4444;
  }

  .btn-danger-outline:hover {
    background: rgba(239, 68, 68, 0.1);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .status-pill {
    font-size: 0.72rem;
    padding: 2px 8px;
    border-radius: 999px;
    text-transform: capitalize;
    font-weight: 600;
  }

  .status-green { background: rgba(16, 185, 129, 0.2); color: #10b981; }
  .status-blue { background: rgba(59, 130, 246, 0.2); color: #3b82f6; }
  .status-orange { background: rgba(245, 158, 11, 0.2); color: #f59e0b; }
  .status-red { background: rgba(239, 68, 68, 0.2); color: #ef4444; }
  .status-gray { background: rgba(139, 148, 158, 0.2); color: #8b949e; }
  .status-faint { background: rgba(72, 79, 88, 0.2); color: #484f58; }

  .badge {
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 0.72rem;
  }

  .badge-secondary {
    background: var(--bg-surface, #21262d);
    color: var(--text-muted, #8b949e);
  }

  .text-primary { color: var(--primary, #3b82f6); }
  .text-danger { color: #ef4444; }
  .text-muted { color: var(--text-muted, #8b949e); }
  .text-sm { font-size: 0.75rem; }

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
</style>
