<script lang="ts">
  import { onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import PageHero from "$lib/study-components/PageHero.svelte";
  import ConfirmDialog from "$lib/study-components/ConfirmDialog.svelte";

  type NotetypeKind = "normal" | "cloze";

  type NotetypeSummary = {
    id: number;
    name: string;
    kind: NotetypeKind;
    field_count: number;
    template_count: number;
    mtime_secs: number;
  };

  type FieldConfig = {
    sticky: boolean;
    rtl: boolean;
    font_name: string;
    font_size: number;
    description: string;
    plain_text: boolean;
    collapsed: boolean;
    exclude_from_search: boolean;
    prevent_deletion: boolean;
  };

  type Field = { ord: number; name: string; config: FieldConfig };

  type TemplateConfig = {
    q_format: string;
    a_format: string;
    q_format_browser: string;
    a_format_browser: string;
    target_deck_id: number;
    browser_font_name: string;
    browser_font_size: number;
  };

  type Template = {
    ord: number;
    name: string;
    mtime_secs: number;
    usn: number;
    config: TemplateConfig;
  };

  type NotetypeConfig = {
    kind: NotetypeKind;
    sort_field_idx: number;
    css: string;
    latex_pre: string;
    latex_post: string;
    latex_svg: boolean;
    original_stock_kind: number;
  };

  type Notetype = {
    id: number;
    name: string;
    mtime_secs: number;
    usn: number;
    config: NotetypeConfig;
    fields: Field[];
    templates: Template[];
  };

  type StockKind =
    | "basic"
    | "basic_and_reversed"
    | "basic_optional_reversed"
    | "cloze";

  let loading = $state(true);
  let error = $state("");
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  let summaries = $state<NotetypeSummary[]>([]);
  let selected = $state<Notetype | null>(null);
  let selectedNoteCount = $state<number | null>(null);
  let detailLoading = $state(false);

  let createOpen = $state(false);
  let createKind = $state<StockKind>("basic");
  let createName = $state("");
  let creating = $state(false);

  let confirmOpen = $state(false);
  let pendingDelete = $state<NotetypeSummary | null>(null);

  let activeTemplateOrd = $state(0);

  let editMode = $state(false);
  let draft = $state<Notetype | null>(null);
  let saving = $state(false);
  let saveSummary = $state<{
    fields_added: number;
    fields_removed: number;
    fields_renamed: number;
    templates_added: number;
    templates_removed: number;
    templates_renamed: number;
    cards_added: number;
    cards_removed: number;
    notes_rewritten: number;
  } | null>(null);

  let cloneOpen = $state(false);
  let cloneName = $state("");
  let cloning = $state(false);

  function deepClone<T>(v: T): T {
    return JSON.parse(JSON.stringify(v)) as T;
  }

  function blankFieldConfig(): FieldConfig {
    return {
      sticky: false,
      rtl: false,
      font_name: "",
      font_size: 0,
      description: "",
      plain_text: false,
      collapsed: false,
      exclude_from_search: false,
      prevent_deletion: false,
    };
  }

  function blankTemplateConfig(): TemplateConfig {
    return {
      q_format: "",
      a_format: "",
      q_format_browser: "",
      a_format_browser: "",
      target_deck_id: 0,
      browser_font_name: "",
      browser_font_size: 0,
    };
  }

  const stockOptions: { value: StockKind; label: string; hint: string }[] = [
    {
      value: "basic",
      label: "Básico",
      hint: "Front/Back simples — 1 card por nota",
    },
    {
      value: "basic_and_reversed",
      label: "Básico + invertido",
      hint: "Front↔Back gera 2 cards por nota",
    },
    {
      value: "basic_optional_reversed",
      label: "Básico (invertido opcional)",
      hint: "Reverso só se campo 'Add Reverse' preenchido",
    },
    {
      value: "cloze",
      label: "Cloze",
      hint: "Use {{c1::texto}} para gerar 1 card por lacuna",
    },
  ];

  function showToast(kind: "ok" | "err", msg: string) {
    toast = { kind, msg };
    setTimeout(() => {
      toast = null;
    }, 2400);
  }

  function formatDate(secs: number): string {
    if (!secs) return "—";
    return new Date(secs * 1000).toLocaleDateString();
  }

  async function loadList() {
    loading = true;
    error = "";
    try {
      await pluginInvoke("study", "study:anki:storage:open");
      const list = await pluginInvoke<NotetypeSummary[]>(
        "study",
        "study:anki:notetypes:list",
      );
      summaries = list || [];
      if (selected) {
        const stillThere = summaries.find((s) => s.id === selected!.id);
        if (!stillThere) {
          selected = null;
          selectedNoteCount = null;
        }
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function selectNotetype(id: number) {
    detailLoading = true;
    try {
      const [nt, count] = await Promise.all([
        pluginInvoke<Notetype>("study", "study:anki:notetypes:get", { id }),
        pluginInvoke<{ items: unknown[]; total: number }>(
          "study",
          "study:anki:notes:list_by_notetype",
          { notetypeId: id, limit: 0, offset: 0 },
        ),
      ]);
      selected = nt;
      selectedNoteCount = count.total ?? 0;
      activeTemplateOrd = nt.templates[0]?.ord ?? 0;
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      detailLoading = false;
    }
  }

  function deselect() {
    selected = null;
    selectedNoteCount = null;
    editMode = false;
    draft = null;
    saveSummary = null;
  }

  function startEdit() {
    if (!selected) return;
    draft = deepClone(selected);
    editMode = true;
    saveSummary = null;
  }

  function cancelEdit() {
    editMode = false;
    draft = null;
    saveSummary = null;
  }

  function nextOrd(items: { ord: number }[]): number {
    return items.length === 0 ? 0 : Math.max(...items.map((x) => x.ord)) + 1;
  }

  function addField() {
    if (!draft) return;
    const ord = nextOrd(draft.fields);
    draft.fields = [
      ...draft.fields,
      { ord, name: `Field ${ord + 1}`, config: blankFieldConfig() },
    ];
  }

  function removeField(ord: number) {
    if (!draft) return;
    if (draft.fields.length <= 1) {
      showToast("err", "Modelo precisa ter ao menos 1 field");
      return;
    }
    draft.fields = draft.fields.filter((f) => f.ord !== ord);
    if (draft.config.sort_field_idx >= draft.fields.length) {
      draft.config.sort_field_idx = 0;
    }
  }

  function moveField(ord: number, direction: -1 | 1) {
    if (!draft) return;
    const arr = [...draft.fields].sort((a, b) => a.ord - b.ord);
    const idx = arr.findIndex((f) => f.ord === ord);
    const target = idx + direction;
    if (idx < 0 || target < 0 || target >= arr.length) return;
    [arr[idx], arr[target]] = [arr[target], arr[idx]];
    arr.forEach((f, i) => (f.ord = i));
    draft.fields = arr;
  }

  function addTemplate() {
    if (!draft) return;
    if (draft.config.kind === "cloze") {
      showToast("err", "Modelos cloze têm 1 template fixo");
      return;
    }
    const ord = nextOrd(draft.templates);
    draft.templates = [
      ...draft.templates,
      {
        ord,
        name: `Card ${ord + 1}`,
        mtime_secs: 0,
        usn: -1,
        config: {
          ...blankTemplateConfig(),
          q_format: "{{Front}}",
          a_format: "{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}",
        },
      },
    ];
    activeTemplateOrd = ord;
  }

  function removeTemplate(ord: number) {
    if (!draft) return;
    if (draft.templates.length <= 1) {
      showToast("err", "Modelo precisa ter ao menos 1 template");
      return;
    }
    draft.templates = draft.templates.filter((t) => t.ord !== ord);
    if (activeTemplateOrd === ord) {
      activeTemplateOrd = draft.templates[0]?.ord ?? 0;
    }
  }

  async function saveDraft() {
    if (!draft) return;
    saving = true;
    saveSummary = null;
    try {
      const summary = await pluginInvoke<typeof saveSummary>(
        "study",
        "study:anki:notetypes:update",
        { notetype: draft },
      );
      saveSummary = summary;
      showToast("ok", "Modelo atualizado");
      const id = draft.id;
      editMode = false;
      draft = null;
      await loadList();
      await selectNotetype(id);
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      saving = false;
    }
  }

  function openClone() {
    if (!selected) return;
    cloneName = `${selected.name} copy`;
    cloneOpen = true;
  }

  async function doClone() {
    if (!selected) return;
    cloning = true;
    try {
      const trimmed = cloneName.trim();
      const args: { id: number; name?: string } = { id: selected.id };
      if (trimmed.length > 0) args.name = trimmed;
      const r = await pluginInvoke<{ id: number }>(
        "study",
        "study:anki:notetypes:clone",
        args,
      );
      cloneOpen = false;
      showToast("ok", "Modelo clonado");
      await loadList();
      await selectNotetype(r.id);
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      cloning = false;
    }
  }

  function openCreate() {
    createKind = "basic";
    createName = "";
    createOpen = true;
  }

  async function doCreate() {
    creating = true;
    try {
      const args: { kind: StockKind; name?: string } = { kind: createKind };
      const trimmed = createName.trim();
      if (trimmed.length > 0) args.name = trimmed;
      const r = await pluginInvoke<{ id: number }>(
        "study",
        "study:anki:notetypes:create_stock",
        args,
      );
      createOpen = false;
      showToast("ok", "Modelo criado");
      await loadList();
      await selectNotetype(r.id);
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    } finally {
      creating = false;
    }
  }

  function askDelete(s: NotetypeSummary) {
    pendingDelete = s;
    confirmOpen = true;
  }

  async function confirmDelete() {
    if (!pendingDelete) return;
    const target = pendingDelete;
    pendingDelete = null;
    try {
      await pluginInvoke("study", "study:anki:notetypes:delete", {
        id: target.id,
      });
      if (selected?.id === target.id) deselect();
      showToast("ok", `"${target.name}" removido`);
      await loadList();
    } catch (e) {
      showToast("err", e instanceof Error ? e.message : String(e));
    }
  }

  const activeTemplate = $derived(
    selected?.templates.find((t) => t.ord === activeTemplateOrd) ??
      selected?.templates[0] ??
      null,
  );

  const sortFieldName = $derived.by(() => {
    if (!selected) return "";
    const idx = selected.config.sort_field_idx;
    return selected.fields[idx]?.name ?? selected.fields[0]?.name ?? "";
  });

  onMount(loadList);
</script>

<section class="study-page">
  <PageHero title="Modelos" subtitle="Note types da coleção">
    {#snippet actions()}
      <button class="btn primary" onclick={openCreate}>+ Novo modelo</button>
    {/snippet}
  </PageHero>

  {#if toast}
    <div class="toast" class:err={toast.kind === "err"} role="status">
      {toast.msg}
    </div>
  {/if}

  {#if loading}
    <div class="state">Carregando modelos…</div>
  {:else if error}
    <div class="state err">{error}</div>
    <button class="btn ghost" onclick={loadList}>Tentar de novo</button>
  {:else if summaries.length === 0}
    <div class="empty">
      <h3>Nenhum modelo ainda</h3>
      <p>
        Crie um modelo a partir de um stock type pra começar a adicionar notas.
      </p>
      <button class="btn primary" onclick={openCreate}>
        Criar primeiro modelo
      </button>
    </div>
  {:else}
    <div class="grid">
      <ul class="list" aria-label="Lista de modelos">
        {#each summaries as s (s.id)}
          <li>
            <button
              class="row"
              class:active={selected?.id === s.id}
              onclick={() => selectNotetype(s.id)}
            >
              <div class="row-name">
                <span class="badge" class:cloze={s.kind === "cloze"}>
                  {s.kind === "cloze" ? "Cloze" : "Normal"}
                </span>
                <span class="row-title">{s.name}</span>
              </div>
              <div class="row-meta">
                <span>{s.field_count} fields</span>
                <span>·</span>
                <span>{s.template_count} templates</span>
              </div>
            </button>
          </li>
        {/each}
      </ul>

      <div class="detail" aria-live="polite">
        {#if detailLoading}
          <div class="state">Carregando detalhes…</div>
        {:else if !selected}
          <div class="placeholder">
            <p>Selecione um modelo na lista pra ver fields, templates e CSS.</p>
          </div>
        {:else}
          <header class="detail-head">
            <div>
              {#if editMode && draft}
                <input
                  class="title-input"
                  type="text"
                  bind:value={draft.name}
                  aria-label="Nome do modelo"
                />
              {:else}
                <h3>{selected.name}</h3>
              {/if}
              <p class="detail-sub">
                {selected.config.kind === "cloze" ? "Cloze" : "Normal"}
                · {selected.fields.length} fields
                · {selected.templates.length} templates
                · atualizado em {formatDate(selected.mtime_secs)}
              </p>
            </div>
            <div class="detail-actions">
              {#if editMode}
                <button
                  class="btn ghost"
                  onclick={cancelEdit}
                  disabled={saving}
                >
                  Cancelar
                </button>
                <button
                  class="btn primary"
                  onclick={saveDraft}
                  disabled={saving}
                >
                  {saving ? "Salvando…" : "Salvar"}
                </button>
              {:else}
                <button class="btn ghost" onclick={openClone}>Clonar</button>
                <button class="btn ghost" onclick={startEdit}>Editar</button>
                <button
                  class="btn ghost danger"
                  onclick={() =>
                    askDelete({
                      id: selected!.id,
                      name: selected!.name,
                      kind: selected!.config.kind,
                      field_count: selected!.fields.length,
                      template_count: selected!.templates.length,
                      mtime_secs: selected!.mtime_secs,
                    })}
                >
                  Excluir
                </button>
              {/if}
            </div>
          </header>

          {#if editMode && selectedNoteCount !== null && selectedNoteCount > 0}
            <div class="info-banner warn">
              <strong>{selectedNoteCount}</strong>
              {selectedNoteCount === 1 ? "nota usa" : "notas usam"} este modelo.
              Adicionar/remover fields ou templates regenera cards e ajusta
              notas existentes. Tags, agenda e revisões ficam intactas.
            </div>
          {:else if !editMode && selectedNoteCount !== null && selectedNoteCount > 0}
            <div class="info-banner">
              <strong>{selectedNoteCount}</strong>
              {selectedNoteCount === 1 ? "nota usa" : "notas usam"} este modelo.
            </div>
          {:else}
            <div class="info-banner subtle">
              Nenhuma nota usa este modelo ainda.
            </div>
          {/if}

          {#if saveSummary}
            <div class="info-banner ok">
              Salvo:
              {#if saveSummary.fields_added}
                +{saveSummary.fields_added} {saveSummary.fields_added === 1 ? "field" : "fields"}
              {/if}
              {#if saveSummary.fields_removed}
                · −{saveSummary.fields_removed} {saveSummary.fields_removed === 1 ? "field" : "fields"}
              {/if}
              {#if saveSummary.templates_added}
                · +{saveSummary.templates_added} {saveSummary.templates_added === 1 ? "template" : "templates"}
              {/if}
              {#if saveSummary.templates_removed}
                · −{saveSummary.templates_removed} {saveSummary.templates_removed === 1 ? "template" : "templates"}
              {/if}
              {#if saveSummary.cards_added}
                · +{saveSummary.cards_added} {saveSummary.cards_added === 1 ? "card" : "cards"}
              {/if}
              {#if saveSummary.cards_removed}
                · −{saveSummary.cards_removed} {saveSummary.cards_removed === 1 ? "card" : "cards"}
              {/if}
              {#if saveSummary.notes_rewritten}
                · {saveSummary.notes_rewritten} {saveSummary.notes_rewritten === 1 ? "nota ajustada" : "notas ajustadas"}
              {/if}
            </div>
          {/if}

          {#if editMode && draft}
            <section class="block">
              <header class="block-head">
                <h4>Fields</h4>
                <button
                  class="btn ghost small"
                  onclick={addField}
                  disabled={saving}
                >
                  + Adicionar field
                </button>
              </header>
              <label class="lbl inline">
                <span>Sort field</span>
                <select
                  bind:value={draft.config.sort_field_idx}
                  disabled={saving}
                >
                  {#each draft.fields as f (f.ord)}
                    <option value={f.ord}>{f.name}</option>
                  {/each}
                </select>
              </label>
              <ol class="fields editable">
                {#each draft.fields as f (f.ord)}
                  <li>
                    <span class="ord">{f.ord + 1}</span>
                    <input
                      type="text"
                      bind:value={f.name}
                      placeholder="Nome do field"
                      disabled={saving}
                    />
                    <span class="flags">
                      <label class="flag">
                        <input
                          type="checkbox"
                          bind:checked={f.config.sticky}
                          disabled={saving}
                        />
                        sticky
                      </label>
                      <label class="flag">
                        <input
                          type="checkbox"
                          bind:checked={f.config.rtl}
                          disabled={saving}
                        />
                        RTL
                      </label>
                      <label class="flag">
                        <input
                          type="checkbox"
                          bind:checked={f.config.plain_text}
                          disabled={saving}
                        />
                        plain
                      </label>
                    </span>
                    <div class="row-actions">
                      <button
                        class="iconbtn"
                        title="Mover acima"
                        onclick={() => moveField(f.ord, -1)}
                        disabled={saving}
                      >
                        ↑
                      </button>
                      <button
                        class="iconbtn"
                        title="Mover abaixo"
                        onclick={() => moveField(f.ord, 1)}
                        disabled={saving}
                      >
                        ↓
                      </button>
                      <button
                        class="iconbtn danger"
                        title="Remover"
                        onclick={() => removeField(f.ord)}
                        disabled={saving || f.config.prevent_deletion}
                      >
                        ✕
                      </button>
                    </div>
                  </li>
                {/each}
              </ol>
            </section>

            <section class="block">
              <header class="block-head">
                <h4>Templates</h4>
                {#if draft.config.kind !== "cloze"}
                  <button
                    class="btn ghost small"
                    onclick={addTemplate}
                    disabled={saving}
                  >
                    + Adicionar template
                  </button>
                {/if}
              </header>
              {#if draft.templates.length > 1}
                <div class="tpl-tabs" role="tablist">
                  {#each draft.templates as t (t.ord)}
                    <button
                      role="tab"
                      aria-selected={activeTemplateOrd === t.ord}
                      class:active={activeTemplateOrd === t.ord}
                      onclick={() => (activeTemplateOrd = t.ord)}
                    >
                      {t.name}
                    </button>
                  {/each}
                </div>
              {/if}
              {#each draft.templates as t (t.ord)}
                {#if t.ord === activeTemplateOrd || draft.templates.length === 1}
                  <div class="tpl-edit">
                    <label class="lbl">
                      <span>Nome</span>
                      <input
                        type="text"
                        bind:value={t.name}
                        disabled={saving}
                      />
                    </label>
                    <div class="tpl-pane">
                      <div class="tpl-col">
                        <span class="tpl-label">Frente (Q)</span>
                        <textarea
                          bind:value={t.config.q_format}
                          rows="8"
                          spellcheck="false"
                          disabled={saving}
                        ></textarea>
                      </div>
                      <div class="tpl-col">
                        <span class="tpl-label">Verso (A)</span>
                        <textarea
                          bind:value={t.config.a_format}
                          rows="8"
                          spellcheck="false"
                          disabled={saving}
                        ></textarea>
                      </div>
                    </div>
                    {#if draft.templates.length > 1}
                      <button
                        class="btn ghost danger small"
                        onclick={() => removeTemplate(t.ord)}
                        disabled={saving}
                      >
                        Remover este template
                      </button>
                    {/if}
                  </div>
                {/if}
              {/each}
            </section>

            <section class="block">
              <h4>CSS global</h4>
              <textarea
                class="css-edit"
                bind:value={draft.config.css}
                rows="8"
                spellcheck="false"
                disabled={saving}
              ></textarea>
            </section>
          {:else}
            <section class="block">
              <h4>Fields</h4>
              <p class="hint">
                Sort field: <code>{sortFieldName}</code>
              </p>
              <ol class="fields">
                {#each selected.fields as f (f.ord)}
                  <li>
                    <span class="ord">{f.ord + 1}</span>
                    <span class="name">{f.name}</span>
                    <span class="flags">
                      {#if f.config.sticky}<span class="chip">sticky</span>{/if}
                      {#if f.config.rtl}<span class="chip">RTL</span>{/if}
                      {#if f.config.plain_text}
                        <span class="chip">plain</span>
                      {/if}
                      {#if f.config.exclude_from_search}
                        <span class="chip">no-search</span>
                      {/if}
                      {#if f.config.prevent_deletion}
                        <span class="chip locked">protegido</span>
                      {/if}
                    </span>
                    {#if f.config.description}
                      <span class="desc">{f.config.description}</span>
                    {/if}
                  </li>
                {/each}
              </ol>
            </section>

            <section class="block">
              <h4>Templates</h4>
              {#if selected.templates.length > 1}
                <div class="tpl-tabs" role="tablist">
                  {#each selected.templates as t (t.ord)}
                    <button
                      role="tab"
                      aria-selected={activeTemplateOrd === t.ord}
                      class:active={activeTemplateOrd === t.ord}
                      onclick={() => (activeTemplateOrd = t.ord)}
                    >
                      {t.name}
                    </button>
                  {/each}
                </div>
              {/if}

              {#if activeTemplate}
                <div class="tpl-pane">
                  <div class="tpl-col">
                    <span class="tpl-label">Frente (Q)</span>
                    <pre>{activeTemplate.config.q_format}</pre>
                  </div>
                  <div class="tpl-col">
                    <span class="tpl-label">Verso (A)</span>
                    <pre>{activeTemplate.config.a_format}</pre>
                  </div>
                </div>
              {/if}
            </section>

            <section class="block">
              <h4>CSS global</h4>
              <pre class="css">{selected.config.css || "/* sem CSS */"}</pre>
            </section>

            {#if selected.config.kind === "cloze"}
              <section class="block">
                <h4>Cloze</h4>
                <p class="hint">
                  Modelos cloze geram 1 card para cada
                  <code>{"{{c1::…}}"}</code>,
                  <code>{"{{c2::…}}"}</code> etc no campo. Use
                  <code>{"{{c1::dica::pista}}"}</code> para incluir uma pista.
                </p>
              </section>
            {/if}
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</section>

{#if createOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) createOpen = false;
    }}
  >
    <div class="modal" role="dialog" aria-modal="true">
      <h3>Novo modelo</h3>

      <label class="lbl">
        <span>Nome (opcional)</span>
        <input
          type="text"
          placeholder="ex.: Vocabulário inglês"
          bind:value={createName}
        />
      </label>

      <fieldset class="stocks">
        <legend>Stock type</legend>
        {#each stockOptions as opt (opt.value)}
          <label class="stock" class:active={createKind === opt.value}>
            <input
              type="radio"
              name="stock"
              value={opt.value}
              checked={createKind === opt.value}
              onchange={() => (createKind = opt.value)}
            />
            <div class="stock-body">
              <strong>{opt.label}</strong>
              <span>{opt.hint}</span>
            </div>
          </label>
        {/each}
      </fieldset>

      <footer class="modal-foot">
        <button
          class="btn ghost"
          onclick={() => (createOpen = false)}
          disabled={creating}
        >
          Cancelar
        </button>
        <button class="btn primary" onclick={doCreate} disabled={creating}>
          {creating ? "Criando…" : "Criar modelo"}
        </button>
      </footer>
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:open={confirmOpen}
  title="Excluir modelo"
  message={pendingDelete
    ? `"${pendingDelete.name}" será removido. Cards e notas associados também serão deletados. Esta ação não pode ser desfeita.`
    : ""}
  confirmLabel="Excluir"
  variant="danger"
  onConfirm={confirmDelete}
/>

{#if cloneOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) cloneOpen = false;
    }}
  >
    <div class="modal" role="dialog" aria-modal="true">
      <h3>Clonar modelo</h3>
      <p class="hint">
        Cria um novo modelo independente com fields, templates e CSS idênticos.
        Notas existentes não são copiadas.
      </p>
      <label class="lbl">
        <span>Nome do clone</span>
        <input type="text" bind:value={cloneName} disabled={cloning} />
      </label>
      <footer class="modal-foot">
        <button
          class="btn ghost"
          onclick={() => (cloneOpen = false)}
          disabled={cloning}
        >
          Cancelar
        </button>
        <button class="btn primary" onclick={doClone} disabled={cloning}>
          {cloning ? "Clonando…" : "Clonar"}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .study-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 1.25);
    width: 100%;
    max-width: 1100px;
    margin-inline: auto;
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

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: calc(var(--padding) * 3) var(--padding);
    border: 1px dashed
      color-mix(in oklab, var(--input-border) 70%, transparent);
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--accent) 3%, transparent);
    text-align: center;
  }
  .empty h3 {
    margin: 0;
    font-size: 16px;
  }
  .empty p {
    margin: 0;
    color: var(--secondary);
    font-size: 13px;
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
    animation: slide-up 180ms ease;
  }
  .toast.err {
    background: color-mix(
      in oklab,
      var(--error, var(--accent)) 18%,
      var(--surface)
    );
  }
  @keyframes slide-up {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .grid {
    display: grid;
    grid-template-columns: 320px 1fr;
    gap: var(--padding);
    align-items: flex-start;
  }
  @media (max-width: 760px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    background: var(--surface);
    padding: 6px;
    max-height: 70vh;
    overflow-y: auto;
  }
  .row {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border: 0;
    background: transparent;
    border-radius: calc(var(--border-radius) - 2px);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font: inherit;
    transition: background 120ms ease;
  }
  .row:hover {
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .row.active {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
  }
  .row:focus-visible {
    outline: 2px solid var(--focus-ring, var(--accent));
    outline-offset: -2px;
  }
  .row-name {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row-title {
    font-weight: 500;
    font-size: 13px;
  }
  .row-meta {
    color: var(--tertiary);
    font-size: 11px;
    display: flex;
    gap: 6px;
  }

  .badge {
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    color: var(--accent);
    font-weight: 600;
  }
  .badge.cloze {
    background: color-mix(
      in oklab,
      var(--warning, var(--accent)) 18%,
      transparent
    );
    color: var(--warning, var(--accent));
  }

  .detail {
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    background: var(--surface);
    padding: calc(var(--padding) * 1.25);
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 280px;
  }

  .placeholder {
    margin: auto;
    color: var(--tertiary);
    font-size: 13px;
    text-align: center;
  }

  .detail-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }
  .detail-head h3 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }
  .detail-sub {
    margin: 4px 0 0;
    color: var(--tertiary);
    font-size: 12px;
  }

  .detail-actions {
    display: flex;
    gap: 8px;
  }

  .info-banner {
    padding: 10px 14px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    color: var(--secondary);
    font-size: 12px;
    line-height: 1.5;
    border-left: 3px solid var(--accent);
  }
  .info-banner.subtle {
    background: color-mix(in oklab, var(--input-border) 30%, transparent);
    border-left-color: var(--tertiary);
  }
  .info-banner strong {
    color: var(--text);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid
      color-mix(in oklab, var(--input-border) 50%, transparent);
  }
  .block h4 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--tertiary);
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--secondary);
  }

  .fields {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .fields li {
    display: grid;
    grid-template-columns: 24px max-content 1fr;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--input-border) 18%, transparent);
    font-size: 13px;
  }
  .fields .ord {
    color: var(--tertiary);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
  }
  .fields .name {
    font-weight: 500;
    color: var(--text);
  }
  .fields .flags {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .fields .desc {
    grid-column: 2 / -1;
    color: var(--tertiary);
    font-size: 11px;
  }
  .chip {
    padding: 1px 6px;
    border-radius: 4px;
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    color: var(--accent);
    font-size: 10px;
    font-weight: 500;
  }
  .chip.locked {
    background: color-mix(
      in oklab,
      var(--warning, var(--accent)) 14%,
      transparent
    );
    color: var(--warning, var(--accent));
  }

  .tpl-tabs {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .tpl-tabs button {
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid
      color-mix(in oklab, var(--input-border) 60%, transparent);
    background: transparent;
    color: var(--secondary);
    cursor: pointer;
    font: inherit;
    font-size: 12px;
  }
  .tpl-tabs button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent, var(--on-cta, white));
  }

  .tpl-pane {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  @media (max-width: 760px) {
    .tpl-pane {
      grid-template-columns: 1fr;
    }
  }
  .tpl-col {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .tpl-label {
    font-size: 11px;
    color: var(--tertiary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  pre {
    margin: 0;
    padding: 10px 12px;
    background: var(--bg);
    border: 1px solid color-mix(in oklab, var(--input-border) 50%, transparent);
    border-radius: var(--border-radius);
    font-size: 11px;
    line-height: 1.55;
    color: var(--secondary);
    overflow-x: auto;
    max-height: 280px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }
  pre.css {
    max-height: 200px;
  }

  code {
    padding: 1px 5px;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    border-radius: 4px;
    font-size: 11px;
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--text);
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
    max-width: 520px;
    background: var(--surface);
    border-radius: var(--border-radius);
    padding: calc(var(--padding) * 1.5);
    display: flex;
    flex-direction: column;
    gap: 14px;
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    box-shadow: 0 20px 50px color-mix(in oklab, black 40%, transparent);
  }
  .modal h3 {
    margin: 0;
    font-size: 16px;
  }

  .lbl {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: var(--secondary);
  }
  .lbl input {
    padding: 8px 10px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
  .lbl input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--accent) 18%, transparent);
  }

  .stocks {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    border: 0;
  }
  .stocks legend {
    font-size: 12px;
    color: var(--tertiary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0 0 4px;
  }
  .stock {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid
      color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    cursor: pointer;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .stock:hover {
    background: color-mix(in oklab, var(--accent) 5%, transparent);
  }
  .stock.active {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .stock input {
    margin-top: 2px;
    accent-color: var(--accent);
  }
  .stock-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .stock-body strong {
    font-size: 13px;
    color: var(--text);
  }
  .stock-body span {
    font-size: 12px;
    color: var(--tertiary);
  }

  .modal-foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 6px;
    border-top: 1px solid
      color-mix(in oklab, var(--input-border) 40%, transparent);
  }

  .btn {
    padding: 8px 16px;
    border-radius: var(--border-radius);
    border: 1px solid transparent;
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 150ms ease, border-color 150ms ease;
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
  .btn.ghost.danger:hover {
    background: color-mix(
      in oklab,
      var(--error, var(--accent)) 10%,
      transparent
    );
  }

  .info-banner.warn {
    background: color-mix(in oklab, var(--warning, var(--accent)) 12%, transparent);
    border-left-color: var(--warning, var(--accent));
  }
  .info-banner.ok {
    background: color-mix(in oklab, var(--success, var(--accent)) 10%, transparent);
    border-left-color: var(--success, var(--accent));
  }

  .title-input {
    width: 100%;
    font-size: 17px;
    font-weight: 600;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    padding: 6px 10px;
    font-family: inherit;
  }
  .title-input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--accent) 18%, transparent);
  }

  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .block-head h4 {
    margin: 0;
  }

  .lbl.inline {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px;
    align-items: center;
  }
  .lbl.inline select {
    padding: 6px 8px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }

  .fields.editable li {
    grid-template-columns: 24px minmax(120px, 1fr) max-content max-content;
  }
  .fields.editable input[type="text"] {
    padding: 6px 8px;
    border: 1px solid
      color-mix(in oklab, var(--input-border) 70%, transparent);
    border-radius: calc(var(--border-radius) - 2px);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
  .fields.editable input[type="text"]:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklab, var(--accent) 18%, transparent);
  }
  .fields.editable .flag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    color: var(--secondary);
  }
  .fields.editable .flag input {
    accent-color: var(--accent);
  }
  .row-actions {
    display: flex;
    gap: 4px;
  }

  .iconbtn {
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid
      color-mix(in oklab, var(--input-border) 60%, transparent);
    background: var(--bg);
    color: var(--secondary);
    border-radius: 6px;
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .iconbtn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--text);
  }
  .iconbtn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .iconbtn.danger:hover:not(:disabled) {
    border-color: var(--error, var(--accent));
    color: var(--error, var(--accent));
  }

  .tpl-edit {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .tpl-edit textarea,
  textarea.css-edit {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid
      color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    line-height: 1.55;
    resize: vertical;
  }
  .tpl-edit textarea:focus,
  textarea.css-edit:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklab, var(--accent) 18%, transparent);
  }

  .btn.small {
    padding: 4px 10px;
    font-size: 12px;
  }

  @media (prefers-reduced-motion: reduce) {
    .row,
    .stock,
    .btn,
    .iconbtn,
    .toast {
      transition: none;
      animation: none;
    }
  }
</style>
