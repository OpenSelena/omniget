<script lang="ts">
  import { t, workspaceRpc } from "$lib/v100/workspace-api";
  import {
    ChevronLeft,
    Wand2,
    CheckCircle2,
    Shield,
    FileCode,
    Package,
    ArrowRight,
    Terminal,
  } from "lucide-svelte";

  let currentStep = $state(1);
  let publishing = $state(false);
  let published = $state(false);

  // Form State
  let category = $state<"tools" | "skills" | "agents" | "mcp">("tools");
  let pkgSlug = $state("my-custom-tool");
  let pkgName = $state("My Custom Tool");
  let author = $state("Local Developer");
  let version = $state("0.1.0");
  let description = $state("A high-performance autonomous agent tool.");
  let permissions = $state<string[]>(["fs:read"]);

  const availablePermissions = [
    { id: "fs:read", label: "File System: Read Only", risk: "low" },
    { id: "fs:write", label: "File System: Write & Modify", risk: "medium" },
    { id: "network:http", label: "Network: HTTP/HTTPS Client", risk: "low" },
    { id: "process:spawn", label: "System: Spawn Process / Shell", risk: "high" },
    { id: "browser:automate", label: "Browser: Headless Automation", risk: "medium" },
  ];

  function togglePermission(permId: string) {
    if (permissions.includes(permId)) {
      permissions = permissions.filter((p) => p !== permId);
    } else {
      permissions = [...permissions, permId];
    }
  }

  const generatedManifest = $derived(
    JSON.stringify(
      {
        id: `${category}/${pkgSlug}`,
        name: pkgName,
        version,
        description,
        category,
        author,
        permissions,
        main: "index.ts",
        created_at: new Date().toISOString(),
      },
      null,
      2
    )
  );

  async function handlePublish() {
    publishing = true;
    try {
      // Simulate package compilation & local registry insertion
      await new Promise((r) => setTimeout(r, 600));
      published = true;
    } finally {
      publishing = false;
    }
  }
</script>

