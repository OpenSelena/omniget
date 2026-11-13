<script lang="ts">
  import { untrack } from "svelte";

  type MascotEmotion = "idle" | "downloading" | "error" | "stalled" | "queue" | "complete" | "amazed";

  function emotionToSrc(e: MascotEmotion): string {
    return `/mascot/${e}.png`;
  }

  let {
    emotion = "idle",
    compact = false,
    stage = false,
    bubbleText,
  }: {
    emotion?: MascotEmotion;
    compact?: boolean;
    stage?: boolean;
    bubbleText?: string;
  } = $props();

  let currentSrc = $state("/mascot/idle.png");
  let showCurrent = $state(false);
  let errored = $state(false);
  let transitionTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const target = emotionToSrc(emotion);
    if (target === currentSrc) {
      if (!untrack(() => errored)) showCurrent = true;
      return;
    }

    if (transitionTimer) {
      clearTimeout(transitionTimer);
      transitionTimer = null;
    }

    if (!untrack(() => showCurrent)) {
      currentSrc = target;
      return;
    }

    showCurrent = false;
    transitionTimer = setTimeout(() => {
      transitionTimer = null;
      currentSrc = target;
    }, 200);

    return () => {
      if (transitionTimer) {
        clearTimeout(transitionTimer);
        transitionTimer = null;
      }
    };
  });

  function onCurrentLoad() {
    showCurrent = true;
    errored = false;
  }

  function onCurrentError() {
    if (!showCurrent) errored = true;
  }
</script>

<div class="mascot" class:compact class:stage>
  {#if !errored}
    <img
      src={currentSrc}
      alt="LibreGet mascot"
      class="mascot-img"
      class:visible={showCurrent}
      onload={onCurrentLoad}
      onerror={onCurrentError}
      draggable="false"
    />
  {/if}
  {#if errored}
    <svg
      class="mascot-fallback"
      viewBox="0 0 64 64"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <rect x="12" y="8" width="40" height="48" rx="6" />
      <path d="M32 22v14m0 0l-6-6m6 6l6-6" />
      <path d="M20 48h24" />
    </svg>
  {/if}
  {#if bubbleText}
    <div class="mascot-bubble" role="status" aria-live="polite">
      <span>{bubbleText}</span>
    </div>
  {/if}
</div>

<style>
  .mascot {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    min-height: 100px;
    position: relative;
    transition: min-height 0.2s ease;
  }

  .mascot.compact {
    min-height: 72px;
  }

  .mascot.stage {
    min-height: 128px;
  }

  .mascot-img {
    width: 100px;
    height: 100px;
    aspect-ratio: 1;
    object-fit: contain;
    opacity: 0;
    transform: scale(0.9);
    transition:
      opacity var(--duration-base) var(--ease-out),
      transform var(--duration-bounce) var(--ease-spring),
      width var(--duration-base) var(--ease-out),
      height var(--duration-base) var(--ease-out);
    pointer-events: none;
    user-select: none;
  }

  .mascot.compact .mascot-img,
  .mascot.compact .mascot-fallback {
    width: 72px;
    height: 72px;
  }

  .mascot.stage .mascot-img,
  .mascot.stage .mascot-fallback {
    width: 128px;
    height: 128px;
  }

  .mascot-img.visible {
    opacity: 1;
    transform: scale(1);
  }

  .mascot-fallback {
    width: 100px;
    height: 100px;
    color: var(--gray);
    opacity: 0.5;
    pointer-events: none;
  }

  .mascot-bubble {
    background: var(--popup-bg);
    border-radius: var(--radius-lg);
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-sm);
    color: var(--text);
    max-width: 280px;
    text-align: center;
    margin-top: var(--space-2);
    position: relative;
    box-shadow: var(--elev-2);
    animation: bubbleFadeIn var(--duration-base) var(--ease-out);
  }

  .mascot-bubble::before {
    content: '';
    position: absolute;
    top: -6px;
    left: 50%;
    transform: translateX(-50%);
    width: 12px;
    height: 6px;
    background: var(--popup-bg);
    clip-path: polygon(50% 0%, 0% 100%, 100% 100%);
  }

  @keyframes bubbleFadeIn {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  @media (prefers-reduced-motion: reduce) {
    .mascot-bubble {
      animation: none;
    }

    .mascot,
    .mascot-img {
      transition: opacity var(--duration-base) var(--ease-out);
      transform: none;
    }

  }
</style>
