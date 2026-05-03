<script lang="ts">
  import { onMount } from "svelte";
  import { PET_STATES } from "$lib/pets/states";
  import type { PetStateId } from "$lib/pets/types";
  import { petsResolveAssetUrl } from "$lib/pets/sync";

  let {
    slug,
    state: stateProp = "idle" as PetStateId,
    scale = 1,
    paused = false,
  }: {
    slug: string;
    state?: PetStateId;
    scale?: number;
    paused?: boolean;
  } = $props();

  const anim = $derived(PET_STATES[stateProp]);
  let resolved = $state<string | null>(null);

  onMount(() => {
    let active = true;
    (async () => {
      if (!slug) return;
      try {
        const url = await petsResolveAssetUrl(slug);
        if (active) resolved = url;
      } catch {
        if (active) resolved = null;
      }
    })();
    return () => {
      active = false;
    };
  });

  $effect(() => {
    if (!slug) {
      resolved = null;
      return;
    }
    let active = true;
    petsResolveAssetUrl(slug)
      .then((url) => {
        if (active) resolved = url;
      })
      .catch(() => {
        if (active) resolved = null;
      });
    return () => {
      active = false;
    };
  });
</script>

{#if resolved}
  <div class="pet-sprite-frame" style:--pet-scale={scale}>
    <div
      class="pet-sprite"
      class:paused
      style:--sprite-url="url({resolved})"
      style:--sprite-row={anim.row}
      style:--sprite-frames={anim.frames}
      style:--sprite-duration="{anim.durationMs}ms"
    ></div>
  </div>
{:else}
  <div class="pet-sprite-frame placeholder" style:--pet-scale={scale}></div>
{/if}

<style>
  .pet-sprite-frame {
    --pet-scale: 1;
    width: calc(192px * var(--pet-scale));
    height: calc(208px * var(--pet-scale));
    overflow: hidden;
    flex-shrink: 0;
  }

  .placeholder {
    background: var(--surface-hi);
    border-radius: var(--radius-sm);
  }

  .pet-sprite {
    --sprite-row: 0;
    --sprite-frames: 6;
    --sprite-duration: 1100ms;
    --sprite-y: calc(var(--sprite-row) * -208px);
    --sprite-end-x: calc(var(--sprite-frames) * -192px);
    width: 192px;
    height: 208px;
    background-image: var(--sprite-url);
    background-repeat: no-repeat;
    background-size: 1536px 1872px;
    image-rendering: pixelated;
    transform: scale(var(--pet-scale));
    transform-origin: top left;
    animation: pet-state var(--sprite-duration) steps(var(--sprite-frames)) infinite;
  }

  .pet-sprite.paused {
    animation-play-state: paused;
  }

  @keyframes pet-state {
    from { background-position: 0 var(--sprite-y); }
    to   { background-position: var(--sprite-end-x) var(--sprite-y); }
  }

  @media (prefers-reduced-motion: reduce) {
    .pet-sprite { animation: none; }
  }

  :global([data-theme^="eink-"]) .pet-sprite {
    animation: none;
    filter: grayscale(1) contrast(1.05);
  }
</style>
