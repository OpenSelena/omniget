<script lang="ts">
  import { onMount } from "svelte";
  import SettingsField from "./SettingsField.svelte";
  import SettingsToggle from "./SettingsToggle.svelte";
  import PetsManager from "$lib/components/PetsManager.svelte";
  import {
    loadDisplayPrefs,
    setDisplayPrefs,
    type DisplayPrefs,
  } from "$lib/pets/display-prefs.svelte";

  let prefs = $state<DisplayPrefs | null>(null);
  let busy = $state(false);

  onMount(() => {
    void load();
  });

  async function load() {
    prefs = await loadDisplayPrefs();
  }

  async function update(patch: Partial<DisplayPrefs>) {
    if (busy) return;
    busy = true;
    try {
      prefs = await setDisplayPrefs(patch);
    } finally {
      busy = false;
    }
  }
</script>

<section class="tab">
  {#if prefs}
    <SettingsField
      label="Mostrar pets na interface"
      description="Mascotes aparecem em painéis, empty states e feedback. Desligado: nenhum mascote em qualquer lugar; suas preferências de pet ativo, animações e galeria continuam salvas."
    >
      <SettingsToggle
        value={prefs.enabled}
        onChange={(v) => update({ enabled: v })}
        ariaLabel="Mostrar pets"
      />
    </SettingsField>

    <SettingsField
      label="Animar mascote"
      description="Quando desligado, o mascote aparece em frame único parado. Útil em e-ink ou para reduzir movimento."
    >
      <SettingsToggle
        value={prefs.motion_enabled}
        onChange={(v) => update({ motion_enabled: v })}
        ariaLabel="Animar mascote"
      />
    </SettingsField>

    <div class="manager-block" class:gated={!prefs.enabled}>
      <PetsManager />
    </div>
  {/if}
</section>

<style>
  .tab {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .manager-block {
    margin-top: var(--space-3);
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  .manager-block.gated {
    opacity: 0.55;
  }
</style>
