import { Node, mergeAttributes } from "@tiptap/core";
import { mount, unmount, type Component } from "svelte";
import DatabaseView from "./DatabaseView.svelte";

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    queryBlock: {
      setQueryBlock: (expr: string) => ReturnType;
    };
  }
}

export const QueryBlock = Node.create({
  name: "queryBlock",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  defining: true,

  addAttributes() {
    return {
      expr: {
        default: "",
        parseHTML: (el) => el.getAttribute("data-query-expr") ?? "",
        renderHTML: (attrs) => ({ "data-query-expr": attrs.expr ?? "" }),
      },
    };
  },

  parseHTML() {
    return [{ tag: "div[data-query-expr]" }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "div",
      mergeAttributes(HTMLAttributes, { class: "tiptap-query-block" }),
      "",
    ];
  },

  addCommands() {
    return {
      setQueryBlock:
        (expr: string) =>
        ({ commands }) =>
          commands.insertContent({
            type: this.name,
            attrs: { expr },
          }),
    };
  },

  addNodeView() {
    return ({ node, editor, getPos }) => {
      const dom = document.createElement("div");
      dom.className = "tiptap-query-block";
      dom.setAttribute("contenteditable", "false");
      dom.setAttribute("data-query-expr", String(node.attrs.expr ?? ""));

      const view = mount(DatabaseView as unknown as Component, {
        target: dom,
        props: {
          expr: String(node.attrs.expr ?? ""),
          onEdit: (newExpr: string) => {
            const pos = typeof getPos === "function" ? getPos() : null;
            if (pos == null) return;
            editor
              .chain()
              .focus()
              .command(({ tr }) => {
                tr.setNodeAttribute(pos, "expr", newExpr);
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
          dom.setAttribute("data-query-expr", String(updatedNode.attrs.expr ?? ""));
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
          node: { attrs: { expr?: string } },
        ) {
          const expr = (node.attrs.expr ?? "").trim();
          state.write(`{{query ${expr}}}`);
          state.closeBlock(node);
        },
        parse: {
          setup(_md: unknown) {
            return undefined;
          },
        },
      },
    };
  },
});
