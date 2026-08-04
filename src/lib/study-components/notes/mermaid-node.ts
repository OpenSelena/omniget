import { Node, mergeAttributes } from "@tiptap/core";
import { mount, unmount, type Component } from "svelte";
import MermaidView from "./MermaidView.svelte";

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    mermaidBlock: {
      setMermaid: (source?: string) => ReturnType;
    };
  }
}

type MarkdownItToken = {
  type: string;
  tag?: string;
  info?: string;
  content?: string;
  markup?: string;
  block?: boolean;
  level?: number;
  meta?: unknown;
  attrs?: Array<[string, string]> | null;
  attrSet?: (name: string, value: string) => void;
  hidden?: boolean;
};

type MarkdownItState = {
  src: string;
  bMarks: number[];
  eMarks: number[];
  tShift: number[];
  sCount: number[];
  line: number;
  push: (type: string, tag: string, nesting: number) => MarkdownItToken;
};

type MarkdownItRule = (
  state: MarkdownItState,
  startLine: number,
  endLine: number,
  silent: boolean,
) => boolean;

type MarkdownItBlockRuler = {
  before: (
    name: string,
    ruleName: string,
    rule: MarkdownItRule,
    options?: { alt?: string[] },
  ) => void;
};

type MarkdownItInstance = {
  block: { ruler: MarkdownItBlockRuler };
};

const FENCE_OPEN = /^```mermaid\s*$/;
const FENCE_CLOSE = /^```\s*$/;

function mermaidFenceRule(
  state: MarkdownItState,
  startLine: number,
  endLine: number,
  silent: boolean,
): boolean {
  const startPos = state.bMarks[startLine] + state.tShift[startLine];
  const max = state.eMarks[startLine];
  if (state.sCount[startLine] - state.tShift[startLine] >= 4) return false;
  const opening = state.src.slice(startPos, max);
  if (!FENCE_OPEN.test(opening)) return false;
  if (silent) return true;

  let nextLine = startLine + 1;
  let closed = false;
  while (nextLine < endLine) {
    const lineStart = state.bMarks[nextLine] + state.tShift[nextLine];
    const lineEnd = state.eMarks[nextLine];
    const text = state.src.slice(lineStart, lineEnd);
    if (FENCE_CLOSE.test(text)) {
      closed = true;
      break;
    }
    nextLine++;
  }

  const lines: string[] = [];
  for (let i = startLine + 1; i < nextLine; i++) {
    const ls = state.bMarks[i] + state.tShift[i];
    const le = state.eMarks[i];
    lines.push(state.src.slice(ls, le));
  }
  const source = lines.join("\n");

  const token = state.push("mermaid", "div", 0);
  token.block = true;
  token.markup = "```";
  token.info = "mermaid";
  token.content = source;
  if (token.attrSet) {
    token.attrSet("source", source);
  } else {
    token.attrs = [["source", source]];
  }

  state.line = closed ? nextLine + 1 : nextLine;
  return true;
}

export const MermaidBlock = Node.create({
  name: "mermaid",
  group: "block",
  atom: false,
  selectable: true,
  draggable: true,
  defining: true,

  addAttributes() {
    return {
      source: {
        default: "",
        parseHTML: (el) => el.getAttribute("data-mermaid-source") ?? "",
        renderHTML: (attrs) => ({
          "data-mermaid-source": String(attrs.source ?? ""),
        }),
      },
    };
  },

  parseHTML() {
    return [{ tag: "div[data-mermaid]" }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "div",
      mergeAttributes(HTMLAttributes, {
        class: "tiptap-mermaid-block",
        "data-mermaid": "",
      }),
      "",
    ];
  },

  addCommands() {
    return {
      setMermaid:
        (source = "graph LR;\n  A-->B") =>
        ({ commands }) =>
          commands.insertContent({
            type: this.name,
            attrs: { source },
          }),
    };
  },

  addNodeView() {
    return ({ node, editor, getPos }) => {
      const dom = document.createElement("div");
      dom.className = "tiptap-mermaid-block";
      dom.setAttribute("data-mermaid", "");
      dom.setAttribute("contenteditable", "false");

      let lastSource = String(node.attrs.source ?? "");
      const blockId = Math.random().toString(36).slice(2, 10);

      const view = mount(MermaidView as unknown as Component, {
        target: dom,
        props: {
          source: lastSource,
          blockId,
          onSourceChange: (next: string) => {
            const pos = typeof getPos === "function" ? getPos() : null;
            if (pos == null) return;
            lastSource = next;
            editor
              .chain()
              .command(({ tr }) => {
                tr.setNodeMarkup(pos, undefined, {
                  ...node.attrs,
                  source: next,
                });
                return true;
              })
              .run();
          },
        },
      });

      return {
        dom,
        update: (updatedNode) => {
          if (updatedNode.type.name !== node.type.name) return false;
          const next = String(updatedNode.attrs.source ?? "");
          if (next !== lastSource) {
            lastSource = next;
            (view as unknown as { source?: string }).source = next;
          }
          return true;
        },
        destroy: () => {
          try {
            unmount(view);
          } catch {}
        },
      };
    };
  },

  addStorage() {
    return {
      markdown: {
        serialize(
          state: { write: (s: string) => void; closeBlock: (n: unknown) => void },
          node: { attrs: { source?: string } },
        ) {
          const src = String(node.attrs.source ?? "").replace(/\s+$/g, "");
          state.write("```mermaid\n");
          if (src.length > 0) {
            state.write(src);
            state.write("\n");
          }
          state.write("```");
          state.closeBlock(node);
        },
        parse: {
          setup(md: MarkdownItInstance) {
            md.block.ruler.before("fence", "mermaid_fence", mermaidFenceRule, {
              alt: ["paragraph", "reference", "blockquote", "list"],
            });
            return undefined;
          },
        },
      },
    };
  },
});
