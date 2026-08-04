<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import ConfirmDialog from "$lib/study-components/ConfirmDialog.svelte";

  type PlaylistSummary = {
    id: number;
    name: string;
    description: string | null;
    color: string | null;
    course_count: number;
    updated_at: number;
  };

  type Course = {
    id: number;
    title: string;
    thumbnail_path: string | null;
    progress_pct: number;
  };

  type PlaylistEntry = {
    course_id: number;
    title: string;
    thumbnail_path: string | null;
    position: number;
    added_at: number;
  };

  let playlists = $state<PlaylistSummary[]>([]);
  let allCourses = $state<Course[]>([]);
  let selected = $state<PlaylistSummary | null>(null);
  let entries = $state<PlaylistEntry[]>([]);
  let loading = $state(true);
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  let createOpen = $state(false);
  let createName = $state("");
  let createColor = $state("#6366f1");
  let createDesc = $state("");

  let renameOpen = $state(false);
  let renameValue = $state("");

  let editMetaOpen = $state(false);
  let metaDesc = $state("");
  let metaColor = $state("");

  let confirmDeleteOpen = $state(false);

  let addCourseOpen = $state(false);
  let addCourseSearch = $state("");

  let dragIndex: number | null = null;

  const colors = [
    "#6366f1",
    "#ec4899",
    "#f97316",
    "#14b8a6",
    "#84cc16",
    "#f59e0b",
    "#06b6d4",
    "#8b5cf6",
  ];

  function showToast(kind: "ok" | "err", msg: string) {
    toast = { kind, msg };
    setTimeout(() => (toast = null), 2400);
  }

  function fmtDate(secs: number): string {
    if (!secs) return "—";
    return new Date(secs * 1000).toLocaleDateString();
  }

  async function load() {
    loading = true;
    try {
      const [pls, courses] = await Promise.all([
        pluginInvoke<PlaylistSummary[]>(
          "study",
          "study:courses:playlists:list",
        ),
        pluginInvoke<Course[]>("study", "study:courses:list"),
      ]);
      playlists = pls;
      allCourses = courses;
      if (selected) {
        const refreshed = playlists.find((p) => p.id === selected!.id);
        if (refreshed) {
          selected = refreshed;
          await loadEntries(refreshed.id);
        } else {
          selected = null;
          entries = [];
        }
      } else if (playlists.length > 0) {
        await selectPlaylist(playlists[0]);
      }
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      loading = false;
    }
  }

  async function selectPlaylist(p: PlaylistSummary) {
    selected = p;
    await loadEntries(p.id);
  }

  async function loadEntries(playlistId: number) {
    try {
      const items = await pluginInvoke<PlaylistEntry[]>(
        "study",
        "study:courses:playlists:courses",
        { playlistId },
      );
      entries = items;
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function createPlaylist() {
    const name = createName.trim();
    if (!name) return;
    try {
      const r = await pluginInvoke<{ id: number }>(
        "study",
        "study:courses:playlists:create",
        {
          name,
          description: createDesc.trim() || undefined,
          color: createColor,
        },
      );
      createOpen = false;
      createName = "";
      createDesc = "";
      await load();
      const created = playlists.find((p) => p.id === r.id);
      if (created) await selectPlaylist(created);
      showToast("ok", "Playlist criada");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function renamePlaylist() {
    if (!selected) return;
    const newName = renameValue.trim();
    if (!newName) {
      renameOpen = false;
      return;
    }
    try {
      await pluginInvoke("study", "study:courses:playlists:rename", {
        id: selected.id,
        newName,
      });
      renameOpen = false;
      await load();
      showToast("ok", "Renomeada");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function saveMeta() {
    if (!selected) return;
    try {
      await pluginInvoke("study", "study:courses:playlists:update_meta", {
        id: selected.id,
        description: metaDesc.trim() || null,
        color: metaColor || null,
      });
      editMetaOpen = false;
      await load();
      showToast("ok", "Atualizada");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function deletePlaylist() {
    if (!selected) return;
    try {
      await pluginInvoke("study", "study:courses:playlists:delete", {
        id: selected.id,
      });
      confirmDeleteOpen = false;
      selected = null;
      entries = [];
      await load();
      showToast("ok", "Removida");
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function addCourse(courseId: number) {
    if (!selected) return;
    try {
      await pluginInvoke("study", "study:courses:playlists:add_course", {
        playlistId: selected.id,
        courseId,
      });
      await loadEntries(selected.id);
      await load();
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function removeCourse(courseId: number) {
    if (!selected) return;
    try {
      await pluginInvoke("study", "study:courses:playlists:remove_course", {
        playlistId: selected.id,
        courseId,
      });
      await loadEntries(selected.id);
      await load();
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  async function reorderEntries(courseIds: number[]) {
    if (!selected) return;
    try {
      await pluginInvoke("study", "study:courses:playlists:reorder", {
        playlistId: selected.id,
        courseIds,
      });
      await loadEntries(selected.id);
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  function onDragStart(e: DragEvent, index: number) {
    dragIndex = index;
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
  }

  function onDrop(e: DragEvent, targetIndex: number) {
    e.preventDefault();
    if (dragIndex == null || dragIndex === targetIndex) {
      dragIndex = null;
      return;
    }
    const newOrder = [...entries];
    const [moved] = newOrder.splice(dragIndex, 1);
    newOrder.splice(targetIndex, 0, moved);
    entries = newOrder;
    reorderEntries(newOrder.map((e) => e.course_id));
    dragIndex = null;
  }

  const inPlaylistIds = $derived(new Set(entries.map((e) => e.course_id)));

  const filteredAddCandidates = $derived.by(() => {
    const q = addCourseSearch.trim().toLowerCase();
    return allCourses
      .filter(
        (c) =>
          !inPlaylistIds.has(c.id) &&
          (q.length === 0 || c.title.toLowerCase().includes(q)),
      )
      .slice(0, 30);
  });

  onMount(load);
</script>

<section class="pl-shell">
  <aside class="left">
    <header class="left-head">
      <a href="/study/library" class="back">← Library</a>
      <h2>Playlists</h2>
      <button class="btn primary" onclick={() => (createOpen = true)}>
        + Nova playlist
      </button>
    </header>

    {#if playlists.length === 0}
      <p class="empty">
        Sem playlists. Crie uma para agrupar cursos por trilha, prioridade,
        área de estudo, etc.
      </p>
    {:else}
      <ul class="pl-list">
        {#each playlists as p (p.id)}
          <li>
            <button
              class="pl-row"
              class:active={selected?.id === p.id}
              onclick={() => selectPlaylist(p)}
              style:--pl-color={p.color ?? "var(--accent)"}
            >
              <span class="pl-dot"></span>
              <span class="pl-name">{p.name}</span>
              <span class="pl-count">{p.course_count}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </aside>

  <main class="right">
    {#if toast}
      <div class="toast" class:err={toast.kind === "err"} role="status">
        {toast.msg}
      </div>
    {/if}

    {#if loading}
      <div class="state">Carregando…</div>
    {:else if !selected}
      <div class="state empty-state">
        <h2>Selecione uma playlist</h2>
        <p>Ou crie uma nova com o botão à esquerda.</p>
      </div>
    {:else}
      <header class="pl-head" style:--pl-color={selected.color ?? "var(--accent)"}>
        <span class="pl-color-dot"></span>
        <div class="title-block">
          <button
            class="title-btn"
            onclick={() => {
              renameValue = selected!.name;
              renameOpen = true;
            }}
          >
            <h1>{selected.name}</h1>
          </button>
          {#if selected.description}
            <p class="desc">{selected.description}</p>
          {/if}
          <p class="meta">
            {entries.length} cursos · atualizada em {fmtDate(selected.updated_at)}
          </p>
        </div>
        <div class="head-actions">
          <button
            class="btn ghost sm"
            onclick={() => {
              metaDesc = selected!.description ?? "";
              metaColor = selected!.color ?? "";
              editMetaOpen = true;
            }}
          >
            Editar
          </button>
          <button
            class="btn ghost sm danger"
            onclick={() => (confirmDeleteOpen = true)}
          >
            Excluir
          </button>
        </div>
      </header>

      <section class="actions-bar">
        <button class="btn primary" onclick={() => (addCourseOpen = true)}>
          + Adicionar curso
        </button>
      </section>

      {#if entries.length === 0}
        <div class="empty-state">
          <h3>Sem cursos nesta playlist</h3>
          <p>Use o botão acima para adicionar.</p>
        </div>
      {:else}
        <ol class="entries">
          {#each entries as e, i (e.course_id)}
            <li
              class="entry"
              draggable="true"
              ondragstart={(ev) => onDragStart(ev, i)}
              ondragover={onDragOver}
              ondrop={(ev) => onDrop(ev, i)}
            >
              <span class="grip" aria-hidden="true">⋮⋮</span>
              <span class="pos">{i + 1}</span>
              <a class="entry-title" href={`/study/course/${e.course_id}`}>
                {e.title}
              </a>
              <span class="added">{fmtDate(e.added_at)}</span>
              <button
                class="btn ghost sm"
                onclick={() => removeCourse(e.course_id)}
              >
                Remover
              </button>
            </li>
          {/each}
        </ol>
        <p class="reorder-hint">
          Arraste pelo <code>⋮⋮</code> para reordenar.
        </p>
      {/if}
    {/if}
  </main>
</section>

{#if createOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) createOpen = false;
    }}
  >
    <div class="modal">
      <h3>Nova playlist</h3>
      <label class="form-field">
        <span>Nome</span>
        <input
          type="text"
          bind:value={createName}
          placeholder="Ex: trilha frontend"
          onkeydown={(e) => {
            if (e.key === "Enter") createPlaylist();
            else if (e.key === "Escape") createOpen = false;
          }}
        />
      </label>
      <label class="form-field">
        <span>Descrição (opcional)</span>
        <input type="text" bind:value={createDesc} />
      </label>
      <div class="form-field">
        <span>Cor</span>
        <div class="color-picker">
          {#each colors as c (c)}
            <button
              type="button"
              class="color-swatch"
              class:active={createColor === c}
              style:background={c}
              onclick={() => (createColor = c)}
              aria-label={`Cor ${c}`}
            ></button>
          {/each}
        </div>
      </div>
      <footer>
        <button class="btn ghost" onclick={() => (createOpen = false)}>
          Cancelar
        </button>
        <button class="btn primary" onclick={createPlaylist}>Criar</button>
      </footer>
    </div>
  </div>
{/if}

{#if renameOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) renameOpen = false;
    }}
  >
    <div class="modal">
      <h3>Renomear playlist</h3>
      <input
        type="text"
        bind:value={renameValue}
        onkeydown={(e) => {
          if (e.key === "Enter") renamePlaylist();
          else if (e.key === "Escape") renameOpen = false;
        }}
      />
      <footer>
        <button class="btn ghost" onclick={() => (renameOpen = false)}>
          Cancelar
        </button>
        <button class="btn primary" onclick={renamePlaylist}>Renomear</button>
      </footer>
    </div>
  </div>
{/if}

{#if editMetaOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) editMetaOpen = false;
    }}
  >
    <div class="modal">
      <h3>Editar metadados</h3>
      <label class="form-field">
        <span>Descrição</span>
        <input type="text" bind:value={metaDesc} />
      </label>
      <div class="form-field">
        <span>Cor</span>
        <div class="color-picker">
          {#each colors as c (c)}
            <button
              type="button"
              class="color-swatch"
              class:active={metaColor === c}
              style:background={c}
              onclick={() => (metaColor = c)}
              aria-label={`Cor ${c}`}
            ></button>
          {/each}
          <button
            type="button"
            class="color-swatch clear"
            class:active={metaColor === ""}
            onclick={() => (metaColor = "")}
            aria-label="Sem cor"
          >×</button>
        </div>
      </div>
      <footer>
        <button class="btn ghost" onclick={() => (editMetaOpen = false)}>
          Cancelar
        </button>
        <button class="btn primary" onclick={saveMeta}>Salvar</button>
      </footer>
    </div>
  </div>
{/if}

{#if addCourseOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) addCourseOpen = false;
    }}
  >
    <div class="modal wide">
      <h3>Adicionar cursos</h3>
      <input
        type="text"
        placeholder="Buscar curso…"
        bind:value={addCourseSearch}
      />
      {#if filteredAddCandidates.length === 0}
        <p class="empty">Nenhum curso disponível.</p>
      {:else}
        <ul class="add-list">
          {#each filteredAddCandidates as c (c.id)}
            <li>
              <button class="add-row" onclick={() => addCourse(c.id)}>
                <span class="add-title">{c.title}</span>
                <span class="add-pct">{Math.round(c.progress_pct)}%</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      <footer>
        <button class="btn ghost" onclick={() => (addCourseOpen = false)}>
          Fechar
        </button>
      </footer>
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:open={confirmDeleteOpen}
  title="Excluir playlist"
  message={selected
    ? `"${selected.name}" será removida. Os cursos não serão deletados — apenas a playlist.`
    : ""}
  confirmLabel="Excluir"
  variant="danger"
  onConfirm={deletePlaylist}
/>

<style>
  .pl-shell {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 12px;
    height: calc(100vh - var(--header-height, 64px));
    overflow: hidden;
  }
  @media (max-width: 760px) {
    .pl-shell {
      grid-template-columns: 1fr;
    }
    .left {
      max-height: 220px;
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
  .back {
    color: var(--tertiary);
    font-size: 12px;
    text-decoration: none;
  }
  .back:hover {
    color: var(--accent);
  }
  .left-head h2 {
    margin: 0 0 6px;
    font-size: 16px;
  }
  .empty {
    color: var(--tertiary);
    font-size: 12px;
  }

  .pl-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pl-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .pl-row:hover {
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .pl-row.active {
    background: color-mix(in oklab, var(--pl-color) 14%, transparent);
  }
  .pl-dot {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    background: var(--pl-color, var(--accent));
    flex-shrink: 0;
  }
  .pl-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pl-count {
    color: var(--tertiary);
    font-size: 11px;
    font-weight: 500;
  }

  .right {
    padding: 24px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .state {
    text-align: center;
    color: var(--tertiary);
    padding: 60px 20px;
  }
  .empty-state h2,
  .empty-state h3 {
    color: var(--text);
    margin: 0 0 8px;
  }

  .pl-head {
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }
  .pl-color-dot {
    flex-shrink: 0;
    width: 10px;
    height: 32px;
    border-radius: 4px;
    background: var(--pl-color, var(--accent));
    margin-top: 4px;
  }
  .title-block {
    flex: 1;
  }
  .title-btn {
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--text);
    font: inherit;
    cursor: text;
    text-align: left;
  }
  .title-btn h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
  }
  .title-btn:hover h1 {
    color: var(--accent);
  }
  .desc {
    margin: 4px 0 0;
    color: var(--secondary);
    font-size: 13px;
  }
  .meta {
    margin: 4px 0 0;
    color: var(--tertiary);
    font-size: 11px;
  }
  .head-actions {
    display: flex;
    gap: 6px;
  }

  .actions-bar {
    display: flex;
    gap: 8px;
  }

  .entries {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .entry {
    display: grid;
    grid-template-columns: 24px 32px 1fr auto auto;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    background: var(--surface);
    transition: background 120ms ease;
  }
  .entry:hover {
    background: color-mix(in oklab, var(--accent) 4%, var(--surface));
  }
  .grip {
    color: var(--tertiary);
    cursor: grab;
    user-select: none;
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .grip:active {
    cursor: grabbing;
  }
  .pos {
    color: var(--tertiary);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    text-align: center;
  }
  .entry-title {
    color: var(--text);
    text-decoration: none;
    font-size: 14px;
  }
  .entry-title:hover {
    color: var(--accent);
  }
  .added {
    color: var(--tertiary);
    font-size: 11px;
  }
  .reorder-hint {
    color: var(--tertiary);
    font-size: 11px;
    margin: 4px 0 0;
  }
  .reorder-hint code {
    padding: 1px 5px;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    border-radius: 4px;
    font-family: var(--font-mono, ui-monospace, monospace);
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
    max-height: 80vh;
    overflow-y: auto;
  }
  .modal.wide {
    max-width: 600px;
  }
  .modal h3 {
    margin: 0;
    font-size: 16px;
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
  .form-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: var(--tertiary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .color-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    text-transform: none;
  }
  .color-swatch {
    width: 28px;
    height: 28px;
    border-radius: 999px;
    border: 2px solid transparent;
    cursor: pointer;
    padding: 0;
  }
  .color-swatch.active {
    border-color: var(--text);
    box-shadow: 0 0 0 2px var(--bg);
  }
  .color-swatch.clear {
    background: var(--bg);
    border-color: var(--input-border);
    color: var(--tertiary);
    font-size: 14px;
  }

  .add-list {
    list-style: none;
    margin: 0;
    padding: 4px;
    background: var(--bg);
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    max-height: 320px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .add-row {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 10px;
    border: 0;
    background: transparent;
    border-radius: 4px;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .add-row:hover {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .add-pct {
    color: var(--tertiary);
    font-size: 11px;
    font-family: var(--font-mono, ui-monospace, monospace);
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
    padding: 8px 14px;
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
</style>
