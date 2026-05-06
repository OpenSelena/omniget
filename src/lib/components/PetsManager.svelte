<script lang="ts">
  import { onMount } from "svelte";
  import {
    petsGetLocalIndex,
    petsGetActive,
    petsSetActive,
    petsUninstall,
    petsForceRefresh,
    petsInstallBundle,
    petsOpenFolder,
    type LocalIndex,
    type LocalPetEntry,
  } from "$lib/pets/sync";
  import type { PetVibe } from "$lib/pets/types";
  import { VIBE_MOOD } from "$lib/pets/mood";
  import PetCard from "./PetCard.svelte";
  import SyncButton from "./SyncButton.svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";

  type Kind = "creature" | "object" | "character";

  let index = $state<LocalIndex | null>(null);
  let activeSlug = $state<string | null>(null);
  let loading = $state(true);
  let busySlug = $state<string | null>(null);
  let bundleBusy = $state(false);
  let query = $state("");
  let kindFilter = $state<Kind | "">("");
  let vibeFilter = $state<PetVibe | "">("");
  let lastSyncRelative = $state("");

  let page = $state(0);
  const PAGE_SIZE = 24;

  const ALL_VIBES: PetVibe[] = [
    "cozy", "calm", "playful", "cheerful", "focused", "mischievous",
    "heroic", "edgy", "mystical", "wholesome", "chaotic", "melancholic",
  ];

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    try {
      const [idx, active] = await Promise.all([petsGetLocalIndex(), petsGetActive()]);
      index = idx;
      activeSlug = active;
      lastSyncRelative = formatRelative(idx.last_synced_at);
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : (e?.message ?? "Erro"));
    } finally {
      loading = false;
    }
  }

  function formatRelative(iso: string | null): string {
    if (!iso) return "nunca";
    const then = new Date(iso).getTime();
    const diff = Math.max(0, Date.now() - then) / 1000;
    if (diff < 60) return "agora";
    if (diff < 3600) return `há ${Math.floor(diff / 60)} min`;
    if (diff < 86400) return `há ${Math.floor(diff / 3600)} h`;
    return `há ${Math.floor(diff / 86400)} d`;
  }

  function vibeMood(vibe: PetVibe): "positive" | "ambiguous" | "negative" {
    if (VIBE_MOOD.positive.includes(vibe)) return "positive";
    if (VIBE_MOOD.negative.includes(vibe)) return "negative";
    return "ambiguous";
  }

  const filtered = $derived.by<LocalPetEntry[]>(() => {
    if (!index) return [];
    const q = query.trim().toLowerCase();
    return index.pets.filter((p) => {
      if (q) {
        const hay = `${p.display_name} ${p.slug} ${p.tags.join(" ")} ${p.vibes.join(" ")}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      if (kindFilter && p.kind !== kindFilter) return false;
      if (vibeFilter && !p.vibes.includes(vibeFilter)) return false;
      return true;
    });
  });

  const totalPages = $derived(Math.max(1, Math.ceil(filtered.length / PAGE_SIZE)));
  const visible = $derived.by<LocalPetEntry[]>(() => {
    const start = page * PAGE_SIZE;
    return filtered.slice(start, start + PAGE_SIZE);
  });

  $effect(() => {
    void query;
    void kindFilter;
    void vibeFilter;
    page = 0;
  });

  $effect(() => {
    if (page >= totalPages) page = Math.max(0, totalPages - 1);
  });

  function nextPage() {
    if (page + 1 < totalPages) {
      page += 1;
      scrollToTop();
    }
  }
  function prevPage() {
    if (page > 0) {
      page -= 1;
      scrollToTop();
    }
  }
  function goToPage(p: number) {
    page = Math.max(0, Math.min(totalPages - 1, p));
    scrollToTop();
  }
  function scrollToTop() {
    const el = document.querySelector(".pets-manager");
    if (el) el.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  async function activate(slug: string) {
    if (busySlug) return;
    busySlug = slug;
    try {
      await petsSetActive(slug);
      activeSlug = slug;
      showToast("info", "Pet ativado");
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : (e?.message ?? "Erro"));
    } finally {
      busySlug = null;
    }
  }

  async function remove(slug: string) {
    if (busySlug) return;
    if (!confirm(`Remover '${slug}'? Os arquivos locais serão apagados.`)) return;
    busySlug = slug;
    try {
      await petsUninstall(slug);
      await load();
      showToast("info", "Pet removido");
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : (e?.message ?? "Erro"));
    } finally {
      busySlug = null;
    }
  }

  async function refresh(slug: string) {
    if (busySlug) return;
    if (!confirm(`Substituir os arquivos de '${slug}' pela versão mais recente do petdex?`)) return;
    busySlug = slug;
    try {
      await petsForceRefresh(slug);
      await load();
      showToast("info", "Pet atualizado");
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : (e?.message ?? "Erro"));
    } finally {
      busySlug = null;
    }
  }

  async function downloadAll() {
    if (bundleBusy) return;
    bundleBusy = true;
    try {
      const report = await petsInstallBundle();
      await load();
      showToast(
        "info",
        `${report.added.length} pets adicionados, ${report.skipped.length} pulados`,
      );
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : (e?.message ?? "Erro"));
    } finally {
      bundleBusy = false;
    }
  }

  async function openFolder() {
    try {
      await petsOpenFolder();
    } catch (e: any) {
      showToast("error", typeof e === "string" ? e : (e?.message ?? "Erro"));
    }
  }
</script>

<section class="pets-manager">
  <header class="head">
    <div class="head-text">
      <h2 class="title">Pets</h2>
      {#if index}
        <p class="subtitle">
          {index.total} pets instalados · última sincronização {lastSyncRelative}
        </p>
      {/if}
    </div>
    <div class="head-actions">
      <SyncButton onChange={load} />
      <button type="button" class="btn-secondary" disabled={bundleBusy} onclick={downloadAll}>
        {bundleBusy ? "Baixando…" : "Baixar todos"}
      </button>
      <button type="button" class="link" onclick={openFolder}>Abrir pasta de pets</button>
    </div>
  </header>

  <div class="filters">
    <input
      type="search"
      class="input"
      placeholder="Buscar por nome, tag ou vibe"
      bind:value={query}
    />
    <select class="input" bind:value={kindFilter}>
      <option value="">Todos os tipos</option>
      <option value="creature">Creatures</option>
      <option value="object">Objects</option>
      <option value="character">Characters</option>
    </select>
    <select class="input" bind:value={vibeFilter}>
      <option value="">Todas as vibes</option>
      {#each ALL_VIBES as vibe (vibe)}
        <option value={vibe}>{vibe}</option>
      {/each}
    </select>
  </div>

  {#if loading}
    <div class="state">Carregando…</div>
  {:else if !index || index.total === 0}
    <div class="empty">
      <h3 class="empty-title">Você ainda não tem pets instalados.</h3>
      <p class="empty-desc">
        Baixe a coleção completa de uma vez ou puxe sob demanda do petdex.
      </p>
      <div class="empty-actions">
        <button type="button" class="btn-primary" disabled={bundleBusy} onclick={downloadAll}>
          {bundleBusy ? "Baixando…" : "Baixar todos os pets"}
        </button>
      </div>
    </div>
  {:else if filtered.length === 0}
    <div class="state">Nenhum pet bate com os filtros.</div>
  {:else}
    <div class="result-info">
      <span>
        {filtered.length === 1
          ? "1 pet"
          : `${filtered.length} pets`}
        {#if totalPages > 1}
          · página {page + 1} de {totalPages}
        {/if}
      </span>
    </div>

    <div class="grid">
      {#each visible as pet (pet.slug)}
        <PetCard
          {pet}
          active={pet.slug === activeSlug}
          busy={busySlug === pet.slug}
          onActivate={activate}
          onRemove={remove}
          onRefresh={refresh}
        />
      {/each}
    </div>

    {#if totalPages > 1}
      <nav class="pagination" aria-label="Paginação de pets">
        <button
          type="button"
          class="page-btn"
          onclick={prevPage}
          disabled={page === 0}
          aria-label="Página anterior"
        >
          ← Anterior
        </button>

        <div class="page-nums">
          {#each Array(totalPages) as _, i (i)}
            {#if i === 0 || i === totalPages - 1 || Math.abs(i - page) <= 1}
              <button
                type="button"
                class="page-num"
                class:active={i === page}
                onclick={() => goToPage(i)}
              >
                {i + 1}
              </button>
            {:else if Math.abs(i - page) === 2}
              <span class="page-ellipsis" aria-hidden="true">…</span>
            {/if}
          {/each}
        </div>

        <button
          type="button"
          class="page-btn"
          onclick={nextPage}
          disabled={page + 1 >= totalPages}
          aria-label="Próxima página"
        >
          Próxima →
        </button>
      </nav>
    {/if}
  {/if}
</section>

<style>
  .pets-manager {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    width: 100%;
  }


  .head {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    justify-content: space-between;
    align-items: flex-start;
  }

  .head-text .title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 600;
  }

  .subtitle {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .head-actions {
    display: flex;
    gap: var(--space-2);
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .btn-secondary {
    padding: 8px 14px;
    background: var(--surface-hi);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-family: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--surface-mut);
  }

  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: progress;
  }

  .link {
    background: transparent;
    border: none;
    color: var(--accent);
    font-family: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
    padding: 8px 0;
    text-decoration: underline;
    text-decoration-color: transparent;
    transition: text-decoration-color var(--duration-fast) var(--ease-out);
  }

  .link:hover {
    text-decoration-color: currentColor;
  }

  .filters {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .input {
    flex: 1 1 200px;
    min-width: 160px;
    padding: 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-family: inherit;
    font-size: var(--text-sm);
  }

  .input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--space-3);
  }

  .state {
    text-align: center;
    padding: var(--space-6);
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-3);
    padding: var(--space-7) var(--space-5);
    background: var(--surface);
    border: 1px dashed var(--border);
    border-radius: var(--radius-md);
  }

  .empty-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
  }

  .empty-desc {
    margin: 0;
    color: var(--text-muted);
    max-width: 420px;
  }

  .empty-actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .btn-primary {
    padding: 10px 20px;
    background: var(--accent);
    color: var(--on-accent);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-size: var(--text-md);
    font-weight: 600;
    cursor: pointer;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-lo);
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: progress;
  }

  .result-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    margin-top: var(--space-3);
    padding-top: var(--space-3);
    border-top: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  }

  .page-btn {
    padding: 7px 14px;
    background: var(--surface-hi);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-family: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out);
  }
  .page-btn:hover:not(:disabled) {
    background: var(--surface-mut);
    border-color: var(--accent);
  }
  .page-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .page-nums {
    display: flex;
    gap: 2px;
    align-items: center;
  }
  .page-num {
    min-width: 32px;
    padding: 6px 10px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--text-sm);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out);
  }
  .page-num:hover {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    color: var(--text);
  }
  .page-num.active {
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
  .page-ellipsis {
    color: var(--text-muted);
    font-size: var(--text-sm);
    padding: 0 4px;
  }
</style>
