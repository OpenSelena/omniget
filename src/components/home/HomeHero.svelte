<script lang="ts">
  import Mascot from "$components/mascot/Mascot.svelte";
  import Confetti from "$components/celebrate/Confetti.svelte";

  type MascotEmotion = "idle" | "downloading" | "error" | "stalled" | "queue" | "complete" | "amazed";

  let {
    emotion,
    compact = false,
    bubbleText,
    celebrate = false,
  }: {
    emotion: MascotEmotion;
    compact?: boolean;
    bubbleText?: string;
    celebrate?: boolean;
  } = $props();
</script>

<div class="home-hero">
  <Mascot {emotion} {compact} {bubbleText} />
  <Confetti active={celebrate} />
</div>

<style>
  .home-hero {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
</style>
