<script lang="ts">
  import { t } from "$lib/i18n";
  type Props = {
    title: string;
    seeAllHref?: string | null;
    onSeeAll?: (() => void) | null;
  };
  let { title, seeAllHref = null, onSeeAll = null }: Props = $props();
</script>

<header class="nav-title">
  <h2>{title}</h2>
  {#if onSeeAll}
    <button type="button" class="see-all" onclick={() => onSeeAll?.()}>
      {$t("study.music.see_all")} <span class="arr" aria-hidden="true">→</span>
    </button>
  {:else if seeAllHref}
    <a class="see-all" href={seeAllHref}>
      {$t("study.music.see_all")} <span class="arr" aria-hidden="true">→</span>
    </a>
  {/if}
</header>

<style>
  .nav-title {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 800;
    letter-spacing: -0.01em;
    color: rgba(255, 255, 255, 0.95);
  }
  .see-all {
    background: transparent;
    border: 0;
    padding: 4px 8px;
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.6);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    text-decoration: none;
    transition: color 160ms ease;
  }
  .see-all:hover {
    color: white;
  }
  .arr {
    display: inline-block;
    margin-left: 4px;
    transition: transform 160ms ease;
  }
  .see-all:hover .arr {
    transform: translateX(2px);
  }
  @media (prefers-reduced-motion: reduce) {
    .see-all, .arr { transition: none; }
  }
</style>
