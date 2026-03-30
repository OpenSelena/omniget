<script lang="ts">
  import { t } from "$lib/i18n";

  let { selectedQuality = $bindable("best"), selectedFormatId } = $props();

  const qualities = [
    { value: "best", key: "omnibox.quality_best" },
    { value: "1080p", key: "omnibox.quality_1080p" },
    { value: "720p", key: "omnibox.quality_720p" },
    { value: "480p", key: "omnibox.quality_480p" },
    { value: "360p", key: "omnibox.quality_360p" },
  ];
</script>

{#if !selectedFormatId}
  <div class="quality-group">
    <span class="quality-label">{$t('omnibox.quality')}</span>
    <div class="quality-pills">
      {#each qualities as q}
        <button
          class="quality-pill"
          class:active={selectedQuality === q.value}
          onclick={() => { selectedQuality = q.value; }}
        >
          {$t(q.key)}
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  .quality-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .quality-label {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--gray);
  }

  .quality-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .quality-pill {
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 500;
    color: var(--gray);
    background: var(--button);
    border: 1px solid transparent;
    border-radius: calc(var(--border-radius) - 2px);
    cursor: pointer;
    white-space: nowrap;
    box-shadow: var(--button-box-shadow);
  }

  .quality-pill.active {
    background: var(--button-elevated);
    color: var(--secondary);
    border-color: var(--cta);
  }

  @media (hover: hover) {
    .quality-pill:not(.active):hover {
      background: var(--button-hover);
      color: var(--secondary);
    }
  }

  .quality-pill:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
