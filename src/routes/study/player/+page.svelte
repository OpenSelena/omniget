<script lang="ts">
  import { t } from "$lib/i18n";
  import PageHero from "$lib/study-components/PageHero.svelte";
</script>

<section class="study-page">
  <PageHero
    breadcrumb={[
      { label: $t("study.hub.library"), href: "/study/library" },
      { label: $t("study.player.title") },
    ]}
    title={$t("study.player.title")}
    subtitle={$t("study.player.placeholder")}
  />
</section>

<style>
  .study-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    width: 100%;
    max-width: 1100px;
    margin-inline: auto;
  }
</style>
