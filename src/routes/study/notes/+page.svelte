<script lang="ts">
  import { onMount } from "svelte";
  import { page as routePage } from "$app/stores";
  import { t } from "$lib/i18n";
  import { awardXp } from "$lib/study-gamification";
  import ConfirmDialog from "$lib/study-components/ConfirmDialog.svelte";
  import SidebarTree from "$lib/study-components/notes/SidebarTree.svelte";
  import RefsPanel from "$lib/study-components/notes/RefsPanel.svelte";
  import CreatePageDialog from "$lib/study-components/notes/CreatePageDialog.svelte";
  import RenamePageDialog from "$lib/study-components/notes/RenamePageDialog.svelte";
  import Editor from "$lib/study-components/notes/Editor.svelte";
  import Breadcrumb from "$lib/study-components/notes/Breadcrumb.svelte";
  import PageHero from "$lib/study-components/notes/PageHero.svelte";
  import CoverManager from "$lib/study-components/notes/CoverManager.svelte";
  import HistoryModal from "$lib/study-components/notes/HistoryModal.svelte";
  import {
    notesPagesList,
    notesPagesGet,
    notesPagesCreate,
    notesPagesDelete,
    notesPagesRenameCascade,
    notesPagesResolve,
    notesPagesEnsure,
    notesPagesSetAliases,
    notesPagesSetTags,
    notesPagesLinkedRefs,
    notesPagesUnlinkedRefs,
    notesTagsList,
    notesJournalToday,
    notesBlocksPageTree,
    notesUndoLastOp,
    notesUndoRedoLast,
    notesAssetsListForPage,
    type NotePage as Page,
    type BlockNode,
    type PageSummary,
    type LinkedRef,
    type TagSummary,
  } from "$lib/notes-bridge";

  let sidebarPages = $state<PageSummary[]>([]);
  let sidebarTags = $state<TagSummary[]>([]);
  let sidebarSearch = $state("");
  let recentLimit = $state(50);

  let currentPage = $state<Page | null>(null);
  let blockTree = $state<BlockNode[]>([]);

  let linkedRefs = $state<LinkedRef[]>([]);
  let unlinkedRefs = $state<LinkedRef[]>([]);
  let refsLoading = $state(false);

  let loading = $state(true);
  let error = $state("");
  let notesTableMissing = $state(false);
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  let renameOpen = $state(false);
  let renameValue = $state("");
  let confirmDeletePageOpen = $state(false);

  let aliasesEditor = $state("");
  let tagsEditor = $state("");
  let aliasesDirty = $state(false);
  let tagsDirty = $state(false);

  let createPageOpen = $state(false);

  let coverUrl = $state<string | null>(null);
  let coverManagerOpen = $state(false);
  let historyOpen = $state(false);
  let firstBlockId = $derived(blockTree.length > 0 ? blockTree[0].id : null);

  async function loadCover(pageId: number) {
    try {
      const assets = await notesAssetsListForPage(pageId);
      const cover = assets.find((a) => a.kind === "cover");
      coverUrl = cover?.path ?? null;
    } catch (e) {
      console.error("loadCover failed", e);
      coverUrl = null;
    }
  }

  function showToast(kind: "ok" | "err", msg: string) {
    toast = { kind, msg };
    setTimeout(() => (toast = null), 2400);
  }

  async function loadSidebar() {
    try {
      const [pages, tags] = await Promise.all([
        notesPagesList(),
        notesTagsList(30),
      ]);
      sidebarPages = pages;
      sidebarTags = tags;
      error = "";
      notesTableMissing = false;
    } catch (e) {
      const raw = e instanceof Error ? e.message : String(e);
      console.error("[notes] loadSidebar failed:", raw);
      if (/no such table/i.test(raw) || /database\s+disk/i.test(raw)) {
        notesTableMissing = true;
        error = "";
      } else {
        notesTableMissing = false;
        error = raw;
      }
    }
  }

  async function openPage(id: number) {
    loading = true;
    error = "";
    try {
      const p = await notesPagesGet(id);
      currentPage = p;
      aliasesEditor = p.aliases.join(", ");
      tagsEditor = p.tags.join(", ");
      aliasesDirty = false;
      tagsDirty = false;
      blockTree = await notesBlocksPageTree(id);
      await Promise.all([refreshRefs(id), loadCover(id)]);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function refreshRefs(pageId: number) {
    refsLoading = true;
    try {
      const [linked, unlinked] = await Promise.all([
        notesPagesLinkedRefs(pageId),
        notesPagesUnlinkedRefs(pageId),
      ]);
      linkedRefs = linked;
      unlinkedRefs = unlinked;
    } catch (e) {
      console.error("refreshRefs failed", e);
    } finally {
      refsLoading = false;
    }
  }

  async function reloadTree() {
    if (!currentPage) return;
    blockTree = await notesBlocksPageTree(currentPage.id);
  }

  async function createPage(name: string) {
    try {
      const r = await notesPagesCreate({ name });
      createPageOpen = false;
      await loadSidebar();
      await openPage(r.id);
      void awardXp("page_created", 15, { page_id: r.id, name });
      showToast("ok", "Página criada");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function deletePage() {
    if (!currentPage) return;
    try {
      await notesPagesDelete(currentPage.id);
      currentPage = null;
      blockTree = [];
      linkedRefs = [];
      unlinkedRefs = [];
      await loadSidebar();
      showToast("ok", "Página removida");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function renamePage(newName: string) {
    if (!currentPage) return;
    if (newName === currentPage.name) {
      renameOpen = false;
      return;
    }
    try {
      const r = await notesPagesRenameCascade({ id: currentPage.id, newName });
      renameOpen = false;
      await loadSidebar();
      await openPage(currentPage.id);
      showToast(
        "ok",
        r.blocks_updated === 0
          ? "Renomeada"
          : r.blocks_updated === 1
            ? "Renomeada e 1 bloco atualizado"
            : `Renomeada e ${r.blocks_updated} blocos atualizados`,
      );
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function saveAliases() {
    if (!currentPage || !aliasesDirty) return;
    const arr = aliasesEditor
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    try {
      await notesPagesSetAliases({ id: currentPage.id, aliases: arr });
      aliasesDirty = false;
      showToast("ok", "Aliases salvos");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function saveTags() {
    if (!currentPage || !tagsDirty) return;
    const arr = tagsEditor
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    try {
      await notesPagesSetTags({ id: currentPage.id, tags: arr });
      tagsDirty = false;
      currentPage = { ...currentPage, tags: arr };
      showToast("ok", "Tags salvas");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function gotoPageByName(name: string) {
    try {
      const p = await notesPagesResolve(name);
      if (p) {
        await openPage(p.id);
        await loadSidebar();
      } else {
        const r = await notesPagesEnsure({ name });
        await openPage(r.id);
        await loadSidebar();
      }
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function openJournalToday() {
    try {
      const r = await notesJournalToday();
      await openPage(r.page_id);
      await loadSidebar();
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function undoLastOp() {
    try {
      await notesUndoLastOp();
      await reloadTree();
      showToast("ok", "Desfeito");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (!msg.includes("no op to undo")) {
        showToast("err", msg);
      }
    }
  }

  async function redoLastOp() {
    try {
      await notesUndoRedoLast();
      await reloadTree();
      showToast("ok", "Refeito");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (!msg.includes("no op to redo")) {
        showToast("err", msg);
      }
    }
  }

  onMount(async () => {
    await loadSidebar();
    const target = $routePage.url.searchParams.get("page");
    if (target) {
      await gotoPageByName(target);
    } else if (sidebarPages.length > 0) {
      await openPage(sidebarPages[0].id);
    } else {
      loading = false;
    }
  });
</script>

<div class="notes-shell" data-surface="notes">
  <SidebarTree
    pages={sidebarPages}
    tags={sidebarTags}
    selectedPageId={currentPage?.id ?? null}
    searchValue={sidebarSearch}
    {recentLimit}
    onCreate={() => (createPageOpen = true)}
    onSelect={openPage}
    onJournalToday={openJournalToday}
    onSearchInput={(v) => (sidebarSearch = v)}
    onShowMore={() => (recentLimit += 50)}
  />

  <main class="editor">
    {#if toast}
      <div class="toast" class:err={toast.kind === "err"} role="status">
        {toast.msg}
      </div>
    {/if}

    {#if loading}
      <div class="state">Carregando…</div>
    {:else if notesTableMissing}
      <div class="state err">
        <p>{$t("study.library.error_loading_notes")}</p>
        <button class="btn" onclick={() => loadSidebar()}>{$t("common.retry")}</button>
      </div>
    {:else if error}
      <div class="state err">
        <p>{error}</p>
        <button class="btn" onclick={() => loadSidebar()}>{$t("common.retry")}</button>
      </div>
    {:else if !currentPage}
      <div class="state">
        {$t("study.library.notes_no_open_page")}
      </div>
    {:else}
      <PageHero
        coverUrl={coverUrl}
        title={currentPage.title ?? currentPage.name}
        subtitle={currentPage.name}
        onTitleClick={() => {
          renameValue = currentPage?.name ?? "";
          renameOpen = true;
        }}
      />
      <header class="editor-head">
        {#if !coverUrl}
          <button
            type="button"
            class="title"
            onclick={() => {
              renameValue = currentPage?.name ?? "";
              renameOpen = true;
            }}
            title="Click para renomear"
          >
            {currentPage.title ?? currentPage.name}
          </button>
        {/if}
        <Breadcrumb page={currentPage} onSegmentClick={(path) => gotoPageByName(path)} />
        <div class="head-actions">
          {#if currentPage.journal_day}
            <span class="badge">journal</span>
          {/if}
          <button
            class="btn ghost sm"
            onclick={() => (coverManagerOpen = true)}
            title="Definir capa da página"
          >
            {coverUrl ? "Capa ✓" : "+ Capa"}
          </button>
          <button
            class="btn ghost sm"
            onclick={() => (historyOpen = true)}
            disabled={firstBlockId === null}
            title="Ver histórico do bloco principal desta página"
          >
            🕐 Histórico
          </button>
          <button
            class="btn ghost sm"
            onclick={undoLastOp}
            title="Desfazer última operação estrutural (Cmd+Alt+Z)"
          >
            ↶
          </button>
          <button
            class="btn ghost sm"
            onclick={redoLastOp}
            title="Refazer (Cmd+Shift+Z)"
          >
            ↷
          </button>
          <a
            class="btn ghost sm"
            href="/study/notes/shortcuts"
            title="Atalhos do editor"
          >
            ?
          </a>
          <button
            class="btn ghost sm"
            onclick={() => window.print()}
            title="Imprimir / Salvar PDF (Ctrl+P)"
          >
            Imprimir/PDF
          </button>
          <button
            class="btn ghost sm danger"
            onclick={() => (confirmDeletePageOpen = true)}
          >
            Excluir página
          </button>
        </div>
      </header>

      <div class="page-meta-bar">
        <label class="meta-field">
          <span>Tags</span>
          <input
            type="text"
            placeholder="comma, separated"
            bind:value={tagsEditor}
            oninput={() => (tagsDirty = true)}
            onblur={saveTags}
          />
        </label>
        <label class="meta-field">
          <span>Aliases</span>
          <input
            type="text"
            placeholder="comma, separated"
            bind:value={aliasesEditor}
            oninput={() => (aliasesDirty = true)}
            onblur={saveAliases}
          />
        </label>
      </div>

      {@const firstBlock = blockTree.length > 0 ? blockTree[0] : null}
      {#if blockTree.length > 1}
        <p class="editor-banner">
          Esta página tem múltiplos blocos antigos. C1 edita só o primeiro;
          os demais ficam preservados no DB e voltam visíveis em C1.5.
        </p>
      {/if}
      <Editor
        pageId={currentPage.id}
        aggregateBlockId={firstBlock?.id ?? null}
        initialMarkdown={firstBlock?.content ?? ""}
        onSaved={() => { void reloadTree(); }}
        onError={(msg) => showToast("err", msg)}
      />
    {/if}
  </main>

  <RefsPanel
    {linkedRefs}
    {unlinkedRefs}
    loading={refsLoading}
    active={currentPage !== null}
    pageId={currentPage?.id ?? null}
    onPageNavigate={gotoPageByName}
  />
</div>

<CoverManager
  open={coverManagerOpen}
  pageId={currentPage?.id ?? null}
  currentCoverUrl={coverUrl}
  onSaved={() => {
    if (currentPage) void loadCover(currentPage.id);
  }}
  onClose={() => (coverManagerOpen = false)}
/>

<HistoryModal
  open={historyOpen}
  blockId={firstBlockId}
  onRestored={() => {
    void reloadTree();
    showToast("ok", "Versão restaurada");
  }}
  onClose={() => (historyOpen = false)}
/>

<CreatePageDialog
  open={createPageOpen}
  onConfirm={createPage}
  onClose={() => (createPageOpen = false)}
/>

<RenamePageDialog
  open={renameOpen}
  initialValue={renameValue}
  onConfirm={renamePage}
  onClose={() => (renameOpen = false)}
/>

<ConfirmDialog
  bind:open={confirmDeletePageOpen}
  title="Excluir página"
  message={currentPage
    ? `"${currentPage.title ?? currentPage.name}" e todos os seus blocos serão removidos. Esta ação não pode ser desfeita.`
    : ""}
  confirmLabel="Excluir"
  variant="danger"
  onConfirm={deletePage}
/>

<style>
  .notes-shell {
    display: grid;
    grid-template-columns: 280px 1fr 320px;
    gap: 12px;
    height: calc(100vh - var(--header-height, 64px));
    overflow: hidden;
  }
  @media (max-width: 1100px) {
    .notes-shell {
      grid-template-columns: 240px 1fr;
    }
    .notes-shell :global(.refs-panel) {
      display: none;
    }
  }
  @media (max-width: 760px) {
    .notes-shell {
      grid-template-columns: 1fr;
    }
    .notes-shell :global(.sidebar) {
      display: none;
    }
  }

  .editor-banner {
    margin: 0;
    padding: 8px 12px;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    border-left: 3px solid var(--accent);
    border-radius: var(--border-radius);
    color: var(--secondary);
    font-size: 12px;
    line-height: 1.5;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px;
    overflow-y: auto;
  }
  .state {
    padding: calc(var(--padding) * 2);
    text-align: center;
    color: var(--secondary);
    font-size: 14px;
  }
  .state.err {
    color: var(--error, var(--accent));
  }

  .editor-head {
    display: flex;
    align-items: baseline;
    gap: 12px;
    flex-wrap: wrap;
  }
  .title {
    background: transparent;
    border: 0;
    padding: 0;
    font: inherit;
    font-size: 22px;
    font-weight: 600;
    color: var(--text);
    cursor: text;
  }
  .title:hover {
    color: var(--accent);
  }
  .head-actions {
    margin-left: auto;
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    color: var(--accent);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .page-meta-bar {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .meta-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: var(--tertiary);
  }
  .meta-field input {
    padding: 6px 10px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
  .meta-field input:focus {
    outline: none;
    border-color: var(--accent);
  }

</style>
