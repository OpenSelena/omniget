<script lang="ts">
  import { onMount } from "svelte";

  interface Props {
    nodeFile: string;
    cssFile?: string;
    props?: Record<string, any>;
  }

  let { nodeFile, cssFile, props = {} }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let errorMsg = $state<string | null>(null);
  let isMounted = $state(false);

  onMount(() => {
    let unmountFn: (() => void) | null = null;

    async function load() {
      if (!container) return;

      try {
        if (cssFile) {
          const href = `/_app/immutable/assets/${cssFile}`;
          if (!document.querySelector(`link[href="${href}"]`)) {
            const link = document.createElement("link");
            link.rel = "stylesheet";
            link.href = href;
            document.head.appendChild(link);
          }
        }

        const modUrl = `/_app/immutable/nodes/${nodeFile}`;
        // Dynamic import bypasses Vite build-time bundling
        const mod = await import(/* @vite-ignore */ modUrl);

        if (mod && typeof mod.component === "function") {
          unmountFn = mod.component(container, props);
          isMounted = true;
        } else {
          errorMsg = `Module ${nodeFile} does not export a component function.`;
        }
      } catch (err: any) {
        console.error(`Failed to load v100 node ${nodeFile}:`, err);
        errorMsg = err?.message || String(err);
      }
    }

    load();

    return () => {
      if (typeof unmountFn === "function") {
        try {
          unmountFn();
        } catch {
          // ignore
        }
      }
    };
  });
</script>

<div class="v100-node-wrapper" bind:this={container}>
  {#if errorMsg}
    <div class="v100-node-error">
      <p>Error loading component: {errorMsg}</p>
    </div>
  {/if}
</div>

<style>
  .v100-node-wrapper {
    width: 100%;
    min-height: 100%;
    display: contents;
  }
  .v100-node-error {
    padding: 16px;
    margin: 16px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 8px;
    color: #ef4444;
    font-size: 13px;
  }
</style>
