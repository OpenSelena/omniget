<script lang="ts">
  import { onMount } from "svelte";
  import { ankiPetsList, ankiPetActive, ankiPetSetActive, type AnkiPet } from "$lib/anki-bridge";
  import Mascot from "$lib/components/Mascot.svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";

  let pets = $state<AnkiPet[]>([]);
  let active = $state<AnkiPet | null>(null);
  let loading = $state(true);
  let busy = $state(false);

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    try {
      const [list, current] = await Promise.all([ankiPetsList(), ankiPetActive()]);
      pets = list;
      active = current;
    } finally {
      loading = false;
    }
  }

  async function pick(pet: AnkiPet) {
    if (busy || pet.slug === active?.slug) return;
    busy = true;
    try {
      const updated = await ankiPetSetActive(pet.slug);
      active = updated;
      showToast("info", `Mascote: ${updated.display_name}`);
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : (e?.message ?? "Erro"));
    } finally {
      busy = false;
    }
  }
</script>

<section class="pets-section">
  <header class="pets-head">
    <h2>Mascote</h2>
    <p class="muted">Quem te acompanha enquanto você revisa.</p>
  </header>

  {#if loading}
    <div class="loader"><div class="ring"></div></div>
  {:else}
    <div class="pets-grid">
      {#each pets as pet (pet.slug)}
        {@const isActive = pet.slug === active?.slug}
        <button
          type="button"
          class="pet-card"
          class:pet-active={isActive}
          onclick={() => pick(pet)}
          disabled={busy}
        >
          <Mascot slug={pet.slug} state="idle" scale={0.45} />
          <div class="pet-info">
            <strong>{pet.display_name}</strong>
            <span class="pet-desc">{pet.description}</span>
          </div>
          {#if isActive}<span class="active-badge">Ativo</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  .pets-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .pets-head h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  .muted {
    margin: 4px 0 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .loader {
    display: flex;
    justify-content: center;
    padding: var(--space-5);
  }
  .ring {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 3px solid var(--surface-mut);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .pets-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: var(--space-3);
  }

  .pet-card {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-3);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    color: inherit;
    transition: border-color var(--duration-fast) var(--ease-out),
      transform var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
  }
  .pet-card:hover:not(:disabled) {
    border-color: var(--accent);
    transform: translateY(-2px);
    box-shadow: var(--elev-2);
  }
  .pet-card:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .pet-card.pet-active {
    border-color: var(--success);
    background: linear-gradient(135deg, color-mix(in srgb, var(--success) 14%, transparent), transparent 60%);
  }

  .pet-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .pet-info strong {
    font-size: var(--text-sm);
    font-weight: 600;
  }
  .pet-desc {
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.4;
  }
  .pet-vibes {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    margin-top: 4px;
  }

  .active-badge {
    position: absolute;
    top: 8px;
    right: 8px;
    background: var(--success);
    color: white;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 8px;
    border-radius: 999px;
  }
</style>
