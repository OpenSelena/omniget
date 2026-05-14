<script lang="ts">
  import MaintenanceCard from "$lib/study-components/MaintenanceCard.svelte";
  import { STUDY_PROGRESS_ENABLED } from "$lib/study-feature-flags";
  import { t } from "$lib/i18n";

  let { children } = $props();
</script>

{#if !STUDY_PROGRESS_ENABLED}
  <MaintenanceCard
    feature={$t("study.maintenance.feature_progress") as string}
    detail={$t("study.maintenance.detail_progress") as string}
  />
{:else}
  {@render children?.()}
{/if}