<div class="wizard-page">
  <div class="nav-back">
    <a href="/llm/catalog" class="back-link">
      <ChevronLeft size={16} />
      {t("catalog.back", "Back to Catalog")}
    </a>
  </div>

  <header class="wizard-head">
    <div class="head-icon">
      <Wand2 size={24} class="text-blue-400" />
    </div>
    <div>
      <h1 class="page-title">{t("catalog.wizard.title", "Package Creation Wizard")}</h1>
      <p class="page-subtitle">
        {t("catalog.wizard.subtitle", "Scaffold and register custom tools, skills, and MCP extensions into OmniGet.")}
      </p>
    </div>
  </header>

  <!-- Stepper -->
  <div class="stepper-bar">
    <button
      type="button"
      class="step-item"
      class:active={currentStep === 1}
      class:done={currentStep > 1}
      onclick={() => (currentStep = 1)}
    >
      <span class="step-num">1</span>
      <span class="step-label">Identity</span>
    </button>
    <div class="step-line"></div>
    <button
      type="button"
      class="step-item"
      class:active={currentStep === 2}
      class:done={currentStep > 2}
      onclick={() => (currentStep = 2)}
    >
      <span class="step-num">2</span>
      <span class="step-label">Permissions</span>
    </button>
    <div class="step-line"></div>
    <button
      type="button"
      class="step-item"
      class:active={currentStep === 3}
      class:done={currentStep > 3}
      onclick={() => (currentStep = 3)}
    >
      <span class="step-num">3</span>
      <span class="step-label">Manifest & Build</span>
    </button>
  </div>

  <!-- Wizard Steps Body -->
  <main class="wizard-card">
    {#if published}
      <div class="success-view">
        <CheckCircle2 size={48} class="text-green-400 mb-2" />
        <h2>{t("catalog.wizard.success_title", "Package Registered Successfully!")}</h2>
        <p class="text-muted">
          Your package <code>{category}/{pkgSlug}</code> is now compiled and available in your local catalog.
        </p>
        <div class="success-actions">
          <a href="/llm/catalog" class="btn btn-primary">Browse Catalog</a>
          <a href="/llm/installed" class="btn btn-secondary">Manage Installed</a>
        </div>
      </div>
    {:else if currentStep === 1}
      <section class="step-section">
        <h2>Step 1: Package Identity & Type</h2>
        <p class="section-hint">Define how your package is cataloged and identified.</p>

        <div class="form-grid">
          <div class="form-group">
            <label for="pkg-category">Package Category</label>
            <select id="pkg-category" bind:value={category} class="form-input">
              <option value="tools">Tool (Function Calling)</option>
              <option value="skills">Skill (Multi-Step Logic)</option>
              <option value="agents">Autonomous Agent</option>
              <option value="mcp">MCP Server Wrapper</option>
            </select>
          </div>

          <div class="form-group">
            <label for="pkg-slug">Package Slug</label>
            <input
              id="pkg-slug"
              type="text"
              bind:value={pkgSlug}
              placeholder="e.g. data-miner"
              class="form-input"
            />
          </div>

          <div class="form-group">
            <label for="pkg-name">Display Name</label>
            <input
              id="pkg-name"
              type="text"
              bind:value={pkgName}
              placeholder="e.g. Advanced Data Miner"
              class="form-input"
            />
          </div>

          <div class="form-group">
            <label for="pkg-version">Initial Version</label>
            <input
              id="pkg-version"
              type="text"
              bind:value={version}
              placeholder="0.1.0"
              class="form-input"
            />
          </div>

          <div class="form-group full-width">
            <label for="pkg-desc">Description</label>
            <textarea
              id="pkg-desc"
              rows="3"
              bind:value={description}
              placeholder="Summary of capabilities..."
              class="form-input"
            ></textarea>
          </div>
        </div>

        <div class="wizard-actions">
          <div></div>
          <button type="button" class="btn btn-primary" onclick={() => (currentStep = 2)}>
            Next: Permissions <ArrowRight size={14} class="ml-1 inline" />
          </button>
        </div>
      </section>
    {:else if currentStep === 2}
      <section class="step-section">
        <h2>Step 2: Sandbox Permissions</h2>
        <p class="section-hint">Specify minimum necessary system capabilities to follow the principle of least privilege.</p>

        <div class="perm-options">
          {#each availablePermissions as perm}
            {@const isChecked = permissions.includes(perm.id)}
            <button
              type="button"
              class="perm-card"
              class:checked={isChecked}
              onclick={() => togglePermission(perm.id)}
            >
              <div class="perm-check">
                {#if isChecked}
                  <CheckCircle2 size={18} class="text-blue-400" />
                {:else}
                  <div class="empty-circle"></div>
                {/if}
              </div>
              <div class="perm-info">
                <span class="perm-name">{perm.label}</span>
                <span class="perm-slug"><code>{perm.id}</code></span>
              </div>
              <span class="risk-badge {perm.risk}">{perm.risk} risk</span>
            </button>
          {/each}
        </div>

        <div class="wizard-actions">
          <button type="button" class="btn btn-secondary" onclick={() => (currentStep = 1)}>
            Back
          </button>
          <button type="button" class="btn btn-primary" onclick={() => (currentStep = 3)}>
            Next: Manifest & Build <ArrowRight size={14} class="ml-1 inline" />
          </button>
        </div>
      </section>
    {:else}
      <section class="step-section">
        <h2>Step 3: Manifest Preview & Registration</h2>
        <p class="section-hint">Review generated package declaration before registering into local store.</p>

        <div class="manifest-box">
          <pre class="manifest-json"><code>{generatedManifest}</code></pre>
        </div>

        <div class="wizard-actions">
          <button type="button" class="btn btn-secondary" onclick={() => (currentStep = 2)}>
            Back
          </button>
          <button
            type="button"
            class="btn btn-primary"
            disabled={publishing}
            onclick={handlePublish}
          >
            {#if publishing}
              Compiling & Registering...
            {:else}
              <Package size={15} class="mr-1 inline" />
              Publish Package to Catalog
            {/if}
          </button>
        </div>
      </section>
    {/if}
  </main>
</div>

<style>
  .wizard-page {
    padding: 24px 32px;
    max-width: 900px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .nav-back {
    display: flex;
    align-items: center;
  }

  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-muted, #8b949e);
    text-decoration: none;
    font-size: 13px;
  }

  .back-link:hover {
    color: var(--accent-hi, #58a6ff);
  }

  .wizard-head {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .head-icon {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    background: var(--fill-1, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .page-title {
    font-size: 20px;
    font-weight: 700;
    margin: 0 0 4px;
  }

  .page-subtitle {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 0;
  }

  .stepper-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
  }

  .step-item {
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: none;
    color: var(--text-muted, #8b949e);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }

  .step-item.active {
    color: var(--accent-hi, #58a6ff);
    font-weight: 600;
  }

  .step-num {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--fill-1, rgba(255, 255, 255, 0.1));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
  }

  .step-item.active .step-num {
    background: var(--accent, #1f6feb);
    color: #fff;
  }

  .step-line {
    flex: 1;
    height: 1px;
    background: var(--separator, rgba(255, 255, 255, 0.1));
    margin: 0 16px;
  }

  .wizard-card {
    background: var(--surface, #161b22);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    padding: 24px;
  }

  .step-section h2 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 6px;
  }

  .section-hint {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 0 0 20px;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group.full-width {
    grid-column: 1 / -1;
  }

  .form-group label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted, #8b949e);
  }

  .form-input {
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--input-bg, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.12));
    color: var(--text, #fff);
    font-size: 13px;
    outline: none;
  }

  .form-input:focus {
    border-color: var(--accent, #58a6ff);
  }

  .perm-options {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 24px;
  }

  .perm-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-radius: 8px;
    background: var(--fill-1, rgba(255, 255, 255, 0.03));
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }

  .perm-card:hover {
    background: var(--fill-2, rgba(255, 255, 255, 0.06));
  }

  .perm-card.checked {
    border-color: rgba(56, 139, 253, 0.4);
    background: rgba(56, 139, 253, 0.06);
  }

  .empty-circle {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--text-muted, #8b949e);
  }

  .perm-info {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .perm-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text, #fff);
  }

  .perm-slug code {
    font-size: 11px;
    color: var(--text-muted, #8b949e);
  }

  .risk-badge {
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 4px;
    text-transform: capitalize;
  }

  .risk-badge.low { background: rgba(63, 185, 80, 0.15); color: #3fb950; }
  .risk-badge.medium { background: rgba(240, 136, 62, 0.15); color: #f0883e; }
  .risk-badge.high { background: rgba(248, 81, 73, 0.15); color: #f85149; }

  .manifest-box {
    margin-bottom: 24px;
    border-radius: 8px;
    background: var(--surface-mut, #0d1117);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    overflow: hidden;
  }

  .manifest-json {
    padding: 16px;
    margin: 0;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: #3fb950;
    overflow-x: auto;
  }

  .wizard-actions {
    display: flex;
    justify-content: space-between;
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border: none;
    text-decoration: none;
  }

  .btn-primary {
    background: var(--cta, #1f6feb);
    color: #fff;
  }

  .btn-primary:hover:not(:disabled) {
    background: #388bfd;
  }

  .btn-secondary {
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    color: var(--text, #fff);
  }

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .success-view {
    text-align: center;
    padding: 40px 20px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .success-actions {
    display: flex;
    gap: 12px;
    margin-top: 16px;
  }
</style>
