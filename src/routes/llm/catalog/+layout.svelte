<script lang="ts">
  import V100NodeView from "$lib/v100/V100NodeView.svelte";
  import type { Snippet } from "svelte";

  let { children }: { children?: Snippet } = $props();
</script>

<V100NodeView
  nodeFile="4.C2uMo1Gn.js"
  cssFile="4.D6qBufHT.css"
/>

{#if children}
  {@render children()}
{/if}
