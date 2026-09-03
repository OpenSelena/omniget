<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import {
    hasInstances,
    resetConnect,
    connectInstance,
    getConnectStep,
    DEFAULT_INSTANCE_URL,
  } from "$lib/stores/omnidisc-store.svelte";
  import ConnectInstanceForm from "$components/omnidisc/ConnectInstanceForm.svelte";

  let hasAny = $derived(hasInstances());
  let addRequested = $derived(page.url.searchParams.get("add") === "1");
  let showForm = $state(false);
  let onboarding = $derived(!hasAny || addRequested || showForm);

  onMount(() => {
    if (!hasInstances() && !addRequested && getConnectStep() === "idle") {
      void connectInstance(DEFAULT_INSTANCE_URL);
    }
  });

  function done() {
    showForm = false;
    if (addRequested) goto("/omnidisc");
  }

  function startAdd() {
    resetConnect();
    showForm = true;
  }
</script>

<div class="od-home" class:onboarding>
  {#if onboarding}
    <section class="hero">
      <div class="hero-mark" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a8 8 0 0 1-8 8H7l-4 3V12a8 8 0 0 1 8-8h2a8 8 0 0 1 8 8z" />
        </svg>
      </div>
      <h1 class="hero-title">{$t("omnidisc.connect.title")}</h1>
      <p class="hero-sub">{$t("omnidisc.connect.subtitle")}</p>
      <ConnectInstanceForm onDone={done} />
      {#if hasAny}
        <button type="button" class="link" onclick={done}>{$t("common.cancel")}</button>
      {/if}
    </section>
  {:else}
    <section class="pick">
      <h2>{$t("omnidisc.home.pick_channel_title")}</h2>
      <p>{$t("omnidisc.home.pick_channel_body")}</p>
      <button type="button" class="secondary" onclick={startAdd}>{$t("omnidisc.connect.add_another")}</button>
    </section>
  {/if}
</div>

<style>
  .od-home {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-6) var(--space-5);
    overflow-y: auto;
  }

  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    max-width: 420px;
    text-align: center;
  }

  .hero-mark {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-lg);
    display: grid;
    place-items: center;
    background: var(--accent-soft);
    color: var(--accent);
    margin-bottom: var(--space-2);
  }

  .hero-title {
    margin: 0;
    font-size: var(--text-xl);
    font-weight: 600;
    color: var(--text);
  }

  .hero-sub {
    margin: 0 0 var(--space-3);
    font-size: var(--text-base);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .hero :global(.connect) {
    text-align: left;
  }

  .link {
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
    padding: var(--space-2);
  }

  .link:hover {
    color: var(--text);
  }

  .pick {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    text-align: center;
    max-width: 360px;
  }

  .pick h2 {
    margin: 0;
    font-size: var(--text-lg);
    color: var(--text);
  }

  .pick p {
    margin: 0 0 var(--space-3);
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .secondary {
    padding: 8px var(--space-4);
    border-radius: var(--radius-md);
    border: none;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .secondary:hover {
    background: var(--surface-hi);
  }

  .secondary:focus-visible,
  .link:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
