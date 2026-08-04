<script lang="ts">
  import { onMount } from "svelte";
  import PageHero from "$lib/study-components/PageHero.svelte";

  let isMac = $state(false);

  onMount(() => {
    if (typeof navigator !== "undefined") {
      isMac = /Mac|iPhone|iPad|iPod/i.test(navigator.platform || navigator.userAgent || "");
    }
  });

  const meta = $derived(isMac ? "Cmd" : "Ctrl");

  type Row = { keys: string[]; desc: string };
  type Section = { title: string; rows: Row[] };

  const SECTIONS = $derived<Section[]>([
    {
      title: "Edição estrutural",
      rows: [
        { keys: ["Tab"], desc: "Indent (vira filho do bloco anterior)" },
        { keys: ["Shift+Tab"], desc: "Outdent (sobe um nível)" },
        { keys: ["Alt+↑"], desc: "Mover bloco pra cima" },
        { keys: ["Alt+↓"], desc: "Mover bloco pra baixo" },
        { keys: [`${meta}+Shift+K`], desc: "Excluir bloco (com confirmação)" },
        { keys: [`${meta}+/`], desc: "Colapsar/expandir bloco" },
        { keys: [`${meta}+D`], desc: "Duplicar bloco (com toda a subtree)" },
      ],
    },
    {
      title: "Status TODO",
      rows: [
        { keys: [`${meta}+Enter`], desc: "Cicla status (TODO → DOING → DONE → vazio)" },
      ],
    },
    {
      title: "Formatação inline",
      rows: [
        { keys: [`${meta}+B`], desc: "Negrito (`**texto**`)" },
        { keys: [`${meta}+I`], desc: "Itálico (`_texto_`)" },
        { keys: [`${meta}+Shift+S`], desc: "Tachado (`~~texto~~`)" },
        { keys: [`${meta}+Shift+C`], desc: "Code inline (`` `texto` ``)" },
        { keys: [`${meta}+Shift+.`], desc: "Blockquote (`> ` na linha)" },
      ],
    },
    {
      title: "Inserção via slash menu",
      rows: [
        { keys: ["/"], desc: "Abre slash menu (15 comandos)" },
        { keys: ["/todo /doing /done /later /now /waiting /canceled"], desc: "Define status do bloco" },
        { keys: ["/today"], desc: "Insere data ISO de hoje" },
        { keys: ["/date"], desc: "Insere link [[YYYY-MM-DD]] do journal de hoje" },
        { keys: ["/page /tag /block"], desc: "Inicia [[, # ou ((" },
        { keys: ["/code"], desc: "Insere bloco de código ``` ```" },
        { keys: ["/query"], desc: "Insere {{query (and (todo TODO))}} skeleton" },
        { keys: ["/embed page", "/embed block"], desc: "Insere {{embed [[…]]}} ou {{embed ((…))}}" },
      ],
    },
    {
      title: "Autocomplete inline",
      rows: [
        { keys: ["[["], desc: "Autocomplete de páginas existentes" },
        { keys: ["#"], desc: "Autocomplete de tags" },
        { keys: ["(("], desc: "Autocomplete de blocos recentes (uuid)" },
      ],
    },
    {
      title: "Histórico",
      rows: [
        { keys: [`${meta}+Z`], desc: "Desfaz última edição de conteúdo do bloco" },
        { keys: [`${meta}+Alt+Z`], desc: "Desfaz última operação estrutural (move/delete/insert)" },
        { keys: [`${meta}+Shift+Z`, `${meta}+Y`], desc: "Refaz última operação estrutural" },
      ],
    },
    {
      title: "Saída",
      rows: [
        { keys: ["Esc"], desc: "Fecha autocomplete / cancela seleção" },
      ],
    },
    {
      title: "Sintaxe Markdown reconhecida no preview",
      rows: [
        { keys: ["`> [!note]` `[!warn]` `[!info]` `[!success]` `[!tip]`"], desc: "Callout colorido abaixo do bloco" },
        { keys: ["` ```lang `\\n`código`\\n` ``` `"], desc: "Code block syntax-highlighted (preview)" },
        { keys: ["`$math$` ou `$$display$$`"], desc: "LaTeX renderizado via KaTeX (preview)" },
        { keys: ["`| col1 | col2 |`\\n`|---|---|`\\n`|...|...|`"], desc: "Tabela markdown renderizada abaixo" },
        { keys: ["`{{query (...)}}` `:sort X :limit N :offset M`"], desc: "Query inline com tabela ao vivo + paginação" },
      ],
    },
    {
      title: "Sintaxe de busca",
      rows: [
        { keys: ["`tag:project`"], desc: "Filtra blocos com link [[project]] ou #project" },
        { keys: ["`page:Daily`"], desc: "Filtra blocos da página Daily" },
        { keys: ["`status:DOING`"], desc: "Filtra por status property" },
        { keys: ["`before:2026-05-01`", "`after:2026-04-01`"], desc: "Janela de updated_at" },
        { keys: ["`tag:\"two words\"`"], desc: "Aspas pra valor com espaço" },
      ],
    },
  ]);
</script>

<section class="shortcuts-page">
  <PageHero
    title="Atalhos do editor de notas"
    subtitle="Detectado: {isMac ? 'Mac' : 'Windows/Linux'} ({meta} = {meta})"
  />

  <p class="muted small">
    Esta página é estática — todos os atalhos listados estão wired no editor
    em <code>/study/notes</code>. Se algo não funcionar, é bug.
  </p>

  {#each SECTIONS as section (section.title)}
    <section class="sec">
      <h2>{section.title}</h2>
      <table class="sc-table">
        <tbody>
          {#each section.rows as row (row.desc)}
            <tr>
              <td class="keys-cell">
                {#each row.keys as k, i (i)}
                  {#if i > 0} ou {/if}
                  {#each k.split("+") as part, j (j)}
                    {#if j > 0}<span class="plus">+</span>{/if}
                    <kbd>{part}</kbd>
                  {/each}
                {/each}
              </td>
              <td class="desc-cell">{row.desc}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  {/each}
</section>

<style>
  .shortcuts-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 1.25);
    width: 100%;
    max-width: 880px;
    margin-inline: auto;
  }
  .muted {
    color: var(--tertiary);
  }
  .small {
    font-size: 12px;
  }
  .sec {
    background: var(--surface);
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    padding: calc(var(--padding) * 0.9);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .sec h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent);
  }
  .sc-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .sc-table td {
    padding: 6px 8px;
    border-bottom: 1px solid color-mix(in oklab, var(--input-border) 40%, transparent);
    vertical-align: top;
  }
  .sc-table tr:last-child td {
    border-bottom: 0;
  }
  .keys-cell {
    width: 35%;
    white-space: nowrap;
  }
  .desc-cell {
    color: var(--secondary);
  }
  kbd {
    display: inline-block;
    padding: 2px 6px;
    background: var(--bg);
    border: 1px solid var(--input-border);
    border-bottom-width: 2px;
    border-radius: 4px;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
    color: var(--text);
    line-height: 1;
  }
  .plus {
    margin: 0 2px;
    color: var(--tertiary);
    font-size: 11px;
  }
  code {
    padding: 1px 4px;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    border-radius: 3px;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
  }
</style>
