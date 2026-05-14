<script lang="ts">
  import MaintenanceCard from "$lib/study-components/MaintenanceCard.svelte";
  import { STUDY_FOCUS_ENABLED } from "$lib/study-feature-flags";
  import { t } from "$lib/i18n";

  let { children } = $props();
</script>

{#if !STUDY_FOCUS_ENABLED}
  <MaintenanceCard
    feature={$t("study.maintenance.feature_focus") as string}
    detail={$t("study.maintenance.detail_focus") as string}
  />
{:else}
  {@render children?.()}
{/if}
