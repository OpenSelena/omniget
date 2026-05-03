<script lang="ts">
  import MaintenanceCard from "$lib/study-components/MaintenanceCard.svelte";
  import { STUDY_NOTES_ENABLED } from "$lib/study-feature-flags";
  import { t } from "$lib/i18n";

  let { children } = $props();
</script>

{#if !STUDY_NOTES_ENABLED}
  <MaintenanceCard
    feature={$t("study.maintenance.feature_notes") as string}
    detail={$t("study.maintenance.detail_notes") as string}
  />
{:else}
  {@render children?.()}
{/if}
