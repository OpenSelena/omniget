<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import {
    companyRpc,
    t,
    downloadFile,
    type Company,
  } from "$lib/v100/company-api";
  import {
    Settings,
    Save,
    Download,
    FileSpreadsheet,
    AlertTriangle,
    Pause,
    Play,
    Trash2,
    Archive,
    Check,
    RefreshCw,
    Folder,
  } from "lucide-svelte";

  const companyId = $derived($page.params.id || "omniget-core");

  let loading = $state(true);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<{ type: "success" | "error"; text: string } | null>(null);

  let company = $state<Company | null>(null);

  // Form fields
  let name = $state("");
  let mission = $state("");
  let issuePrefix = $state("");
  let workspace = $state("");
  let status = $state<"active" | "paused" | "archived">("active");
  let pauseReason = $state("");

  // Delete confirmation
  let showDeleteModal = $state(false);
  let deleteConfirmInput = $state("");
  let deleting = $state(false);

  async function loadCompany() {
    loading = true;
    error = null;
    try {
      const res = await companyRpc<Company>("company.get", { company: companyId });
      company = res;
      if (company) {
        name = company.name;
        mission = company.mission ?? "";
        issuePrefix = company.issue_prefix;
        workspace = company.workspace ?? "";
        status = company.status;
        pauseReason = company.pause_reason ?? "";
      }
    } catch (err: any) {
      error = err?.message || "Failed to load company settings";
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadCompany();
  });

  function showNotice(text: string, type: "success" | "error" = "success") {
    notice = { type, text };
    setTimeout(() => {
      if (notice?.text === text) notice = null;
    }, 4000);
  }

  async function handleSaveProfile(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) {
      showNotice("Company name is required", "error");
      return;
    }
    saving = true;
    try {
      const updated = await companyRpc<Company>("company.update", {
        company: companyId,
        patch: {
          name: name.trim(),
          mission: mission.trim(),
          issue_prefix: issuePrefix.trim().toUpperCase() || "OMNI",
          workspace: workspace.trim() || null,
        },
      });
      if (updated) company = updated;
      showNotice(t("company.settings.saved_profile", "Company settings saved successfully."));
    } catch (err: any) {
      showNotice(err?.message || "Failed to save settings", "error");
    } finally {
      saving = false;
    }
  }

  async function toggleStatus() {
    const nextStatus = status === "active" ? "paused" : "active";
    try {
      await companyRpc("company.status", {
        company: companyId,
        status: nextStatus,
        reason: nextStatus === "paused" ? pauseReason || "Manual pause by user" : null,
      });
      status = nextStatus;
      if (company) company.status = nextStatus;
      showNotice(`Organization status changed to ${nextStatus}.`);
    } catch (err: any) {
      showNotice(err?.message || "Failed to update status", "error");
    }
  }

  async function exportBlueprint() {
    try {
      const dashboard = await companyRpc("company.dashboard", { company: companyId });
      const payload = {
        blueprint_version: "1.0.0",
        exported_at: new Date().toISOString(),
        company: {
          name,
          mission,
          issue_prefix: issuePrefix,
          workspace,
        },
        staff: dashboard?.staff ?? [],
        goals: dashboard?.goals ?? [],
        projects: dashboard?.projects ?? [],
        budgets: dashboard?.budgets ?? [],
      };
      downloadFile(
        `${(issuePrefix || "omni").toLowerCase()}-blueprint.json`,
        JSON.stringify(payload, null, 2),
        "application/json"
      );
      showNotice(t("company.settings.blueprint_downloaded", "Blueprint JSON exported successfully."));
    } catch (err: any) {
      showNotice(err?.message || "Failed to export blueprint", "error");
    }
  }

  async function exportAuditTrail() {
    try {
      const res = await companyRpc<{ csv: string }>("audit.csv", { company: companyId });
      downloadFile(
        `${(issuePrefix || "omni").toLowerCase()}-audit-log.csv`,
        res?.csv || "seq,ts,action,entity,actor\n",
        "text/csv"
      );
      showNotice(t("company.settings.audit_downloaded", "Audit trail CSV exported."));
    } catch (err: any) {
      showNotice(err?.message || "Failed to export audit trail", "error");
    }
  }

  async function handleDeleteCompany() {
    if (deleteConfirmInput.trim() !== name.trim()) {
      alert("Confirmation name does not match.");
      return;
    }
    deleting = true;
    try {
      await companyRpc("company.delete", { company: companyId });
      goto("/llm/company");
    } catch (err: any) {
      alert("Failed to delete company: " + (err?.message || err));
      deleting = false;
    }
  }
</script>

<svelte:head>
  <title>{t("company.settings.title", "Company Settings")} - OmniGet</title>
</svelte:head>

