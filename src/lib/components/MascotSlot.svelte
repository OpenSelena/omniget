<script lang="ts">
  import { onMount } from "svelte";
  import Mascot from "./Mascot.svelte";
  import {
    getDisplayPrefs,
    loadDisplayPrefs,
    motionEnabled as motionEnabledFn,
    petsLoaded,
  } from "$lib/pets/display-prefs.svelte";
  import { getStateForEvent, type AnkiMoodEvent } from "$lib/pets/mood";
  import type { PetStateId } from "$lib/pets/types";

  type Fallback =
    | "none"
    | "check"
    | "spinner"
    | "warn"
    | "error"
    | "celebrate"
    | "rest"
    | "wave";

  let {
    slug,
    state: stateProp,
    event,
    scale = 1,
    fallback = "none" as Fallback,
    fallbackLabel = "",
    overrideEnabled,
  }: {
    slug?: string | null;
    state?: PetStateId;
    event?: AnkiMoodEvent;
    scale?: number;
    fallback?: Fallback;
    fallbackLabel?: string;
    overrideEnabled?: boolean;
  } = $props();

  let prefsLoaded = $state(petsLoaded());

  onMount(() => {
    if (!prefsLoaded) {
      void loadDisplayPrefs().then(() => {
        prefsLoaded = true;
      });
    }
  });

  const resolvedState = $derived<PetStateId>(
    stateProp ?? (event ? getStateForEvent(event) : "idle"),
  );

  const enabledByPrefs = $derived(
    overrideEnabled !== undefined ? overrideEnabled : getDisplayPrefs().enabled,
  );

  const showMascot = $derived(prefsLoaded && enabledByPrefs && !!slug);
  const showFallback = $derived(prefsLoaded && (!enabledByPrefs || !slug) && fallback !== "none");
</script>

{#if showMascot}
  {#if motionEnabledFn()}
    <Mascot slug={slug!} state={resolvedState} {scale} />
  {:else}
    <Mascot slug={slug!} state={resolvedState} {scale} paused />
  {/if}
{:else if showFallback}
  <span class="slot-fallback fb-{fallback}" aria-label={fallbackLabel || undefined} role={fallbackLabel ? "img" : undefined}>
    {#if fallback === "check"}
      <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="9" />
        <path d="M8 12.5l2.5 2.5L16 9.5" />
      </svg>
    {:else if fallback === "spinner"}
      <span class="spin" aria-hidden="true"></span>
    {:else if fallback === "warn"}
      <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M12 9v4" />
        <path d="M12 17h.01" />
        <path d="M10.3 3.7l-8 14a2 2 0 0 0 1.7 3h16a2 2 0 0 0 1.7-3l-8-14a2 2 0 0 0-3.4 0z" />
      </svg>
    {:else if fallback === "error"}
      <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="9" />
        <path d="M15 9l-6 6M9 9l6 6" />
      </svg>
    {:else if fallback === "celebrate"}
      <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M12 3v3" />
        <path d="M5 5l2 2" />
        <path d="M19 5l-2 2" />
        <path d="M3 12h3" />
        <path d="M21 12h-3" />
        <path d="M12 9a4 4 0 0 1 4 4v6H8v-6a4 4 0 0 1 4-4z" />
      </svg>
    {:else if fallback === "rest"}
      <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M21 12.79A9 9 0 1 1 11.21 3a7 7 0 0 0 9.79 9.79z" />
      </svg>
    {:else if fallback === "wave"}
      <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M3 12c2 2 4 2 6 0s4-2 6 0 4 2 6 0" />
      </svg>
    {/if}
    {#if fallbackLabel}<span class="sr-only">{fallbackLabel}</span>{/if}
  </span>
{/if}

<style>
  .slot-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }

  .slot-fallback.fb-check,
  .slot-fallback.fb-celebrate,
  .slot-fallback.fb-wave {
    color: var(--success);
  }

  .slot-fallback.fb-warn {
    color: var(--warning);
  }

  .slot-fallback.fb-error {
    color: var(--error);
  }

  .spin {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--surface-hi);
    border-top-color: var(--accent);
    animation: slot-spin 0.8s linear infinite;
  }

  @keyframes slot-spin {
    to { transform: rotate(360deg); }
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>
