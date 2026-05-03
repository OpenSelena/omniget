<script lang="ts">
  import { onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import PageHero from "$lib/study-components/PageHero.svelte";

  type NotetypeSummary = {
    id: number;
    name: string;
    kind: string;
    field_count: number;
    template_count: number;
  };

  type DeckSummary = {
    id: number;
    name: string;
    filtered: boolean;
  };

  type ApkgSummary = {
    notetypes_added: number;
    decks_added: number;
    notes_added: number;
    cards_added: number;
    revlog_added: number;
    skipped_existing_notes: number;
    media_added: number;
  };

  type JsonSummary = {
    notetypes_added: number;
    decks_added: number;
    deck_configs_added: number;
    notes_added: number;
    cards_added: number;
    revlog_added: number;
    skipped_existing_notes: number;
  };

  type CsvSummary = {
    imported: number;
    skipped: number;
    errors: string[];
  };

  type Result =
    | { kind: "apkg"; data: ApkgSummary }
    | { kind: "json"; data: JsonSummary }
    | { kind: "csv"; data: CsvSummary };

  let notetypes = $state<NotetypeSummary[]>([]);
  let decks = $state<DeckSummary[]>([]);
  let csvNotetypeId = $state<number | null>(null);
  let csvDeckId = $state<number | null>(null);
  let csvHasHeader = $state(true);
  let csvDelimiter = $state<"" | "\t" | "," | ";">("");

  let busy = $state(false);
  let error = $state("");
  let result = $state<Result | null>(null);
  let lastSourcePath = $state("");

  async function refreshLookups() {
    try {
      const [nts, ds] = await Promise.all([
        pluginInvoke<NotetypeSummary[]>("study", "study:anki:notetypes:list"),
        pluginInvoke<DeckSummary[]>("study", "study:anki:decks:list"),
      ]);
      notetypes = nts;
      decks = ds.filter((d) => !d.filtered);
      if (csvNotetypeId === null && notetypes.length > 0) {
        csvNotetypeId = notetypes[0].id;
      }
      if (csvDeckId === null) {
        const def = decks.find((d) => d.id === 1) ?? decks[0];
        if (def) csvDeckId = def.id;
      }
    } catch (e) {
      console.error("lookup failed", e);
    }
  }

  onMount(async () => {
    try {
      await pluginInvoke("study", "study:anki:storage:open");
    } catch (e) {
      console.error("open storage failed", e);
    }
    await refreshLookups();
  });

  function detectKind(path: string): "apkg" | "json" | "csv" | null {
    const lower = path.toLowerCase();
    if (lower.endsWith(".apkg") || lower.endsWith(".colpkg")) return "apkg";
    if (lower.endsWith(".json")) return "json";
    if (lower.endsWith(".csv") || lower.endsWith(".tsv") || lower.endsWith(".txt"))
      return "csv";
    return null;
  }

  async function pickAndImport() {
    error = "";
    result = null;
    try {
      const dialog = await import("@tauri-apps/plugin-dialog");
      const picked = await dialog.open({
        multiple: false,
        filters: [
          {
            name: "Anki collections / notes",
            extensions: ["apkg", "colpkg", "json", "csv", "tsv", "txt"],
          },
        ],
      });
      if (typeof picked !== "string" || !picked) return;
      lastSourcePath = picked;
      const kind = detectKind(picked);
      if (!kind) {
        error = `Formato não reconhecido: ${picked}`;
        return;
      }
      busy = true;
      if (kind === "apkg") {
        const data = await pluginInvoke<ApkgSummary>(
          "study",
          "study:anki:import:apkg",
          { sourcePath: picked },
        );
        result = { kind: "apkg", data };
      } else if (kind === "json") {
        const data = await pluginInvoke<JsonSummary>(
          "study",
          "study:anki:import:json",
          { sourcePath: picked },
        );
        result = { kind: "json", data };
      } else {
        if (csvNotetypeId === null || csvDeckId === null) {
          error = "Selecione um modelo e um deck antes de importar CSV.";
          return;
        }
        const delim = csvDelimiter === "" ? null : csvDelimiter;
        const data = await pluginInvoke<CsvSummary>(
          "study",
          "study:anki:import:csv_notes",
          {
            notetypeId: csvNotetypeId,
            deckId: csvDeckId,
            sourcePath: picked,
            delimiter: delim,
            hasHeader: csvHasHeader,
          },
        );
        result = { kind: "csv", data };
      }
      await refreshLookups();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function summaryRows(r: Result): { label: string; value: number | string }[] {
    if (r.kind === "apkg") {
      return [
        { label: "Notes adicionadas", value: r.data.notes_added },
        { label: "Cards adicionados", value: r.data.cards_added },
        { label: "Decks novos", value: r.data.decks_added },
        { label: "Modelos novos", value: r.data.notetypes_added },
        { label: "Mídia copiada", value: r.data.media_added },
        { label: "Revlog importado", value: r.data.revlog_added },
        { label: "Notes ignoradas (já existem)", value: r.data.skipped_existing_notes },
      ];
    }
    if (r.kind === "json") {
      return [
        { label: "Notes adicionadas", value: r.data.notes_added },
        { label: "Cards adicionados", value: r.data.cards_added },
        { label: "Decks novos", value: r.data.decks_added },
        { label: "Configs de deck novas", value: r.data.deck_configs_added },
        { label: "Modelos novos", value: r.data.notetypes_added },
        { label: "Revlog importado", value: r.data.revlog_added },
        { label: "Notes ignoradas (já existem)", value: r.data.skipped_existing_notes },
      ];
    }
    return [
      { label: "Linhas importadas", value: r.data.imported },
      { label: "Linhas ignoradas", value: r.data.skipped },
    ];
  }
</script>

<section class="study-page">
  <PageHero title="Importar" subtitle="Carregue uma coleção do Anki ou um arquivo de notas" />

  <div class="format-grid">
    <article class="format-card">
      <h3>.apkg / .colpkg</h3>
      <p>Coleção exportada do Anki desktop. Inclui notes, cards, decks, modelos, revlog e mídia.</p>
      <small class="muted">Conflitos: notes existentes (mesmo id ou guid) são ignoradas.</small>
    </article>
    <article class="format-card">
      <h3>.json</h3>
      <p>Snapshot interno do plugin (gerado por Exportar &gt; JSON). Round-trip completo.</p>
    </article>
    <article class="format-card csv-card">
      <h3>.csv / .tsv</h3>
      <p>Planilha com colunas dos fields do modelo + opcionais Tags e Deck.</p>
      <div class="csv-options">
        <label>
          <span>Modelo</span>
          <select bind:value={csvNotetypeId} disabled={busy}>
            {#each notetypes as nt (nt.id)}
              <option value={nt.id}>{nt.name}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Deck</span>
          <select bind:value={csvDeckId} disabled={busy}>
            {#each decks as d (d.id)}
              <option value={d.id}>{d.name}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Delimitador</span>
          <select bind:value={csvDelimiter} disabled={busy}>
            <option value="">Auto</option>
            <option value={"\t"}>Tab</option>
            <option value=",">Vírgula</option>
            <option value=";">Ponto-e-vírgula</option>
          </select>
        </label>
        <label class="checkbox">
          <input type="checkbox" bind:checked={csvHasHeader} disabled={busy} />
          <span>Primeira linha é cabeçalho</span>
        </label>
      </div>
    </article>
  </div>

  <div class="cta-row">
    <button type="button" class="btn-primary" onclick={pickAndImport} disabled={busy}>
      {busy ? "Importando…" : "Escolher arquivo e importar"}
    </button>
    {#if lastSourcePath && !busy}
      <span class="last-path">Último: {lastSourcePath}</span>
    {/if}
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if result}
    <section class="card result-card">
      <header class="card-head">
        <h2>Importação concluída</h2>
        <span class="kind-badge">{result.kind.toUpperCase()}</span>
      </header>
      <ul class="summary-list">
        {#each summaryRows(result) as row (row.label)}
          <li>
            <span class="row-label">{row.label}</span>
            <span class="row-value">{row.value}</span>
          </li>
        {/each}
      </ul>
      {#if result.kind === "csv" && result.data.errors.length > 0}
        <details class="errors">
          <summary>{result.data.errors.length} linhas com erro</summary>
          <ul>
            {#each result.data.errors.slice(0, 20) as err (err)}
              <li>{err}</li>
            {/each}
            {#if result.data.errors.length > 20}
              <li class="muted">… e mais {result.data.errors.length - 20}</li>
            {/if}
          </ul>
        </details>
      {/if}
      <footer class="card-foot">
        <a class="back-link" href="/study/anki">Ver no painel →</a>
      </footer>
    </section>
  {/if}
</section>

<style>
  .study-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 1.5);
    width: 100%;
    max-width: 900px;
    margin-inline: auto;
  }

  .format-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--padding);
  }
  .format-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: calc(var(--padding) * 1.5);
    background: var(--button-elevated);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
  }
  .format-card h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--secondary);
    font-family: var(--font-mono, monospace);
  }
  .format-card p {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text);
    line-height: 1.5;
  }
  .csv-card {
    grid-column: 1 / -1;
  }
  .csv-options {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
    margin-top: 10px;
  }
  .csv-options label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: var(--tertiary);
  }
  .csv-options label.checkbox {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: var(--secondary);
    font-size: 12px;
  }
  .csv-options select {
    background: var(--input-bg);
    border: 1px solid var(--input-border);
    color: var(--secondary);
    padding: 6px 8px;
    border-radius: var(--border-radius);
    font-size: 13px;
  }
  .csv-options select:focus-visible {
    outline: 2px solid var(--focus-ring, var(--accent));
    outline-offset: -1px;
  }

  .cta-row {
    display: flex;
    align-items: center;
    gap: var(--padding);
    flex-wrap: wrap;
  }
  .btn-primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 10px 24px;
    background: var(--accent);
    color: var(--on-accent, var(--on-cta, white));
    border: 0;
    border-radius: var(--border-radius);
    font-family: inherit;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: filter 150ms ease;
  }
  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .btn-primary:focus-visible {
    outline: 2px solid var(--focus-ring, var(--accent));
    outline-offset: 2px;
  }
  .btn-primary:disabled {
    background: color-mix(in oklab, var(--input-border) 80%, transparent);
    color: var(--tertiary);
    cursor: not-allowed;
  }
  .last-path {
    font-size: 11px;
    color: var(--tertiary);
    font-family: var(--font-mono, monospace);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
  }

  .error {
    color: var(--error);
    font-size: 13px;
    padding: 8px 12px;
    background: color-mix(in oklab, var(--error) 8%, transparent);
    border: 1px solid color-mix(in oklab, var(--error) 25%, var(--input-border));
    border-radius: var(--border-radius);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: var(--padding);
    padding: calc(var(--padding) * 2);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--button-elevated);
  }
  .card-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--padding);
  }
  .card-head h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--secondary);
  }
  .kind-badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    color: var(--accent);
    font-family: var(--font-mono, monospace);
  }
  .summary-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 8px;
  }
  .summary-list li {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 8px 12px;
    background: var(--bg);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
  }
  .row-label {
    color: var(--tertiary);
    font-size: 12px;
  }
  .row-value {
    color: var(--accent);
    font-family: var(--font-mono, monospace);
    font-variant-numeric: tabular-nums;
    font-weight: 500;
  }
  .errors summary {
    cursor: pointer;
    font-size: 13px;
    color: var(--secondary);
    padding: 6px 0;
  }
  .errors ul {
    margin: 6px 0 0;
    padding-left: 20px;
    font-size: 12px;
    color: var(--tertiary);
    line-height: 1.5;
  }
  .muted {
    color: var(--tertiary);
    font-size: 11px;
  }
  .card-foot {
    display: flex;
    justify-content: flex-end;
    padding-top: var(--padding);
    border-top: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
  }
  .back-link {
    color: var(--accent);
    text-decoration: none;
    font-size: 13px;
    padding: 4px 10px;
    border-radius: var(--border-radius);
    transition: background 150ms ease;
  }
  .back-link:hover {
    background: color-mix(in oklab, var(--accent) 10%, transparent);
  }
  .back-link:focus-visible {
    outline: 2px solid var(--focus-ring, var(--accent));
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .btn-primary,
    .back-link {
      transition: none;
    }
  }
</style>
