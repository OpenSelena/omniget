<script lang="ts">
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { COURSE_PLATFORMS, type CoursePlatform } from "$lib/course-platforms";
  import { t } from "$lib/i18n";

  let authStatus: Record<string, { checked: boolean; email: string | null }> = $state({});

  onMount(() => {
    for (const platform of COURSE_PLATFORMS) {
      if (platform.enabled && platform.authCheckCommand) {
        authStatus[platform.id] = { checked: false, email: null };
        invoke<string>(platform.authCheckCommand)
          .then((email) => {
            authStatus[platform.id] = { checked: true, email };
          })
          .catch(() => {
            authStatus[platform.id] = { checked: true, email: null };
          });
      }
    }
  });

  function handleCardClick(platform: CoursePlatform) {
    if (!platform.enabled) return;
    goto(platform.route);
  }

  function handleKeyDown(e: KeyboardEvent, platform: CoursePlatform) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleCardClick(platform);
    }
  }
</script>

<div class="courses-page">
  <h1>{$t("courses.title")}</h1>

  <div class="platform-grid">
    {#each COURSE_PLATFORMS as platform (platform.id)}
      <div
        class="platform-card"
        class:disabled={!platform.enabled}
        role="button"
        tabindex={platform.enabled ? 0 : -1}
        onclick={() => handleCardClick(platform)}
        onkeydown={(e) => handleKeyDown(e, platform)}
      >
        <div class="card-icon" style="--platform-color: {platform.color}">
          <svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            {#if platform.icon === "hotmart"}
              <path d="M6 4v16" />
              <path d="M18 4v16" />
              <path d="M6 12h12" />
            {:else if platform.icon === "udemy"}
              <path d="M22 9l-10 -4l-10 4l10 4l10 -4v6" />
              <path d="M6 10.6v5.4a6 3 0 0 0 12 0v-5.4" />
            {:else}
              <path d="M4 19.5A2.5 2.5 0 016.5 17H20" />
              <path d="M6.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15A2.5 2.5 0 016.5 2z" />
              <path d="M8 7h8" />
              <path d="M8 11h6" />
            {/if}
          </svg>
        </div>
        <span class="card-name">{platform.name}</span>
        <span class="card-status">
          {#if !platform.enabled}
            {$t("courses.coming_soon")}
          {:else if !authStatus[platform.id]?.checked}
            <span class="status-dot checking"></span>
          {:else if authStatus[platform.id]?.email}
            <span class="status-dot connected"></span>
            <span class="status-email">{authStatus[platform.id].email}</span>
          {:else}
            <span class="status-dot disconnected"></span>
            {$t("courses.not_connected")}
          {/if}
        </span>
      </div>
    {/each}
  </div>
</div>

<style>
  .courses-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    max-width: 800px;
  }

  h1 {
    font-size: 20px;
    font-weight: 500;
    margin-block: 0;
  }

  .platform-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--padding);
  }

  .platform-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--padding) * 0.75);
    padding: calc(var(--padding) * 2) var(--padding);
    background: var(--button-elevated);
    border-radius: var(--border-radius);
    cursor: pointer;
    transition: transform 0.15s, background 0.15s;
  }

  @media (hover: hover) {
    .platform-card:not(.disabled):hover {
      background: var(--sidebar-highlight);
      transform: translateY(-2px);
    }
  }

  .platform-card:not(.disabled):active {
    transform: translateY(0);
  }

  .platform-card:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .platform-card.disabled {
    opacity: 0.4;
    cursor: default;
  }

  .card-icon {
    width: 52px;
    height: 52px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: calc(var(--border-radius) - 2px);
    background: color-mix(in srgb, var(--platform-color) 15%, transparent);
    color: var(--platform-color);
  }

  .card-name {
    font-size: 14.5px;
    font-weight: 500;
    color: var(--secondary);
  }

  .card-status {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--gray);
    min-height: 16px;
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-dot.connected {
    background: var(--green);
  }

  .status-dot.disconnected {
    background: var(--gray);
  }

  .status-dot.checking {
    background: var(--gray);
    animation: pulse 1s ease-in-out infinite;
  }

  .status-email {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 140px;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.3; }
    50% { opacity: 1; }
  }

  @media (max-width: 535px) {
    .platform-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .platform-card {
      transition: none;
    }
    .status-dot.checking {
      animation: none;
    }
  }
</style>