<div class="settings-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Settings size={22} class="text-primary" />
      </div>
      <div>
        <h1>{t("company.settings.header", "Organization Settings")}</h1>
        <p class="subtitle">
          {t("company.settings.sub", "Manage workspace identity, operational limits, export blue-prints, and system permissions.")}
        </p>
      </div>
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

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading settings...")}</p>
    </div>
  {:else if error || !company}
    <div class="error-card">
      <AlertTriangle size={20} class="text-danger" />
      <div>
        <h4>{t("company.settings.failed", "Failed to Load Settings")}</h4>
        <p>{error || "Organization not found"}</p>
      </div>
      <button class="btn btn-secondary btn-sm" onclick={loadCompany}>
        {t("common.retry", "Retry")}
      </button>
    </div>
  {:else}
    <div class="settings-content">
      <!-- Section 1: General Profile -->
      <section class="settings-card">
        <div class="card-head">
          <h3>{t("company.settings.profile_heading", "Organization Identity")}</h3>
          <p>{t("company.settings.profile_desc", "Configure the public name, mission charter, and issue identifiers.")}</p>
        </div>

        <form onsubmit={handleSaveProfile} class="settings-form">
          <div class="form-group">
            <label for="comp-name">{t("company.settings.name_label", "Company Name *")}</label>
            <input
              id="comp-name"
              type="text"
              bind:value={name}
              placeholder="e.g. OmniGet Operations"
              required
            />
          </div>

          <div class="form-group">
            <label for="comp-mission">{t("company.settings.mission_label", "Mission / Mandate")}</label>
            <textarea
              id="comp-mission"
              rows="3"
              bind:value={mission}
              placeholder="The primary purpose and autonomous directive of this organization..."
            ></textarea>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="comp-prefix">{t("company.settings.prefix_label", "Issue Key Prefix")}</label>
              <input
                id="comp-prefix"
                type="text"
                maxlength="6"
                bind:value={issuePrefix}
                placeholder="OMNI"
                style="text-transform: uppercase;"
              />
              <span class="hint">{t("company.settings.prefix_hint", "Used for issue keys, e.g. OMNI-1, OMNI-2.")}</span>
            </div>

            <div class="form-group">
              <label for="comp-workspace">{t("company.settings.workspace_label", "Workspace Directory")}</label>
              <div class="input-with-icon">
                <Folder size={14} class="field-icon" />
                <input
                  id="comp-workspace"
                  type="text"
                  bind:value={workspace}
                  placeholder="x:\omniget"
                />
              </div>
              <span class="hint">{t("company.settings.workspace_hint", "Local folder root for agent filesystem tools.")}</span>
            </div>
          </div>

          <div class="form-actions">
            <button type="submit" class="btn btn-primary" disabled={saving}>
              <Save size={15} />
              <span>{saving ? t("common.saving", "Saving...") : t("common.save_changes", "Save Changes")}</span>
            </button>
          </div>
        </form>
      </section>

      <!-- Section 2: Operations State -->
      <section class="settings-card">
        <div class="card-head">
          <h3>{t("company.settings.status_heading", "Operational Status")}</h3>
          <p>{t("company.settings.status_desc", "Pause all automated agent execution loops or resume standard operations.")}</p>
        </div>

        <div class="status-box">
          <div class="status-left">
            <span class="status-badge status-{status}">{status}</span>
            <div class="status-info">
              <h4>{status === "active" ? t("company.settings.status_active", "Operations Active") : t("company.settings.status_paused", "Operations Paused")}</h4>
              <p>
                {status === "active"
                  ? t("company.settings.active_note", "All agents and routines execute normally when scheduled or assigned.")
                  : t("company.settings.paused_note", "All runs, loops, and routine dispatches are currently halted.")}
              </p>
            </div>
          </div>

          <button
            class="btn {status === 'active' ? 'btn-secondary' : 'btn-primary'}"
            onclick={toggleStatus}
          >
            {#if status === "active"}
              <Pause size={15} />
              <span>{t("company.settings.pause_ops", "Pause All Operations")}</span>
            {:else}
              <Play size={15} />
              <span>{t("company.settings.resume_ops", "Resume Operations")}</span>
            {/if}
          </button>
        </div>
      </section>

      <!-- Section 3: Blueprint & Data Portability -->
      <section class="settings-card">
        <div class="card-head">
          <h3>{t("company.settings.portability_heading", "Data Portability & Blueprints")}</h3>
          <p>{t("company.settings.portability_desc", "Export company structure for backup or to share as a reusable template.")}</p>
        </div>

        <div class="export-grid">
          <div class="export-item">
            <div class="export-text">
              <h4>{t("company.settings.export_blueprint_title", "Export Blueprint (.json)")}</h4>
              <p>{t("company.settings.export_blueprint_sub", "Download complete architecture: staff definitions, routines, goals, and budget rules.")}</p>
            </div>
            <button class="btn btn-secondary btn-sm" onclick={exportBlueprint}>
              <Download size={14} />
              <span>{t("common.export_json", "Export JSON")}</span>
            </button>
          </div>

          <div class="export-item">
            <div class="export-text">
              <h4>{t("company.settings.export_audit_title", "Cryptographic Audit Log (.csv)")}</h4>
              <p>{t("company.settings.export_audit_sub", "Export verifiable immutable event ledger with cryptographic SHA-256 signatures.")}</p>
            </div>
            <button class="btn btn-secondary btn-sm" onclick={exportAuditTrail}>
              <FileSpreadsheet size={14} />
              <span>{t("common.export_csv", "Export CSV")}</span>
            </button>
          </div>
        </div>
      </section>

      <!-- Section 4: Danger Zone -->
      <section class="settings-card danger-section">
        <div class="card-head">
          <h3 class="text-danger">{t("company.common.danger_zone", "Danger Zone")}</h3>
          <p>{t("company.settings.danger_desc", "Destructive operations that cannot be undone. Exercise extreme caution.")}</p>
        </div>

        <div class="danger-row">
          <div>
            <h4>{t("company.settings.delete_title", "Delete Organization")}</h4>
            <p>{t("company.settings.delete_sub", "Permanently remove this company, all associated staff, issue records, and routines.")}</p>
          </div>
          <button class="btn btn-danger btn-sm" onclick={() => (showDeleteModal = true)}>
            <Trash2 size={14} />
            <span>{t("company.settings.delete_btn", "Delete Company")}</span>
          </button>
        </div>
      </section>
    </div>
  {/if}

  <!-- Delete Modal Confirmation -->
  {#if showDeleteModal}
    <div class="modal-backdrop" onclick={() => (showDeleteModal = false)}>
      <div class="modal-box" onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3 class="text-danger">{t("company.settings.confirm_delete_title", "Confirm Deletion")}</h3>
        </div>

        <div class="modal-body">
          <p>{t("company.settings.delete_warning", "This action is permanent and cannot be undone. All staff records, scheduled routines, and issues will be purged.")}</p>
          <p>
            {t("company.settings.type_name_to_confirm", "Please type")}
            <strong class="highlight-text">{name}</strong>
            {t("company.settings.to_confirm", "to confirm:")}
          </p>
          <input
            type="text"
            placeholder={name}
            bind:value={deleteConfirmInput}
            class="confirm-input"
          />
        </div>

        <div class="modal-footer">
          <button
            type="button"
            class="btn btn-secondary"
            disabled={deleting}
            onclick={() => (showDeleteModal = false)}
          >
            {t("common.cancel", "Cancel")}
          </button>
          <button
            type="button"
            class="btn btn-danger"
            disabled={deleting || deleteConfirmInput.trim() !== name.trim()}
            onclick={handleDeleteCompany}
          >
            <Trash2 size={14} />
            <span>{deleting ? t("common.deleting", "Deleting...") : t("common.delete_permanently", "Delete Permanently")}</span>
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .settings-page {
    padding: 24px;
    max-width: 960px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
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

  h1 {
    font-size: 1.35rem;
    font-weight: 700;
    margin: 0;
  }

  .subtitle {
    margin: 4px 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
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

  .settings-content {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .settings-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .card-head h3 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .card-head p {
    margin: 4px 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
  }

  .settings-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-muted, #8b949e);
  }

  .form-group input,
  .form-group textarea {
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
    font-family: inherit;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 14px;
  }

  @media (max-width: 600px) {
    .form-row {
      grid-template-columns: 1fr;
    }
  }

  .hint {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
  }

  .input-with-icon {
    position: relative;
    display: flex;
    align-items: center;
  }

  :global(.field-icon) {
    position: absolute;
    left: 10px;
    color: var(--text-muted, #8b949e);
  }

  .input-with-icon input {
    width: 100%;
    padding-left: 32px;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 6px;
  }

  /* Operations Status */
  .status-box {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px;
    background: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 6px;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .status-badge {
    padding: 4px 10px;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .status-badge.status-active {
    background: rgba(16, 185, 129, 0.2);
    color: #10b981;
  }

  .status-badge.status-paused {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .status-info h4 {
    margin: 0;
    font-size: 0.95rem;
  }

  .status-info p {
    margin: 2px 0 0 0;
    font-size: 0.82rem;
    color: var(--text-muted, #8b949e);
  }

  /* Export */
  .export-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .export-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    background: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 6px;
  }

  .export-text h4 {
    margin: 0;
    font-size: 0.92rem;
    font-weight: 600;
  }

  .export-text p {
    margin: 2px 0 0 0;
    font-size: 0.8rem;
    color: var(--text-muted, #8b949e);
  }

  /* Danger */
  .danger-section {
    border-color: rgba(239, 68, 68, 0.3);
  }

  .danger-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .danger-row h4 {
    margin: 0;
    font-size: 0.95rem;
  }

  .danger-row p {
    margin: 2px 0 0 0;
    font-size: 0.82rem;
    color: var(--text-muted, #8b949e);
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-box {
    background: var(--bg-surface-elevated, #1c2128);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 10px;
    width: 100%;
    max-width: 480px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.15rem;
  }

  .modal-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-size: 0.85rem;
    color: var(--text-normal, #e6edf3);
  }

  .highlight-text {
    color: #ef4444;
  }

  .confirm-input {
    width: 100%;
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--bg-surface, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  /* Buttons & Utilities */
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

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .text-danger { color: #ef4444; }

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
