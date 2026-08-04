<script lang="ts">
  import MaintenanceCard from "$lib/study-components/MaintenanceCard.svelte";
  import { STUDY_ACHIEVEMENTS_ENABLED } from "$lib/study-feature-flags";
  import { t } from "$lib/i18n";

  let { children } = $props();
</script>

{#if !STUDY_ACHIEVEMENTS_ENABLED}
  <MaintenanceCard
    feature={$t("study.maintenance.feature_achievements") as string}
    detail={$t("study.maintenance.detail_achievements") as string}
  />
{:else}
  {@render children?.()}
{/if}
