<script lang="ts">
  import { onMount } from "svelte";
  import { page as routePage } from "$app/stores";
  import Editor from "$lib/study-components/notes/Editor.svelte";
  import CalendarStrip from "$lib/study-components/notes/CalendarStrip.svelte";
  import ActivityHeatmap from "$lib/study-components/notes/ActivityHeatmap.svelte";
  import {
    notesJournalToday,
    notesJournalForDay,
    notesJournalPrev,
    notesJournalNext,
    notesJournalListRecent,
    notesJournalListInRange,
    notesPagesGet,
    notesBlocksPageTree,
    notesTemplatesList,
    notesTemplatesApplyBuiltin,
    notesTemplatesApply,
    type NotePage as Page,
    type BlockNode,
    type JournalSummary,
    type TemplateSummary,
    type BuiltinKind,
  } from "$lib/notes-bridge";

  let recent = $state<JournalSummary[]>([]);
  let heatmap = $state<JournalSummary[]>([]);
  let currentDay = $state<number | null>(null);
  let currentPage = $state<Page | null>(null);
  let blockTree = $state<BlockNode[]>([]);

  let loading = $state(true);
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  let showJumpDialog = $state(false);
  let jumpDate = $state("");

  let applyTemplateOpen = $state(false);
  let userTemplates = $state<TemplateSummary[]>([]);
  const builtinTemplates: { kind: BuiltinKind; label: string }[] = [
    { kind: "daily-journal", label: "Daily journal" },
    { kind: "lesson-notes", label: "Lesson notes" },
    { kind: "book-highlights", label: "Book highlights" },
  ];

  function showToast(kind: "ok" | "err", msg: string) {
    toast = { kind, msg };
    setTimeout(() => (toast = null), 2400);
  }

  function decodeDay(d: number): { y: number; m: number; d: number } {
    return {
      y: Math.floor(d / 10000),
      m: Math.floor((d % 10000) / 100),
      d: d % 100,
    };
  }

  function encodeDayDate(date: Date): number {
    return (
      date.getFullYear() * 10000 +
      (date.getMonth() + 1) * 100 +
      date.getDate()
    );
  }

  function fmtDay(d: number): string {
    const { y, m, d: day } = decodeDay(d);
    const date = new Date(y, m - 1, day);
    return date.toLocaleDateString("pt-BR", {
      weekday: "long",
      day: "2-digit",
      month: "long",
      year: "numeric",
    });
  }

  function isToday(d: number): boolean {
    return d === encodeDayDate(new Date());
  }

  async function loadRecent() {
    try {
      recent = await notesJournalListRecent(30);
    } catch (e) {
      console.error("loadRecent failed", e);
    }
  }

  async function loadHeatmap() {
    try {
      const today = new Date();
      const start = new Date(today);
      start.setDate(today.getDate() - 90);
      heatmap = await notesJournalListInRange({
        fromDay: encodeDayDate(start),
        toDay: encodeDayDate(today),
      });
    } catch (e) {
      console.error("loadHeatmap failed", e);
    }
  }

  async function loadTemplates() {
    try {
      userTemplates = await notesTemplatesList();
    } catch (e) {
      console.error("loadTemplates failed", e);
    }
  }

  async function openDay(day: number) {
    loading = true;
    try {
      const r = await notesJournalForDay({ day });
      currentDay = r.journal_day;
      const p = await notesPagesGet(r.page_id);
      currentPage = p;
      blockTree = await notesBlocksPageTree(p.id);
      await loadRecent();
      await loadHeatmap();
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      loading = false;
    }
  }

  async function openToday() {
    try {
      const r = await notesJournalToday();
      await openDay(r.journal_day);
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function nav(direction: "prev" | "next") {
    if (currentDay == null) return;
    try {
      const r =
        direction === "prev"
          ? await notesJournalPrev(currentDay)
          : await notesJournalNext(currentDay);
      if (r.journal_day != null) {
        await openDay(r.journal_day);
      } else {
        showToast(
          "ok",
          direction === "prev" ? "Sem journal anterior" : "Sem journal posterior",
        );
      }
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function jumpToDate() {
    if (!jumpDate) {
      showJumpDialog = false;
      return;
    }
    const parts = jumpDate.split("-");
    if (parts.length !== 3) {
      showJumpDialog = false;
      return;
    }
    const day =
      parseInt(parts[0]) * 10000 +
      parseInt(parts[1]) * 100 +
      parseInt(parts[2]);
    showJumpDialog = false;
    jumpDate = "";
    await openDay(day);
  }

  async function reloadTree() {
    if (!currentPage) return;
    blockTree = await notesBlocksPageTree(currentPage.id);
  }

  async function applyBuiltinTemplate(kind: BuiltinKind) {
    if (!currentPage) return;
    try {
      const r = await notesTemplatesApplyBuiltin({
        kind,
        targetPageId: currentPage.id,
        vars: {},
      });
      applyTemplateOpen = false;
      await reloadTree();
      showToast("ok", `${r.blocks_created} blocos do template criados`);
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function applyUserTemplate(templatePageId: number) {
    if (!currentPage) return;
    try {
      const r = await notesTemplatesApply({
        templatePageId,
        targetPageId: currentPage.id,
        vars: {},
      });
      applyTemplateOpen = false;
      await reloadTree();
      showToast("ok", `${r.blocks_created} blocos do template criados`);
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  onMount(async () => {
    await Promise.all([loadTemplates(), loadHeatmap(), loadRecent()]);
    const target = $routePage.url.searchParams.get("day");
    if (target) {
      const day = Number(target);
      if (Number.isFinite(day)) {
        await openDay(day);
        return;
      }
    }
    await openToday();
  });
</script>

<div class="journal-shell" data-surface="notes">
  <aside class="left">
    <header class="left-head">
      <a href="/study/notes" class="back">← Notas</a>
      <h2 class="page-title">Journal</h2>
    </header>

    <button
      type="button"
      class="btn primary today-btn"
      onclick={openToday}
      class:active={currentDay !== null && isToday(currentDay)}
    >
      Hoje
    </button>

    <div class="nav-row">
      <button type="button" class="btn ghost sm" onclick={() => nav("prev")}>← Ant.</button>
      <button type="button" class="btn ghost sm" onclick={() => nav("next")}>Próx. →</button>
    </div>

    <button
      type="button"
      class="btn ghost sm jump"
      onclick={() => (showJumpDialog = true)}
    >
      Ir para data…
    </button>

    <ActivityHeatmap journals={heatmap} onPick={openDay} />

    <section class="recent-list">
      <h3>Histórico</h3>
      <ul>
        {#each recent.slice(0, 30) as r (r.page_id)}
          <li>
            <button
              type="button"
              class="recent-row"
              class:active={currentDay === r.journal_day}
              onclick={() => openDay(r.journal_day)}
            >
              <span class="recent-day">
                {(() => {
                  const { m, d } = decodeDay(r.journal_day);
                  return `${String(d).padStart(2, "0")}/${String(m).padStart(2, "0")}`;
                })()}
              </span>
              <span class="recent-count">{r.block_count}</span>
            </button>
          </li>
        {:else}
          <li class="empty">Sem journals ainda.</li>
        {/each}
      </ul>
    </section>
  </aside>

  <main class="editor-col">
    {#if toast}
      <div class="toast" class:err={toast.kind === "err"} role="status">
        {toast.msg}
      </div>
    {/if}

    <CalendarStrip
      journals={recent}
      {currentDay}
      onPick={openDay}
    />

    {#if loading}
      <div class="state">Carregando journal…</div>
    {:else if !currentPage || currentDay === null}
      <div class="state">Nenhum journal aberto.</div>
    {:else}
      <header class="ed-head">
        <div class="title-block">
          <h1>{fmtDay(currentDay)}</h1>
          <span class="path">{currentPage.name}</span>
        </div>
        <div class="head-actions">
          <button
            type="button"
            class="btn ghost sm"
            onclick={() => (applyTemplateOpen = true)}
          >
            + Aplicar template
          </button>
          {#if currentDay !== null && isToday(currentDay)}
            <span class="badge today">hoje</span>
          {/if}
        </div>
      </header>

      {@const firstBlock = blockTree.length > 0 ? blockTree[0] : null}
      <Editor
        pageId={currentPage.id}
        aggregateBlockId={firstBlock?.id ?? null}
        initialMarkdown={firstBlock?.content ?? ""}
        onSaved={() => { void reloadTree(); }}
        onError={(msg) => showToast("err", msg)}
      />
    {/if}
  </main>
</div>

{#if showJumpDialog}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) showJumpDialog = false;
    }}
  >
    <div class="modal">
      <h3>Ir para data</h3>
      <input
        type="date"
        bind:value={jumpDate}
        onkeydown={(e) => {
          if (e.key === "Enter") jumpToDate();
          else if (e.key === "Escape") showJumpDialog = false;
        }}
      />
      <footer>
        <button type="button" class="btn ghost" onclick={() => (showJumpDialog = false)}>
          Cancelar
        </button>
        <button type="button" class="btn primary" onclick={jumpToDate}>Abrir</button>
      </footer>
    </div>
  </div>
{/if}

{#if applyTemplateOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) applyTemplateOpen = false;
    }}
  >
    <div class="modal wide">
      <h3>Aplicar template</h3>
      <p class="hint">
        Os blocos do template serão adicionados ao final deste journal.
      </p>

      <section>
        <h4>Built-in</h4>
        <div class="tpl-grid">
          {#each builtinTemplates as t (t.kind)}
            <button
              type="button"
              class="tpl-card"
              onclick={() => applyBuiltinTemplate(t.kind)}
            >
              <strong>{t.label}</strong>
              <span class="tpl-hint">{t.kind}</span>
            </button>
          {/each}
        </div>
      </section>

      {#if userTemplates.length > 0}
        <section>
          <h4>Suas páginas marcadas como template</h4>
          <div class="tpl-grid">
            {#each userTemplates as t (t.page_id)}
              <button
                type="button"
                class="tpl-card"
                onclick={() => applyUserTemplate(t.page_id)}
              >
                <strong>{t.title ?? t.name}</strong>
                <span class="tpl-hint">{t.name}</span>
              </button>
            {/each}
          </div>
        </section>
      {/if}

      <footer>
        <button type="button" class="btn ghost" onclick={() => (applyTemplateOpen = false)}>
          Cancelar
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .journal-shell {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 12px;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }
  @media (max-width: 760px) {
    .journal-shell {
      grid-template-columns: 1fr;
    }
    .left {
      display: none;
    }
  }
  .left {
    border-right: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
  }
  .left-head {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .left-head h2 {
    margin: 0;
    font-size: 16px;
  }
  .back {
    color: var(--tertiary);
    font-size: 12px;
    text-decoration: none;
  }
  .back:hover {
    color: var(--accent);
  }
  .today-btn.active {
    background: color-mix(in oklab, var(--success, var(--accent)) 80%, white);
  }
  .nav-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }
  .jump {
    width: 100%;
  }
  .recent-list h3 {
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--tertiary);
  }
  .recent-list ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .recent-row {
    width: 100%;
    display: flex;
    justify-content: space-between;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--secondary);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .recent-row:hover {
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .recent-row.active {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    color: var(--text);
  }
  .recent-day {
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .recent-count {
    color: var(--tertiary);
    font-weight: 500;
  }
  .empty {
    color: var(--tertiary);
    font-size: 12px;
    padding: 4px 0;
  }
  .editor-col {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
    overflow-y: auto;
    max-width: 820px;
    margin-inline: auto;
    width: 100%;
  }
  .state {
    padding: calc(var(--padding) * 2);
    text-align: center;
    color: var(--secondary);
  }
  .ed-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }
  .ed-head h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 600;
    text-transform: capitalize;
  }
  .path {
    color: var(--tertiary);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .badge.today {
    padding: 3px 10px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--success, var(--accent)) 14%, transparent);
    color: var(--success, var(--accent));
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
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
    background: color-mix(
      in oklab,
      var(--error, var(--accent)) 18%,
      var(--surface)
    );
  }
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: color-mix(in oklab, black 50%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 90;
    padding: var(--padding);
  }
  .modal {
    width: 100%;
    max-width: 460px;
    background: var(--surface);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .modal.wide {
    max-width: 640px;
  }
  .modal h3 {
    margin: 0;
    font-size: 16px;
  }
  .modal h4 {
    margin: 8px 0 4px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--tertiary);
  }
  .modal input {
    padding: 10px 12px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 14px;
  }
  .modal input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .modal .hint {
    color: var(--tertiary);
    font-size: 12px;
    margin: 0;
  }
  .modal footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .tpl-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .tpl-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    background: transparent;
    cursor: pointer;
    text-align: left;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .tpl-card:hover {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 5%, transparent);
  }
  .tpl-card strong {
    font-size: 13px;
    color: var(--text);
  }
  .tpl-hint {
    font-size: 11px;
    color: var(--tertiary);
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .btn {
    padding: 6px 12px;
    border-radius: var(--border-radius);
    border: 1px solid transparent;
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 150ms ease;
  }
  .btn.primary {
    background: var(--accent);
    color: var(--on-accent, var(--on-cta, white));
  }
  .btn.ghost {
    background: transparent;
    border-color: var(--input-border);
    color: var(--text);
  }
  .btn.ghost:hover {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .btn.sm {
    padding: 4px 10px;
    font-size: 11px;
  }
</style>
