<script lang="ts">
  import { t } from "$lib/i18n";

  let {
    selectedQuality = $bindable("best"),
    selectedFormatId,
    availableHeights = null as number[] | null,
    hasAudioOnly = false,
  } = $props();

  // Full ordered list of resolution options (label, yt-dlp quality value, height for filtering)
  const RESOLUTION_OPTIONS = [
    { value: "best", key: "omnibox.quality_best", height: Infinity },
    { value: "2160p", key: "omnibox.quality_2160p", height: 2160 },
    { value: "1440p", key: "omnibox.quality_1440p", height: 1440 },
    { value: "1080p", key: "omnibox.quality_1080p", height: 1080 },
    { value: "720p", key: "omnibox.quality_720p", height: 720 },
    { value: "480p", key: "omnibox.quality_480p", height: 480 },
    { value: "360p", key: "omnibox.quality_360p", height: 360 },
  ];

  // Derive which resolution pills to show.
  // Before formats load: show standard set (Best + 1080p + 720p + 480p + 360p).
  // After formats load: show only resolutions actually available, plus Best always.
  let visibleOptions = $derived.by(() => {
    if (!availableHeights) {
      // Pre-fetch fallback: strip 2K / 4K from hardcoded list to keep UI tidy
      return RESOLUTION_OPTIONS.filter(o => o.height === Infinity || o.height <= 1080);
    }
    return RESOLUTION_OPTIONS.filter(o => {
      if (o.height === Infinity) return true; // "Best" always shown
      return availableHeights!.some(h => h === o.height);
    });
  });

  // i18n keys that may not exist yet — provide a fallback label
  function qualityLabel(value: string, key: string): string {
    const translated = $t(key);
    // If no translation key exists (e.g. 2160p, 1440p), build a label from value
    if (translated === key) return value;
    return translated;
  }
</script>

{#if !selectedFormatId}
  <div class="quality-group">
    <span class="quality-label">{$t('omnibox.quality')}</span>
    <div class="quality-pills">
      {#each visibleOptions as q (q.value)}
        <button
          class="quality-pill"
          class:active={selectedQuality === q.value}
          onclick={() => { selectedQuality = q.value; }}
        >
          {qualityLabel(q.value, q.key)}
        </button>
      {/each}
      {#if hasAudioOnly}
        <button
          class="quality-pill quality-pill--audio"
          class:active={selectedQuality === "audio"}
          onclick={() => { selectedQuality = "audio"; }}
        >
          {$t('omnibox.quality_audio')}
        </button>
      {/if}
    </div>
    {#if availableHeights}
      <span class="quality-hint">{$t('omnibox.quality_from_formats')}</span>
    {/if}
  </div>
{/if}

<style>
  .quality-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .quality-label {
    font-size: var(--text-sm);
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

  .quality-pill--audio {
    border-style: dashed;
    border-color: var(--content-border);
  }

  .quality-pill--audio.active {
    border-style: solid;
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

  .quality-hint {
    font-size: 10.5px;
    color: var(--gray);
    opacity: 0.7;
  }
</style>
