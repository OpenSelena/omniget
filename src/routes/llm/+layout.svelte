<script lang="ts">
  import V100NodeView from "$lib/v100/V100NodeView.svelte";
  import type { Snippet } from "svelte";

  let { children }: { children?: Snippet } = $props();
</script>

<V100NodeView
  nodeFile="3.BpxRnUE8.js"
  cssFile="3.uvL4iGUh.css"
/>

{#if children}
  {@render children()}
{/if}
