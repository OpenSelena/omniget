<script lang="ts">
  import { onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import PageHero from "$lib/study-components/PageHero.svelte";
  import ConfirmDialog from "$lib/study-components/ConfirmDialog.svelte";

  type DeckSummary = {
    id: number;
    name: string;
    filtered: boolean;
    new_count: number;
    learn_count: number;
    review_count: number;
  };

  type FilteredSearchTerm = {
    search: string;
    limit: number;
    order: string;
  };

  let decks = $state<DeckSummary[]>([]);
  let loading = $state(true);
  let error = $state("");
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  let createOpen = $state(false);
  let newName = $state("");
  let newSearch = $state("is:due");
  let newLimit = $state(100);
  let newOrder = $state("oldest_seen_first");
  let newReschedule = $state(true);
  let createBusy = $state(false);

  let rebuilding = $state<number | null>(null);
  let emptying = $state<number | null>(null);

  let confirmDeleteOpen = $state(false);
  let deleteTarget = $state<DeckSummary | null>(null);
  let deleteBusy = $state(false);

  const ORDER_OPTIONS = [
    { value: "oldest_seen_first", label: "Mais antigo primeiro" },
    { value: "random", label: "Aleatório" },
    { value: "interval_descending", label: "Maior intervalo" },
    { value: "interval_ascending", label: "Menor intervalo" },
    { value: "lapses_descending", label: "Mais lapsos" },
    { value: "added_descending", label: "Adicionados recentemente" },
    { value: "added_ascending", label: "Adicionados há mais tempo" },
    { value: "due_first", label: "Vencidos primeiro" },
  ];

  const PRESETS = [
    { label: "Vencidos hoje", search: "is:due" },
    { label: "Aprendendo", search: "is:learn" },
    { label: "Suspensos", search: "is:suspended" },
    { label: "Marcados (flag)", search: "flag:1 OR flag:2 OR flag:3 OR flag:4" },
    { label: "Tagged 'difícil'", search: 'tag:difícil' },
    { label: "Lapsos > 5", search: "prop:lapses>5" },
  ];

  function showToast(kind: "ok" | "err", msg: string) {
    toast = { kind, msg };
    setTimeout(() => (toast = null), 2400);
  }

  async function load() {
    loading = true;
    error = "";
    try {
      await pluginInvoke("study", "study:anki:storage:open");
      const all = await pluginInvoke<DeckSummary[]>(
        "study",
        "study:anki:decks:list",
      );
      decks = all.filter((d) => d.filtered);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function createFiltered() {
    const name = newName.trim();
    if (!name || !newSearch.trim()) return;
    createBusy = true;
    try {
      const term: FilteredSearchTerm = {
        search: newSearch.trim(),
        limit: Math.max(1, Math.min(99999, Math.floor(newLimit))),
        order: newOrder,
      };
      await pluginInvoke<{ id: number; cards: number }>(
        "study",
        "study:anki:decks:create_filtered",
        {
          name,
          searchTerms: [term],
          reschedule: newReschedule,
        },
      );
      showToast("ok", `Deck "${name}" criado`);
      createOpen = false;
      newName = "";
      newSearch = "is:due";
      newLimit = 100;
      await load();
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      createBusy = false;
    }
  }

  async function rebuild(deck: DeckSummary) {
    rebuilding = deck.id;
    try {
      const r = await pluginInvoke<{ cards: number }>(
        "study",
        "study:anki:decks:rebuild_filtered",
        { id: deck.id },
      );
      showToast(
        "ok",
        r.cards === 1
          ? "1 card no deck filtrado"
          : `${r.cards} cards no deck filtrado`,
      );
      await load();
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      rebuilding = null;
    }
  }

  async function emptyDeck(deck: DeckSummary) {
    emptying = deck.id;
    try {
      const r = await pluginInvoke<{ returned: number }>(
        "study",
        "study:anki:decks:empty_filtered",
        { id: deck.id },
      );
      showToast(
        "ok",
        r.returned === 0
          ? "Deck já estava vazio"
          : r.returned === 1
            ? "1 card devolvido pro deck original"
            : `${r.returned} cards devolvidos`,
      );
      await load();
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      emptying = null;
    }
  }

  function askDelete(deck: DeckSummary) {
    deleteTarget = deck;
    confirmDeleteOpen = true;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    deleteBusy = true;
    try {
      await pluginInvoke("study", "study:anki:decks:delete_filtered", {
        id: deleteTarget.id,
      });
      showToast("ok", "Deck filtrado removido");
      confirmDeleteOpen = false;
      deleteTarget = null;
      await load();
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      deleteBusy = false;
    }
  }

  onMount(load);
</script>

<section class="study-page">
  <PageHero
    title="Decks filtrados"
    subtitle="Cards selecionados por uma query — bom para revisão dirigida"
  />

  {#if toast}
    <div class="toast" class:err={toast.kind === "err"} role="status">
      {toast.msg}
    </div>
  {/if}

  <div class="toolbar">
    <a class="back-link" href="/study/anki/decks">← Voltar pra Decks</a>
    <button
      type="button"
      class="btn primary"
      onclick={() => (createOpen = true)}
    >
      + Novo deck filtrado
    </button>
  </div>

  {#if loading}
    <div class="state">Carregando…</div>
  {:else if error}
    <div class="state err">{error}</div>
    <button class="btn ghost" onclick={load}>Tentar de novo</button>
  {:else if decks.length === 0}
    <div class="empty">
      <p>Nenhum deck filtrado ainda.</p>
      <p class="hint">
        Decks filtrados puxam cards de outros decks via query (ex: <code>is:due</code>).
        Quando você termina, os cards voltam pro deck original.
      </p>
      <button
        type="button"
        class="btn primary"
        onclick={() => (createOpen = true)}
      >
        Criar primeiro
      </button>
    </div>
  {:else}
    <ul class="filtered-list">
      {#each decks as deck (deck.id)}
        {@const total = deck.new_count + deck.learn_count + deck.review_count}
        <li class="filtered-row">
          <div class="filtered-info">
            <h3>{deck.name}</h3>
            <p class="meta">
              <span class="pill new">{deck.new_count} novos</span>
              <span class="pill learn">{deck.learn_count} aprendendo</span>
              <span class="pill review">{deck.review_count} revisão</span>
            </p>
          </div>
          <div class="filtered-actions">
            <button
              type="button"
              class="btn ghost sm"
              onclick={() => rebuild(deck)}
              disabled={rebuilding === deck.id || emptying === deck.id}
              title="Refaz a query e popula o deck"
            >
              {rebuilding === deck.id ? "Reconstruindo…" : "Reconstruir"}
            </button>
            <button
              type="button"
              class="btn ghost sm"
              onclick={() => emptyDeck(deck)}
              disabled={rebuilding === deck.id || emptying === deck.id || total === 0}
              title="Devolve os cards ao deck original sem apagar o filtrado"
            >
              {emptying === deck.id ? "Esvaziando…" : "Esvaziar"}
            </button>
            <button
              type="button"
              class="btn ghost sm danger"
              onclick={() => askDelete(deck)}
              disabled={rebuilding === deck.id || emptying === deck.id}
            >
              Apagar
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if createOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) createOpen = false; }}
  >
    <div class="modal" role="dialog" aria-modal="true">
      <h3>Criar deck filtrado</h3>
      <p class="modal-hint">
        O deck puxa cards que casam com a query. Use sintaxe Anki:
        <code>is:due</code>, <code>tag:foo</code>, <code>deck:Bar</code>.
      </p>

      <label class="field">
        <span>Nome</span>
        <input
          type="text"
          bind:value={newName}
          placeholder="Ex: Revisão hoje"
        />
      </label>

      <label class="field">
        <span>Query</span>
        <input
          type="text"
          bind:value={newSearch}
          placeholder="is:due"
        />
      </label>

      <details class="presets">
        <summary>Presets</summary>
        <div class="preset-grid">
          {#each PRESETS as p (p.label)}
            <button
              type="button"
              class="preset-btn"
              onclick={() => (newSearch = p.search)}
            >
              <span class="preset-label">{p.label}</span>
              <span class="preset-q">{p.search}</span>
            </button>
          {/each}
        </div>
      </details>

      <div class="row">
        <label class="field">
          <span>Limite</span>
          <input
            type="number"
            min="1"
            max="9999"
            bind:value={newLimit}
          />
        </label>

        <label class="field">
          <span>Ordem</span>
          <select bind:value={newOrder}>
            {#each ORDER_OPTIONS as opt (opt.value)}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </label>
      </div>

      <label class="check">
        <input type="checkbox" bind:checked={newReschedule} />
        <span>Re-agendar cards (Anki recomenda manter ligado)</span>
      </label>

      <div class="modal-foot">
        <button
          type="button"
          class="btn ghost"
          onclick={() => (createOpen = false)}
          disabled={createBusy}
        >
          Cancelar
        </button>
        <button
          type="button"
          class="btn primary"
          onclick={createFiltered}
          disabled={createBusy || !newName.trim() || !newSearch.trim()}
        >
          {createBusy ? "Criando…" : "Criar"}
        </button>
      </div>
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:open={confirmDeleteOpen}
  title="Apagar deck filtrado"
  message={deleteTarget
    ? `O deck "${deleteTarget.name}" será removido. Os cards voltam pro deck original — nada é perdido.`
    : ""}
  confirmLabel="Apagar"
  variant="danger"
  onConfirm={confirmDelete}
/>

<style>
  .study-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 1.25);
    width: 100%;
    max-width: 820px;
    margin-inline: auto;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .back-link {
    color: var(--accent);
    font-size: 13px;
    text-decoration: none;
  }
  .back-link:hover {
    text-decoration: underline;
  }

  .state {
    padding: 24px;
    text-align: center;
    color: var(--secondary);
    font-size: 13px;
  }
  .state.err {
    color: var(--error, var(--accent));
  }

  .empty {
    padding: 48px 24px;
    text-align: center;
    color: var(--secondary);
    font-size: 13px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }
  .empty .hint {
    margin: 0;
    color: var(--tertiary);
    font-size: 12px;
    max-width: 420px;
    line-height: 1.55;
  }

  .filtered-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .filtered-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    background: var(--surface);
  }
  .filtered-info {
    flex: 1;
    min-width: 0;
  }
  .filtered-info h3 {
    margin: 0 0 6px;
    font-size: 14px;
    font-weight: 600;
  }
  .meta {
    display: flex;
    gap: 6px;
    margin: 0;
  }
  .pill {
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .pill.new {
    background: color-mix(in oklab, var(--blue, var(--accent)) 14%, transparent);
    color: var(--blue, var(--accent));
  }
  .pill.learn {
    background: color-mix(in oklab, var(--orange, var(--accent)) 14%, transparent);
    color: var(--orange, var(--accent));
  }
  .pill.review {
    background: color-mix(in oklab, var(--green, var(--accent)) 14%, transparent);
    color: var(--green, var(--accent));
  }

  .filtered-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .btn {
    padding: 7px 14px;
    border-radius: var(--border-radius);
    border: 1px solid transparent;
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .btn.primary {
    background: var(--accent);
    color: var(--on-accent, var(--on-cta, white));
  }
  .btn.primary:hover:not(:disabled) {
    background: var(--accent-hover, var(--accent));
  }
  .btn.ghost {
    background: transparent;
    border-color: var(--input-border);
    color: var(--text);
  }
  .btn.ghost:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .btn.ghost.danger {
    color: var(--error, var(--accent));
    border-color: color-mix(in oklab, var(--error, var(--accent)) 35%, var(--input-border));
  }
  .btn.ghost.danger:hover:not(:disabled) {
    background: color-mix(in oklab, var(--error, var(--accent)) 10%, transparent);
  }
  .btn.sm {
    padding: 5px 10px;
    font-size: 12px;
  }

  .toast {
    position: fixed;
    bottom: 20px;
    right: 20px;
    padding: 10px 16px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--accent) 16%, var(--surface));
    color: var(--text);
    font-size: 13px;
    box-shadow: 0 8px 24px color-mix(in oklab, black 20%, transparent);
    z-index: 100;
  }
  .toast.err {
    background: color-mix(in oklab, var(--error, var(--accent)) 18%, var(--surface));
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--dialog-backdrop, rgba(0, 0, 0, 0.55));
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
    padding: 16px;
  }
  .modal {
    background: var(--popup-bg, var(--surface));
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    padding: 20px;
    max-width: 540px;
    width: 100%;
    box-shadow: 0 20px 50px color-mix(in oklab, black 30%, transparent);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .modal h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .modal-hint {
    margin: 0;
    color: var(--tertiary);
    font-size: 12px;
    line-height: 1.55;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: var(--tertiary);
  }
  .field input,
  .field select {
    padding: 8px 10px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
  .field input:focus,
  .field select:focus {
    outline: none;
    border-color: var(--accent);
  }
  .row {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 8px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--secondary);
    font-size: 12px;
    cursor: pointer;
  }
  .check input {
    accent-color: var(--accent);
  }

  .presets summary {
    cursor: pointer;
    color: var(--accent);
    font-size: 12px;
  }
  .preset-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    margin-top: 8px;
  }
  .preset-btn {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 10px;
    background: var(--bg);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    text-align: left;
    cursor: pointer;
  }
  .preset-btn:hover {
    background: color-mix(in oklab, var(--accent) 6%, transparent);
    border-color: var(--accent);
  }
  .preset-label {
    font-size: 12px;
    color: var(--text);
  }
  .preset-q {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 10px;
    color: var(--tertiary);
  }

  .modal-foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    border-top: 1px solid color-mix(in oklab, var(--input-border) 40%, transparent);
    padding-top: 12px;
  }

  code {
    padding: 1px 6px;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    border-radius: 4px;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
  }
</style>
