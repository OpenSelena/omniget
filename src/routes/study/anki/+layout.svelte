<script lang="ts">
  import Sidebar from "$lib/study-components/anki/Sidebar.svelte";
  import MaintenanceCard from "$lib/study-components/MaintenanceCard.svelte";
  import { STUDY_ANKI_ENABLED } from "$lib/study-feature-flags";
  import { t } from "$lib/i18n";

  let { children } = $props();
</script>

{#if !STUDY_ANKI_ENABLED}
  <MaintenanceCard
    feature={$t("study.maintenance.feature_anki") as string}
    detail={$t("study.maintenance.detail_anki") as string}
  />
{:else}
  <div class="anki-shell">
    <Sidebar />
    <div class="anki-content">
      {@render children?.()}
    </div>
  </div>
{/if}

<style>
  .anki-shell {
    display: flex;
    gap: var(--space-4);
    align-items: flex-start;
    width: 100%;
    max-width: 1200px;
    margin: 0 auto;
    padding: 0 var(--space-3) var(--space-5);
  }

  .anki-content {
    flex: 1;
    min-width: 0;
  }

  @media (max-width: 720px) {
    .anki-shell {
      gap: var(--space-2);
      padding: 0 var(--space-2) var(--space-4);
    }
  }
</style>
