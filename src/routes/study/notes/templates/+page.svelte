<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import ConfirmDialog from "$lib/study-components/ConfirmDialog.svelte";

  type TemplateSummary = {
    page_id: number;
    name: string;
    title: string | null;
    block_count: number;
    placeholders: string[];
    updated_at: number;
  };

  type PageSummary = {
    id: number;
    name: string;
    title: string | null;
    journal_day: number | null;
    block_count: number;
    updated_at: number;
  };

  type Block = {
    id: number;
    uuid: string;
    page_id: number;
    parent_id: number | null;
    order_idx: number;
    content: string;
    collapsed: boolean;
    properties: Record<string, unknown>;
    created_at: number;
    updated_at: number;
  };

  type BlockNode = Block & { children: BlockNode[] };

  const builtins: { kind: string; label: string; description: string }[] = [
    {
      kind: "daily-journal",
      label: "Daily journal",
      description: "Foco do dia, tarefas, notas, reflexão (com placeholders).",
    },
    {
      kind: "lesson-notes",
      label: "Lesson notes",
      description: "Estrutura para anotações de aula com objetivos e dúvidas.",
    },
    {
      kind: "book-highlights",
      label: "Book highlights",
      description: "Frontmatter com title/author + seções de highlights.",
    },
    {
      kind: "weekly-review",
      label: "Weekly review",
      description: "Conquistas, bloqueios, aprendizados, foco da próxima semana + query de tasks pendentes.",
    },
    {
      kind: "concept-page",
      label: "Concept page",
      description: "Definição, por que importa, exemplos, links relacionados, fonte.",
    },
  ];

  let templates = $state<TemplateSummary[]>([]);
  let allPages = $state<PageSummary[]>([]);
  let loading = $state(true);
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  let selected = $state<TemplateSummary | null>(null);
  let selectedPreview = $state<BlockNode[]>([]);
  let previewLoading = $state(false);

  let applyOpen = $state(false);
  let applyKind = $state<"builtin" | "user">("user");
  let applyBuiltinKind = $state("");
  let applyUserPageId = $state<number | null>(null);
  let applyUserPlaceholders = $state<string[]>([]);
  let applyTargetPageId = $state<number | null>(null);
  let applyTargetSearch = $state("");
  let applyVars = $state<Record<string, string>>({});
  let applying = $state(false);

  let markPageOpen = $state(false);
  let markPageSearch = $state("");

  let confirmUnmarkOpen = $state(false);

  function showToast(kind: "ok" | "err", msg: string) {
    toast = { kind, msg };
    setTimeout(() => (toast = null), 2400);
  }

  function fmt(secs: number): string {
    if (!secs) return "—";
    return new Date(secs * 1000).toLocaleDateString();
  }

  async function loadAll() {
    loading = true;
    try {
      const [tpls, pgs] = await Promise.all([
        pluginInvoke<TemplateSummary[]>("study", "study:notes:templates:list"),
        pluginInvoke<PageSummary[]>("study", "study:notes:pages:list"),
      ]);
      templates = tpls;
      allPages = pgs;
      if (templates.length > 0 && !selected) {
        await selectTemplate(templates[0]);
      }
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      loading = false;
    }
  }

  async function selectTemplate(t: TemplateSummary) {
    selected = t;
    previewLoading = true;
    try {
      const tree = await pluginInvoke<BlockNode[]>(
        "study",
        "study:notes:blocks:page_tree",
        { pageId: t.page_id },
      );
      selectedPreview = tree;
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
      selectedPreview = [];
    } finally {
      previewLoading = false;
    }
  }

  function flattenPreview(
    nodes: BlockNode[],
    depth = 0,
    out: { content: string; depth: number }[] = [],
  ) {
    for (const n of nodes) {
      out.push({ content: n.content, depth });
      flattenPreview(n.children, depth + 1, out);
    }
    return out;
  }

  const flatPreview = $derived(flattenPreview(selectedPreview));

  function openApplyForUserTemplate(t: TemplateSummary) {
    applyKind = "user";
    applyUserPageId = t.page_id;
    applyUserPlaceholders = t.placeholders;
    applyVars = Object.fromEntries(t.placeholders.map((p) => [p, ""]));
    applyTargetPageId = null;
    applyTargetSearch = "";
    applyOpen = true;
  }

  function openApplyForBuiltin(kind: string) {
    applyKind = "builtin";
    applyBuiltinKind = kind;
    applyVars = {};
    applyUserPlaceholders = builtinPlaceholders(kind);
    applyVars = Object.fromEntries(applyUserPlaceholders.map((p) => [p, ""]));
    applyTargetPageId = null;
    applyTargetSearch = "";
    applyOpen = true;
  }

  function builtinPlaceholders(kind: string): string[] {
    switch (kind) {
      case "daily-journal":
        return ["focus", "first-task", "mood"];
      case "lesson-notes":
        return ["course", "topic", "goal"];
      case "book-highlights":
        return ["title", "author"];
      default:
        return [];
    }
  }

  async function applyNow() {
    if (!applyTargetPageId) {
      showToast("err", "Escolha a página alvo");
      return;
    }
    applying = true;
    try {
      const cmd =
        applyKind === "builtin"
          ? "study:notes:templates:apply_builtin"
          : "study:notes:templates:apply";
      const args: Record<string, unknown> =
        applyKind === "builtin"
          ? {
              kind: applyBuiltinKind,
              targetPageId: applyTargetPageId,
              vars: applyVars,
            }
          : {
              templatePageId: applyUserPageId,
              targetPageId: applyTargetPageId,
              vars: applyVars,
            };
      const r = await pluginInvoke<{ blocks_created: number }>(
        "study",
        cmd,
        args,
      );
      applyOpen = false;
      showToast("ok", `${r.blocks_created} blocos adicionados`);
      goto(`/study/notes?page=${encodeURIComponent(targetPageName())}`);
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      applying = false;
    }
  }

  function targetPageName(): string {
    const p = allPages.find((x) => x.id === applyTargetPageId);
    return p?.name ?? "";
  }

  const filteredTargets = $derived.by(() => {
    const q = applyTargetSearch.trim().toLowerCase();
    if (!q) return allPages.slice(0, 20);
    return allPages
      .filter(
        (p) =>
          p.name.toLowerCase().includes(q) ||
          (p.title ?? "").toLowerCase().includes(q),
      )
      .slice(0, 20);
  });

  async function markAsTemplate(pageId: number) {
    try {
      await pluginInvoke("study", "study:notes:templates:mark", {
        pageId,
        isTemplate: true,
      });
      markPageOpen = false;
      await loadAll();
      showToast("ok", "Página marcada como template");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function unmarkAsTemplate() {
    if (!selected) return;
    try {
      await pluginInvoke("study", "study:notes:templates:mark", {
        pageId: selected.page_id,
        isTemplate: false,
      });
      confirmUnmarkOpen = false;
      selected = null;
      selectedPreview = [];
      await loadAll();
      showToast("ok", "Página desmarcada como template");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  const filteredMarkPages = $derived.by(() => {
    const q = markPageSearch.trim().toLowerCase();
    const isAlreadyTemplate = (id: number) =>
      templates.some((t) => t.page_id === id);
    return allPages
      .filter(
        (p) =>
          !isAlreadyTemplate(p.id) &&
          (q.length === 0 ||
            p.name.toLowerCase().includes(q) ||
            (p.title ?? "").toLowerCase().includes(q)),
      )
      .slice(0, 30);
  });

  onMount(loadAll);
</script>

<div class="tpl-shell">
  <aside class="left">
    <header class="left-head">
      <a href="/study/notes" class="back">← Notas</a>
      <h2 class="page-title">Templates</h2>
      <button
        class="btn ghost sm"
        onclick={() => {
          markPageSearch = "";
          markPageOpen = true;
        }}
      >
        + Marcar página
      </button>
    </header>

    <section>
      <h3>Built-in</h3>
      <ul class="t-list">
        {#each builtins as b (b.kind)}
          <li>
            <article class="t-card builtin">
              <header>
                <strong>{b.label}</strong>
                <span class="badge">built-in</span>
              </header>
              <p>{b.description}</p>
              <div class="placeholders">
                {#each builtinPlaceholders(b.kind) as ph (ph)}
                  <span class="ph">&lt;%{ph}%&gt;</span>
                {/each}
              </div>
              <button
                class="btn primary sm"
                onclick={() => openApplyForBuiltin(b.kind)}
              >
                Aplicar
              </button>
            </article>
          </li>
        {/each}
      </ul>
    </section>

    <section>
      <h3>Suas páginas marcadas</h3>
      {#if templates.length === 0}
        <p class="empty">
          Nenhuma página marcada como template. Marque uma página existente para
          reusá-la em outras páginas.
        </p>
      {:else}
        <ul class="t-list">
          {#each templates as t (t.page_id)}
            <li>
              <button
                class="t-card user"
                class:selected={selected?.page_id === t.page_id}
                onclick={() => selectTemplate(t)}
              >
                <header>
                  <strong>{t.title ?? t.name}</strong>
                  <span class="meta">{t.block_count} blocos</span>
                </header>
                <p class="path">{t.name}</p>
                {#if t.placeholders.length > 0}
                  <div class="placeholders">
                    {#each t.placeholders as ph (ph)}
                      <span class="ph">&lt;%{ph}%&gt;</span>
                    {/each}
                  </div>
                {/if}
                <span class="updated">atualizada em {fmt(t.updated_at)}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </aside>

  <main class="right">
    {#if toast}
      <div class="toast" class:err={toast.kind === "err"} role="status">
        {toast.msg}
      </div>
    {/if}

    {#if loading}
      <div class="state">Carregando templates…</div>
    {:else if !selected}
      <div class="state">
        <h3>Selecione um template à esquerda</h3>
        <p>
          Clique em um template para ver o preview, ou aplique um built-in
          direto.
        </p>
      </div>
    {:else}
      <header class="prev-head">
        <div>
          <h1>{selected.title ?? selected.name}</h1>
          <span class="path">{selected.name}</span>
        </div>
        <div class="actions">
          <button
            class="btn primary"
            onclick={() => {
              if (selected) openApplyForUserTemplate(selected);
            }}
          >
            Aplicar em uma página
          </button>
          <a class="btn ghost" href={`/study/notes?page=${encodeURIComponent(selected.name)}`}>
            Editar template
          </a>
          <button
            class="btn ghost danger"
            onclick={() => (confirmUnmarkOpen = true)}
          >
            Desmarcar
          </button>
        </div>
      </header>

      {#if selected.placeholders.length > 0}
        <section class="ph-detected">
          <h3>Placeholders detectados</h3>
          <div class="placeholders inline">
            {#each selected.placeholders as ph (ph)}
              <span class="ph">&lt;%{ph}%&gt;</span>
            {/each}
          </div>
          <p class="hint">
            Built-ins: <code>&lt;%today%&gt;</code>,
            <code>&lt;%now%&gt;</code>, <code>&lt;%year%&gt;</code> resolvem
            automaticamente.
          </p>
        </section>
      {/if}

      <section class="preview">
        <h3>Preview</h3>
        {#if previewLoading}
          <p>Carregando preview…</p>
        {:else if flatPreview.length === 0}
          <p class="empty">Template vazio.</p>
        {:else}
          <div class="block-preview">
            {#each flatPreview as item (item.content + item.depth)}
              <div class="prev-row" style:padding-left={`${item.depth * 18}px`}>
                <span class="bullet">•</span>
                <span class="prev-content">{item.content}</span>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  </main>
</div>

{#if applyOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) applyOpen = false;
    }}
  >
    <div class="modal wide">
      <h3>Aplicar template</h3>

      <label class="form-field">
        <span>Página alvo</span>
        <input
          type="text"
          placeholder="Buscar página…"
          bind:value={applyTargetSearch}
        />
        <ul class="target-list">
          {#each filteredTargets as p (p.id)}
            <li>
              <button
                class="target-row"
                class:selected={applyTargetPageId === p.id}
                onclick={() => (applyTargetPageId = p.id)}
              >
                <span>{p.title ?? p.name}</span>
                <span class="meta">{p.name}</span>
              </button>
            </li>
          {:else}
            <li class="empty">Nenhuma página encontrada.</li>
          {/each}
        </ul>
      </label>

      {#if applyUserPlaceholders.length > 0}
        <section>
          <h4>Variáveis</h4>
          <div class="vars">
            {#each applyUserPlaceholders as ph (ph)}
              <label class="form-field inline-label">
                <span class="lbl">&lt;%{ph}%&gt;</span>
                <input type="text" bind:value={applyVars[ph]} />
              </label>
            {/each}
          </div>
        </section>
      {/if}

      <footer>
        <button class="btn ghost" onclick={() => (applyOpen = false)}>
          Cancelar
        </button>
        <button class="btn primary" onclick={applyNow} disabled={applying}>
          {applying ? "Aplicando…" : "Aplicar"}
        </button>
      </footer>
    </div>
  </div>
{/if}

{#if markPageOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) markPageOpen = false;
    }}
  >
    <div class="modal wide">
      <h3>Marcar página como template</h3>
      <p class="hint">
        A página vira reusável. <code>&lt;%var%&gt;</code> no conteúdo viram
        placeholders preenchidos no apply.
      </p>
      <input
        type="text"
        placeholder="Buscar página…"
        bind:value={markPageSearch}
      />
      <ul class="target-list">
        {#each filteredMarkPages as p (p.id)}
          <li>
            <button class="target-row" onclick={() => markAsTemplate(p.id)}>
              <span>{p.title ?? p.name}</span>
              <span class="meta">{p.name}</span>
            </button>
          </li>
        {:else}
          <li class="empty">Nenhuma página disponível.</li>
        {/each}
      </ul>
      <footer>
        <button class="btn ghost" onclick={() => (markPageOpen = false)}>
          Fechar
        </button>
      </footer>
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:open={confirmUnmarkOpen}
  title="Desmarcar template"
  message="A página continua existindo, apenas perde a marcação como template."
  confirmLabel="Desmarcar"
  variant="danger"
  onConfirm={unmarkAsTemplate}
/>

<style>
  .tpl-shell {
    display: grid;
    grid-template-columns: 360px 1fr;
    gap: 12px;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }
  @media (max-width: 760px) {
    .tpl-shell {
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
    gap: 16px;
    overflow-y: auto;
  }
  .left-head {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .left-head h2 {
    margin: 0 0 4px;
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

  .left h3 {
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--tertiary);
  }

  .t-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .t-card {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px;
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    background: var(--surface);
    color: var(--text);
    text-align: left;
    font: inherit;
    cursor: pointer;
    transition: border-color 120ms ease;
  }
  .t-card:hover {
    border-color: var(--accent);
  }
  .t-card.selected {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 6%, var(--surface));
  }
  .t-card header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .t-card strong {
    font-size: 13px;
  }
  .t-card .meta {
    font-size: 11px;
    color: var(--tertiary);
  }
  .t-card p {
    margin: 0;
    font-size: 12px;
    color: var(--secondary);
    line-height: 1.4;
  }
  .t-card .path {
    color: var(--tertiary);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 10px;
  }
  .t-card .updated {
    color: var(--tertiary);
    font-size: 10px;
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
  .empty {
    color: var(--tertiary);
    font-size: 12px;
    padding: 6px 0;
  }

  .placeholders {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .placeholders.inline {
    margin-top: 8px;
  }
  .ph {
    padding: 2px 8px;
    border-radius: 4px;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    color: var(--accent);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
  }

  .right {
    padding: 24px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .right .state {
    text-align: center;
    color: var(--tertiary);
    padding: 60px 20px;
  }
  .right .state h3 {
    margin: 0 0 8px;
    color: var(--text);
  }

  .prev-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .prev-head h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
  }
  .prev-head .path {
    color: var(--tertiary);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .ph-detected {
    padding: 14px 16px;
    border-left: 3px solid var(--accent);
    background: color-mix(in oklab, var(--accent) 5%, transparent);
    border-radius: var(--border-radius);
  }
  .ph-detected h3 {
    margin: 0 0 6px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--tertiary);
  }
  .ph-detected .hint {
    margin: 8px 0 0;
    font-size: 12px;
    color: var(--secondary);
  }

  .preview h3 {
    margin: 0 0 8px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--tertiary);
  }
  .block-preview {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px;
    background: var(--bg);
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
  }
  .prev-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    color: var(--secondary);
    font-size: 13px;
    line-height: 1.5;
  }
  .bullet {
    color: var(--tertiary);
    flex-shrink: 0;
  }
  .prev-content {
    white-space: pre-wrap;
    flex: 1;
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
    max-width: 560px;
    background: var(--surface);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: 80vh;
    overflow-y: auto;
  }
  .modal.wide {
    max-width: 680px;
  }
  .modal h3 {
    margin: 0;
    font-size: 16px;
  }
  .modal h4 {
    margin: 8px 0 4px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--tertiary);
  }
  .modal .hint {
    color: var(--tertiary);
    font-size: 12px;
    margin: 0;
  }
  .form-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--tertiary);
  }
  .form-field input {
    padding: 8px 10px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
  .form-field input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .form-field.inline-label {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }
  .form-field.inline-label .lbl {
    flex: 0 0 140px;
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--accent);
  }

  .target-list {
    list-style: none;
    margin: 8px 0 0;
    padding: 4px;
    background: var(--bg);
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    max-height: 220px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .target-row {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 6px 10px;
    border: 0;
    background: transparent;
    border-radius: 4px;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font: inherit;
    font-size: 13px;
  }
  .target-row:hover {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .target-row.selected {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
  }
  .target-row .meta {
    font-size: 11px;
    color: var(--tertiary);
    font-family: var(--font-mono, ui-monospace, monospace);
  }

  .vars {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .modal footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 6px;
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
    z-index: 100;
  }
  .toast.err {
    background: color-mix(
      in oklab,
      var(--error, var(--accent)) 18%,
      var(--surface)
    );
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
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
  .btn.ghost.danger {
    color: var(--error, var(--accent));
    border-color: color-mix(
      in oklab,
      var(--error, var(--accent)) 40%,
      var(--input-border)
    );
  }
  .btn.sm {
    padding: 4px 10px;
    font-size: 11px;
  }

  code {
    padding: 1px 5px;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    border-radius: 4px;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
  }
</style>
