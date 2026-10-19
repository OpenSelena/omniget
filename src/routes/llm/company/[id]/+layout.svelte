<script lang="ts">
  import V100NodeView from "$lib/v100/V100NodeView.svelte";
  import type { Snippet } from "svelte";

  let { children }: { children?: Snippet } = $props();
</script>

<V100NodeView
  nodeFile="5.DGp_VfM_.js"
  cssFile="5.DW0cyISk.css"
/>

{#if children}
  {@render children()}
{/if}
