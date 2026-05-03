<script lang="ts">
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import { isDebugEnabled } from "$lib/stores/debug-store.svelte";
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();

  const ALL_TABS = [
    { href: "/about/terms", labelKey: "about.tab.terms" },
    { href: "/about/project", labelKey: "about.tab.project" },
    { href: "/about/roadmap", labelKey: "about.tab.roadmap" },
    { href: "/about/changelog", labelKey: "about.tab.changelog" },
    { href: "/about/debug", labelKey: "about.tab.debug" },
  ] as const;

  let visibleTabs = $derived(
    isDebugEnabled() ? ALL_TABS : ALL_TABS.filter((t) => t.href !== "/about/debug")
  );

  function isActive(href: string): boolean {
    return page.url.pathname === href;
  }
</script>

<div class="about-layout">
  <nav class="about-tabs">
    {#each visibleTabs as tab}
      <a
        href={tab.href}
        class="about-tab"
        class:active={isActive(tab.href)}
      >
        {$t(tab.labelKey)}
      </a>
    {/each}
  </nav>

  <div class="about-content">
    {@render children()}
  </div>
</div>

<style>
  .about-layout {
    display: flex;
    flex-direction: column;
    max-width: 520px;
    margin: 0 auto;
    padding-top: calc(var(--padding) * 1.5);
    gap: calc(var(--padding) * 1.5);
  }

  .about-tabs {
    display: flex;
    gap: 4px;
    background: var(--surface);
    border-radius: var(--radius-md);
    padding: 4px;
    box-shadow: var(--elev-1);
  }

  .about-tab {
    flex: 1;
    text-align: center;
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--text-dim);
    border-radius: var(--radius-sm);
    cursor: pointer;
    user-select: none;
    text-decoration: none;
    transition: color var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out);
  }

  @media (hover: hover) {
    .about-tab:hover:not(.active) {
      color: var(--text);
      background: var(--surface-hi);
    }
  }

  .about-tab.active {
    background: var(--accent-soft);
    color: var(--accent);
    cursor: default;
  }

  @media (prefers-reduced-motion: reduce) {
    .about-tab {
      transition: none;
    }
  }

  .about-tab:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .about-content {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 1.5);
    padding-bottom: calc(var(--padding) * 3);
  }
</style>
