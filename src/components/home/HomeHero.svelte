<script lang="ts">
  import Mascot from "$components/mascot/Mascot.svelte";
  import Confetti from "$components/celebrate/Confetti.svelte";

  type MascotEmotion = "idle" | "downloading" | "error" | "stalled" | "queue" | "complete" | "amazed";

  let {
    emotion,
    compact = false,
    stage = false,
    bubbleText,
    celebrate = false,
  }: {
    emotion: MascotEmotion;
    compact?: boolean;
    stage?: boolean;
    bubbleText?: string;
    celebrate?: boolean;
  } = $props();
</script>

<div class="home-hero" class:compact class:stage>
  <Mascot {emotion} compact={compact && !stage} stage={stage} bubbleText={stage ? undefined : bubbleText} />
  <Confetti active={celebrate} />
</div>

<style>
  .home-hero {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    width: 100%;
    max-height: none;
    overflow: visible;
  }

  .home-hero.compact {
    max-height: none;
  }

  .home-hero.stage {
    width: auto;
  }
</style>
