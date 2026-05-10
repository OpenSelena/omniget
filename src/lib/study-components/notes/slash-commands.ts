import type { Editor, Range } from "@tiptap/core";

export type SlashCommandGroup =
  | "estrutura"
  | "blocos"
  | "callout"
  | "renderers"
  | "refs"
  | "datas"
  | "utilidades";

export type SlashCommand = {
  id: string;
  label: string;
  hint: string;
  aliases: string[];
  indicator: string;
  group: SlashCommandGroup;
  run: (editor: Editor, range: Range) => void;
};

const GROUP_ORDER: SlashCommandGroup[] = [
  "estrutura",
  "blocos",
  "callout",
  "renderers",
  "refs",
  "datas",
  "utilidades",
];

const GROUP_LABEL: Record<SlashCommandGroup, string> = {
  estrutura: "Estrutura",
  blocos: "Blocos",
  callout: "Callouts",
  renderers: "Renderers",
  refs: "Refs",
  datas: "Datas",
  utilidades: "Utilidades",
};

function todayIso(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function nowHm(): string {
  const d = new Date();
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

export const SLASH_COMMANDS: SlashCommand[] = [
  {
    id: "paragraph",
    label: "Parágrafo",
    hint: "Texto simples",
    aliases: ["p", "paragraph", "texto", "text"],
    indicator: "¶",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).setParagraph().run(),
  },
  {
    id: "h1",
    label: "Heading 1",
    hint: "Título principal",
    aliases: ["h1", "heading1", "titulo1", "t1"],
    indicator: "H1",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleHeading({ level: 1 }).run(),
  },
  {
    id: "h2",
    label: "Heading 2",
    hint: "Título de seção",
    aliases: ["h2", "heading2", "titulo2", "t2"],
    indicator: "H2",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleHeading({ level: 2 }).run(),
  },
  {
    id: "h3",
    label: "Heading 3",
    hint: "Sub-seção",
    aliases: ["h3", "heading3", "titulo3", "t3"],
    indicator: "H3",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleHeading({ level: 3 }).run(),
  },
  {
    id: "h4",
    label: "Heading 4",
    hint: "Sub-sub",
    aliases: ["h4", "heading4"],
    indicator: "H4",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleHeading({ level: 4 }).run(),
  },
  {
    id: "h5",
    label: "Heading 5",
    hint: "Hierarquia profunda",
    aliases: ["h5", "heading5"],
    indicator: "H5",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleHeading({ level: 5 }).run(),
  },
  {
    id: "h6",
    label: "Heading 6",
    hint: "Eyebrow",
    aliases: ["h6", "heading6"],
    indicator: "H6",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleHeading({ level: 6 }).run(),
  },
  {
    id: "bullet",
    label: "Lista",
    hint: "Bullet list",
    aliases: ["bullet", "list", "lista", "ul"],
    indicator: "•",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleBulletList().run(),
  },
  {
    id: "ordered",
    label: "Lista ordenada",
    hint: "Numeração automática",
    aliases: ["ordered", "ol", "numbered", "numerada"],
    indicator: "1.",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleOrderedList().run(),
  },
  {
    id: "task",
    label: "Tarefa",
    hint: "Checkbox",
    aliases: ["task", "todo", "tarefa", "check"],
    indicator: "[ ]",
    group: "estrutura",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleTaskList().run(),
  },
  {
    id: "quote",
    label: "Citação",
    hint: "Blockquote",
    aliases: ["quote", "blockquote", "citacao"],
    indicator: "❝",
    group: "blocos",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleBlockquote().run(),
  },
  {
    id: "code",
    label: "Bloco de código",
    hint: "Syntax highlight",
    aliases: ["code", "codigo", "pre"],
    indicator: "<>",
    group: "blocos",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).toggleCodeBlock().run(),
  },
  {
    id: "math",
    label: "Math block",
    hint: "LaTeX via KaTeX",
    aliases: ["math", "latex", "formula", "tex"],
    indicator: "Σ",
    group: "blocos",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent({ type: "blockMath", attrs: { latex: "" } })
        .run(),
  },
  {
    id: "query",
    label: "Database view (query)",
    hint: "Insere {{query <expr>}} live (table view)",
    aliases: ["query", "db", "database", "dataview"],
    indicator: "{}",
    group: "blocos",
    run: (editor, range) => {
      const expr = window.prompt(
        "Expressão da query (ex: (and (todo TODO))):",
        "(and (todo TODO))",
      );
      if (!expr) return;
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent({ type: "queryBlock", attrs: { expr: expr.trim() } })
        .run();
    },
  },
  {
    id: "embed-page",
    label: "Embed de página",
    hint: "Transcluir conteúdo de outra página",
    aliases: ["embed", "embedpage", "transclusao", "transclude"],
    indicator: "⤴",
    group: "blocos",
    run: (editor, range) => {
      const name = window.prompt("Nome da página a embedar:");
      if (!name || !name.trim()) return;
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent({
          type: "embedBlock",
          attrs: {
            targetKind: "page",
            targetName: name.trim(),
            targetUuid: "",
          },
        })
        .run();
    },
  },
  {
    id: "divider",
    label: "Divisor",
    hint: "Linha horizontal",
    aliases: ["divider", "hr", "rule", "linha"],
    indicator: "—",
    group: "blocos",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).setHorizontalRule().run(),
  },
  {
    id: "table",
    label: "Tabela",
    hint: "3×3 inicial",
    aliases: ["table", "tabela", "grid"],
    indicator: "▦",
    group: "blocos",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertTable({ rows: 3, cols: 3, withHeaderRow: true })
        .run(),
  },
  {
    id: "image",
    label: "Imagem (URL)",
    hint: "Cole a URL externa",
    aliases: ["image", "img", "imagem", "picture"],
    indicator: "▢",
    group: "blocos",
    run: (editor, range) => {
      const url = window.prompt("URL da imagem:");
      if (!url) return;
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .setImage({ src: url })
        .run();
    },
  },
  {
    id: "callout-note",
    label: "Callout: Note",
    hint: "Caixa informativa",
    aliases: ["note", "info", "callout"],
    indicator: "ℹ",
    group: "callout",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .setCallout("note")
        .run(),
  },
  {
    id: "callout-tip",
    label: "Callout: Tip",
    hint: "Dica destacada",
    aliases: ["tip", "dica", "callout"],
    indicator: "💡",
    group: "callout",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).setCallout("tip").run(),
  },
  {
    id: "callout-important",
    label: "Callout: Important",
    hint: "Destaque",
    aliases: ["important", "importante", "callout"],
    indicator: "❗",
    group: "callout",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).setCallout("important").run(),
  },
  {
    id: "callout-warning",
    label: "Callout: Warning",
    hint: "Aviso",
    aliases: ["warning", "aviso", "callout"],
    indicator: "⚠",
    group: "callout",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).setCallout("warning").run(),
  },
  {
    id: "callout-caution",
    label: "Callout: Caution",
    hint: "Cuidado",
    aliases: ["caution", "perigo", "danger", "callout"],
    indicator: "🚨",
    group: "callout",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).setCallout("caution").run(),
  },
  {
    id: "mermaid",
    label: "Diagrama (mermaid)",
    hint: "Flowcharts, sequência, gantt — sintaxe mermaid",
    aliases: ["mermaid", "diagram", "diagrama", "flow", "fluxo"],
    indicator: "▦",
    group: "renderers",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent({
          type: "mermaid",
          attrs: { source: "graph LR;\n  A-->B" },
        })
        .run(),
  },
  {
    id: "flowchart",
    label: "Fluxograma (flowchart.js)",
    hint: "Sintaxe flowchart.js: símbolos start/end/operation/condition",
    aliases: ["flowchart", "fluxograma", "fluxo", "fc"],
    indicator: "⇄",
    group: "renderers",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent({
          type: "flowchart",
          attrs: {
            source:
              "st=>start: Start\nop=>operation: Step\ne=>end: End\nst->op->e",
          },
        })
        .run(),
  },
  {
    id: "mindmap",
    label: "Mapa mental (markmap)",
    hint: "Markdown indentado: # raiz, ## ramo, ### folha",
    aliases: ["mindmap", "markmap", "mapa", "mental", "brain"],
    indicator: "⌘",
    group: "renderers",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent({
          type: "mindmap",
          attrs: {
            source: "# Root\n## Branch A\n### Leaf 1\n### Leaf 2\n## Branch B",
          },
        })
        .run(),
  },
  {
    id: "abc",
    label: "Partitura (notação ABC)",
    hint: "Notação musical ABC — escala, melodia, harmonia",
    aliases: ["abc", "musica", "music", "score", "partitura", "notation"],
    indicator: "♪",
    group: "renderers",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent({
          type: "abc",
          attrs: {
            source: "X:1\nT:Scale\nM:4/4\nL:1/4\nK:C\nC D E F | G A B c |",
          },
        })
        .run(),
  },
  {
    id: "plantuml",
    label: "Diagrama UML (PlantUML)",
    hint: "Sequência, classe, casos de uso — sintaxe PlantUML",
    aliases: ["plantuml", "puml", "uml", "diagrama", "sequence", "class"],
    indicator: "⚙",
    group: "renderers",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent({
          type: "plantuml",
          attrs: {
            source: "@startuml\nAlice -> Bob: hello\nBob --> Alice: hi\n@enduml",
          },
        })
        .run(),
  },
  {
    id: "link",
    label: "Link",
    hint: "[texto](url)",
    aliases: ["link", "url", "a"],
    indicator: "🔗",
    group: "refs",
    run: (editor, range) => {
      const url = window.prompt("URL:");
      if (!url) return;
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent(`[${url}](${url})`)
        .run();
    },
  },
  {
    id: "page-link",
    label: "Page link [[",
    hint: "Inicia link de página (autocomplete em D1)",
    aliases: ["page", "pagina", "wiki", "wikilink"],
    indicator: "[[",
    group: "refs",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).insertContent("[[").run(),
  },
  {
    id: "block-ref",
    label: "Block ref ((",
    hint: "Inicia ref de bloco (autocomplete em D1)",
    aliases: ["ref", "block", "bloco"],
    indicator: "((",
    group: "refs",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).insertContent("((").run(),
  },
  {
    id: "tag",
    label: "Tag #",
    hint: "Inicia hashtag",
    aliases: ["tag", "hashtag", "etiqueta"],
    indicator: "#",
    group: "refs",
    run: (editor, range) =>
      editor.chain().focus().deleteRange(range).insertContent("#").run(),
  },
  {
    id: "today",
    label: "Hoje (texto)",
    hint: "Insere YYYY-MM-DD",
    aliases: ["today", "hoje", "date", "data"],
    indicator: "📅",
    group: "datas",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent(`${todayIso()} `)
        .run(),
  },
  {
    id: "today-link",
    label: "Hoje (link)",
    hint: "Insere [[YYYY-MM-DD]]",
    aliases: ["todaylink", "datelink", "datalink"],
    indicator: "📆",
    group: "datas",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent(`[[${todayIso()}]] `)
        .run(),
  },
  {
    id: "now",
    label: "Agora (HH:MM)",
    hint: "Insere hora atual",
    aliases: ["now", "agora", "hora", "time"],
    indicator: "⏱",
    group: "datas",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent(`${nowHm()} `)
        .run(),
  },
  {
    id: "clear",
    label: "Limpar formatação",
    hint: "Remove marks da seleção",
    aliases: ["clear", "limpar", "unset"],
    indicator: "⌫",
    group: "utilidades",
    run: (editor, range) =>
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .unsetAllMarks()
        .clearNodes()
        .run(),
  },
  {
    id: "property",
    label: "Propriedade",
    hint: "Insere linha key:: value",
    aliases: ["property", "prop", "propriedade", "kv"],
    indicator: "::",
    group: "utilidades",
    run: (editor, range) => {
      const key = window.prompt("Chave da propriedade:");
      if (!key || !key.trim()) return;
      const cleanKey = key.trim().replace(/[^a-zA-Z0-9_-]/g, "_");
      const value = window.prompt(`Valor de ${cleanKey}:`) ?? "";
      editor
        .chain()
        .focus()
        .deleteRange(range)
        .insertContent(`${cleanKey}:: ${value}`)
        .run();
    },
  },
  {
    id: "template",
    label: "Aplicar template",
    hint: "Abre página de templates",
    aliases: ["template", "tpl", "templates"],
    indicator: "▤",
    group: "utilidades",
    run: (editor, range) => {
      editor.chain().focus().deleteRange(range).run();
      window.location.href = "/study/notes/templates";
    },
  },
];

export type FilteredSlashGroup = {
  group: SlashCommandGroup;
  label: string;
  items: SlashCommand[];
};

export function filterSlashCommands(query: string): FilteredSlashGroup[] {
  const q = query.trim().toLowerCase();
  const matches = q
    ? SLASH_COMMANDS.filter((c) => {
        if (c.label.toLowerCase().includes(q)) return true;
        if (c.id.toLowerCase().includes(q)) return true;
        if (c.aliases.some((a) => a.toLowerCase().includes(q))) return true;
        return false;
      })
    : SLASH_COMMANDS.slice();
  const grouped: Record<string, SlashCommand[]> = {};
  for (const c of matches) {
    (grouped[c.group] = grouped[c.group] || []).push(c);
  }
  const out: FilteredSlashGroup[] = [];
  for (const g of GROUP_ORDER) {
    if (grouped[g] && grouped[g].length > 0) {
      out.push({ group: g, label: GROUP_LABEL[g], items: grouped[g] });
    }
  }
  return out;
}
