<script lang="ts">
  import { onMount } from "svelte";
  import {
    llmRpc,
    t,
    type SkillItem,
  } from "$lib/v100/llm-engine-api";
  import {
    Sparkles,
    Search,
    Play,
    RefreshCw,
    Wrench,
    CheckCircle2,
    BookOpen,
    Code,
  } from "lucide-svelte";

  let loading = $state(true);
  let refreshing = $state(false);
  let skills = $state<SkillItem[]>([]);
  let searchQuery = $state("");

  async function loadSkills(quiet = false) {
    if (!quiet) loading = true;
    else refreshing = true;
    try {
      const res = await llmRpc<SkillItem[]>("skills.list");
      skills = Array.isArray(res) ? res : [];
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  onMount(() => {
    loadSkills();
  });

  const filteredSkills = $derived(
    skills.filter((s) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        (s.trigger_phrase ?? "").toLowerCase().includes(q)
      );
    })
  );

  async function toggleSkill(skill: SkillItem) {
    skill.enabled = !skill.enabled;
    try {
      await llmRpc("skills.toggle", { id: skill.id });
    } catch {
      skill.enabled = !skill.enabled;
    }
  }
</script>

<svelte:head>
  <title>{t("llm.skills.title", "Agent Skills")} - OmniGet</title>
</svelte:head>

<div class="skills-page">
  <!-- Top Bar -->
  <header class="top-bar">
    <div class="header-left">
      <div class="icon-bubble">
        <Sparkles size={22} class="text-primary" />
      </div>
      <div>
        <div class="title-row">
          <h1>{t("llm.skills.header", "Autonomous Agent Skills")}</h1>
          <span class="count-pill">{skills.length} Installed</span>
        </div>
        <p class="subtitle">
          {t("llm.skills.sub", "Composable skills, domain capabilities, and specialized procedural workflows.")}
        </p>
      </div>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary btn-sm" onclick={() => loadSkills(true)} disabled={refreshing}>
        <RefreshCw size={14} class={refreshing ? "spin" : ""} />
        <span>{refreshing ? t("common.refreshing", "Refreshing...") : t("common.refresh", "Refresh")}</span>
      </button>
    </div>
  </header>

  <!-- Search Bar -->
  <div class="controls-bar">
    <div class="search-wrap">
      <Search size={14} class="search-icon" />
      <input
        type="search"
        placeholder={t("llm.skills.search_placeholder", "Filter skills by name, trigger phrase...")}
        bind:value={searchQuery}
      />
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <RefreshCw size={24} class="spin text-primary" />
      <p>{t("common.loading", "Loading agent skills...")}</p>
    </div>
  {:else}
    <div class="skills-grid">
      {#each filteredSkills as s (s.id)}
        <div class="skill-card" class:disabled={!s.enabled}>
          <div class="card-head">
            <div>
              <h3 class="skill-name">{s.name}</h3>
              <span class="author-tag">v{s.version} by {s.author}</span>
            </div>

            <label class="switch-wrap">
              <input
                type="checkbox"
                checked={s.enabled}
                onchange={() => toggleSkill(s)}
              />
              <span class="slider"></span>
            </label>
          </div>

          <p class="skill-desc">{s.description}</p>

          {#if s.tools_required && s.tools_required.length > 0}
            <div class="tools-req">
              <span class="req-label">Required Tools:</span>
              <div class="req-pills">
                {#each s.tools_required as tool}
                  <span class="tool-req-pill">
                    <Wrench size={11} />
                    <span>{tool}</span>
                  </span>
                {/each}
              </div>
            </div>
          {/if}

          <div class="card-footer">
            <button
              class="btn btn-secondary btn-sm run-btn"
              disabled={!s.enabled}
              onclick={() => alert(`Activated skill: ${s.name}`)}
            >
              <Play size={13} />
              <span>Run Skill</span>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .skills-page {
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

  .controls-bar {
    border-bottom: 1px solid var(--border-color, #2d3340);
    padding-bottom: 12px;
  }

  .search-wrap {
    position: relative;
    max-width: 360px;
  }

  :global(.search-icon) {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted, #8b949e);
  }

  .search-wrap input {
    width: 100%;
    padding: 6px 10px 6px 30px;
    border-radius: 6px;
    background: var(--bg-surface, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    color: var(--text-normal, #e6edf3);
    font-size: 0.85rem;
  }

  .skills-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
  }

  .skill-card {
    background: var(--bg-surface-elevated, #161b22);
    border: 1px solid var(--border-color, #2d3340);
    border-radius: 8px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: opacity 0.2s ease;
  }

  .skill-card.disabled {
    opacity: 0.6;
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }

  .skill-name {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
  }

  .author-tag {
    font-size: 0.72rem;
    color: var(--text-muted, #8b949e);
  }

  .skill-desc {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-muted, #8b949e);
    line-height: 1.45;
    flex: 1;
  }

  .tools-req {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.75rem;
  }

  .req-label {
    color: var(--text-muted, #8b949e);
  }

  .req-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .tool-req-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-surface, #21262d);
    color: var(--text-normal, #e6edf3);
    font-family: monospace;
    font-size: 0.7rem;
  }

  .card-footer {
    display: flex;
    justify-content: flex-end;
    padding-top: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  /* Switch */
  .switch-wrap {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
    cursor: pointer;
  }

  .switch-wrap input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    inset: 0;
    background-color: var(--bg-surface, #21262d);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 20px;
    transition: 0.2s;
  }

  .slider:before {
    position: absolute;
    content: "";
    height: 12px;
    width: 12px;
    left: 3px;
    bottom: 3px;
    background-color: var(--text-muted, #8b949e);
    border-radius: 50%;
    transition: 0.2s;
  }

  input:checked + .slider {
    background-color: var(--primary, #3b82f6);
    border-color: var(--primary, #3b82f6);
  }

  input:checked + .slider:before {
    transform: translateX(16px);
    background-color: #fff;
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
</style>
