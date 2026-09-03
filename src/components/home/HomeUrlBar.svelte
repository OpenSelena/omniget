<script lang="ts">
  import OmniboxInput from "$components/omnibox/OmniboxInput.svelte";
  import type { HomeInputMode } from "$lib/home/omnibox-controller";
  import { t } from "$lib/i18n";

  let {
    url = $bindable(""),
    mode = $bindable<HomeInputMode>("url"),
    variant = "bar",
    onInput,
    onModeChange,
  }: {
    url?: string;
    mode?: HomeInputMode;
    variant?: "bar" | "stage";
    onInput: () => void;
    onModeChange?: (mode: HomeInputMode) => void;
  } = $props();

  const modes: HomeInputMode[] = ["url", "batch", "torrent", "p2p"];

  function setMode(next: HomeInputMode) {
    mode = next;
    onModeChange?.(next);
  }
</script>

<div class="home-url-bar" class:stage={variant === "stage"}>
  {#if variant === "stage"}
    <div class="home-capabilities" role="tablist">
      {#each modes as m}
        <button
          type="button"
          class="home-capability"
          class:active={mode === m}
          role="tab"
          aria-selected={mode === m}
          onclick={() => setMode(m)}
        >
          <span class="home-capability-icon" aria-hidden="true">
            <svg viewBox="0 0 20 20" width="18" height="18" fill="currentColor">
              {#if m === "url"}
                <path d="M8.2 4.35a3.4 3.4 0 0 1 4.8 0l.85.85a1 1 0 1 1-1.42 1.4l-.85-.85a1.4 1.4 0 1 0-1.98 1.98l1.2 1.2a1 1 0 0 1-1.42 1.42l-1.2-1.2a3.4 3.4 0 0 1 0-4.8zm4.38 3.05a1 1 0 0 1 1.42 0l1.2 1.2a3.4 3.4 0 0 1-4.8 4.8l-.85-.85a1 1 0 1 1 1.42-1.4l.85.85a1.4 1.4 0 1 0 1.98-1.98l-1.2-1.2a1 1 0 0 1 0-1.42z" />
              {:else if m === "batch"}
                <path d="M3.4 4.4A1.4 1.4 0 0 1 4.8 3h10.4A1.4 1.4 0 0 1 16.6 4.4v1.2A1.4 1.4 0 0 1 15.2 7H4.8A1.4 1.4 0 0 1 3.4 5.6V4.4z" />
                <path d="M3.4 9.4A1.4 1.4 0 0 1 4.8 8h10.4A1.4 1.4 0 0 1 16.6 9.4v1.2A1.4 1.4 0 0 1 15.2 12H4.8A1.4 1.4 0 0 1 3.4 10.6V9.4z" />
                <path d="M3.4 14.4A1.4 1.4 0 0 1 4.8 13h10.4A1.4 1.4 0 0 1 16.6 14.4v1.2A1.4 1.4 0 0 1 15.2 17H4.8A1.4 1.4 0 0 1 3.4 15.6v-1.2z" />
              {:else if m === "torrent"}
                <path d="M10 2.4a1.1 1.1 0 0 1 1.1 1.1v6.05l1.55-1.55a1.1 1.1 0 1 1 1.56 1.56l-3.4 3.4a1.1 1.1 0 0 1-1.56 0l-3.4-3.4A1.1 1.1 0 1 1 7.4 8l1.5 1.55V3.5A1.1 1.1 0 0 1 10 2.4z" />
                <path d="M4.2 13.6c0-.6.5-1.1 1.1-1.1h9.4c.6 0 1.1.5 1.1 1.1v1.7A2.2 2.2 0 0 1 13.6 17.5H6.4A2.2 2.2 0 0 1 4.2 15.3v-1.7z" />
              {:else}
                <path d="M6.2 4.3a1 1 0 0 1 1.4 0l3.1 3.1 1.55-1.55A1.4 1.4 0 0 1 14.25 5h1.9A1.35 1.35 0 0 1 17.5 6.35v1.9a1.4 1.4 0 0 1-.41 1l-1.55 1.55 3.16 3.16a1 1 0 0 1-1.42 1.42l-3.15-3.16-1.55 1.55a1.4 1.4 0 0 1-1 .41h-1.9A1.35 1.35 0 0 1 8.33 12.8V10.9c0-.37.15-.73.41-1l1.55-1.55L7.2 5.7a1 1 0 0 1 0-1.4z" />
                <path d="M3.4 12.15a1 1 0 0 1 1.42 0l2.05 2.05-1.12 1.12A1 1 0 1 1 4.33 16.74L3.4 15.8a1 1 0 0 1 0-1.42z" />
              {/if}
            </svg>
          </span>
          <span class="home-capability-label">{$t(`home.mode_${m}`)}</span>
        </button>
      {/each}
    </div>
  {:else}
    <div class="mac-segmented" role="tablist">
      {#each modes as m}
        <button
          type="button"
          class="mac-segmented-btn"
          class:active={mode === m}
          role="tab"
          aria-selected={mode === m}
          onclick={() => setMode(m)}
        >
          <svg class="mode-glyph" viewBox="0 0 20 20" width="13" height="13" fill="currentColor" aria-hidden="true">
            {#if m === "url"}
              <path d="M8.2 4.35a3.4 3.4 0 0 1 4.8 0l.85.85a1 1 0 1 1-1.42 1.4l-.85-.85a1.4 1.4 0 1 0-1.98 1.98l1.2 1.2a1 1 0 0 1-1.42 1.42l-1.2-1.2a3.4 3.4 0 0 1 0-4.8zm4.38 3.05a1 1 0 0 1 1.42 0l1.2 1.2a3.4 3.4 0 0 1-4.8 4.8l-.85-.85a1 1 0 1 1 1.42-1.4l.85.85a1.4 1.4 0 1 0 1.98-1.98l-1.2-1.2a1 1 0 0 1 0-1.42z" />
            {:else if m === "batch"}
              <path d="M3.4 4.4A1.4 1.4 0 0 1 4.8 3h10.4A1.4 1.4 0 0 1 16.6 4.4v1.2A1.4 1.4 0 0 1 15.2 7H4.8A1.4 1.4 0 0 1 3.4 5.6V4.4z" />
              <path d="M3.4 9.4A1.4 1.4 0 0 1 4.8 8h10.4A1.4 1.4 0 0 1 16.6 9.4v1.2A1.4 1.4 0 0 1 15.2 12H4.8A1.4 1.4 0 0 1 3.4 10.6V9.4z" />
              <path d="M3.4 14.4A1.4 1.4 0 0 1 4.8 13h10.4A1.4 1.4 0 0 1 16.6 14.4v1.2A1.4 1.4 0 0 1 15.2 17H4.8A1.4 1.4 0 0 1 3.4 15.6v-1.2z" />
            {:else if m === "torrent"}
              <path d="M10 2.4a1.1 1.1 0 0 1 1.1 1.1v6.05l1.55-1.55a1.1 1.1 0 1 1 1.56 1.56l-3.4 3.4a1.1 1.1 0 0 1-1.56 0l-3.4-3.4A1.1 1.1 0 1 1 7.4 8l1.5 1.55V3.5A1.1 1.1 0 0 1 10 2.4z" />
              <path d="M4.2 13.6c0-.6.5-1.1 1.1-1.1h9.4c.6 0 1.1.5 1.1 1.1v1.7A2.2 2.2 0 0 1 13.6 17.5H6.4A2.2 2.2 0 0 1 4.2 15.3v-1.7z" />
            {:else}
              <path d="M6.2 4.3a1 1 0 0 1 1.4 0l3.1 3.1 1.55-1.55A1.4 1.4 0 0 1 14.25 5h1.9A1.35 1.35 0 0 1 17.5 6.35v1.9a1.4 1.4 0 0 1-.41 1l-1.55 1.55 3.16 3.16a1 1 0 0 1-1.42 1.42l-3.15-3.16-1.55 1.55a1.4 1.4 0 0 1-1 .41h-1.9A1.35 1.35 0 0 1 8.33 12.8V10.9c0-.37.15-.73.41-1l1.55-1.55L7.2 5.7a1 1 0 0 1 0-1.4z" />
              <path d="M3.4 12.15a1 1 0 0 1 1.42 0l2.05 2.05-1.12 1.12A1 1 0 1 1 4.33 16.74L3.4 15.8a1 1 0 0 1 0-1.42z" />
            {/if}
          </svg>
          {$t(`home.mode_${m}`)}
        </button>
      {/each}
    </div>
  {/if}
  {#if mode === "url"}
    <OmniboxInput bind:url onInput={onInput} prominent={variant === "stage"} />
  {/if}
</div>

<style>
  .home-url-bar {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    width: 100%;
  }

  .home-url-bar.stage {
    align-items: center;
    gap: var(--space-5);
    max-width: 560px;
  }

  .mode-glyph {
    flex-shrink: 0;
    display: block;
  }
</style>
