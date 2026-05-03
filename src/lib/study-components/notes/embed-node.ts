import { Node, mergeAttributes } from "@tiptap/core";
import { mount, unmount, type Component } from "svelte";
import EmbedView from "./EmbedView.svelte";
import type { EmbedTarget } from "$lib/notes-bridge";

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    embedBlock: {
      setEmbedPage: (name: string) => ReturnType;
    };
  }
}

export const EmbedBlock = Node.create({
  name: "embedBlock",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  defining: true,

  addAttributes() {
    return {
      targetKind: {
        default: "page" as "page" | "block",
        parseHTML: (el) => el.getAttribute("data-embed-kind") ?? "page",
        renderHTML: (attrs) => ({ "data-embed-kind": attrs.targetKind ?? "page" }),
      },
      targetName: {
        default: "",
        parseHTML: (el) => el.getAttribute("data-embed-name") ?? "",
        renderHTML: (attrs) => ({ "data-embed-name": attrs.targetName ?? "" }),
      },
      targetUuid: {
        default: "",
        parseHTML: (el) => el.getAttribute("data-embed-uuid") ?? "",
        renderHTML: (attrs) => ({ "data-embed-uuid": attrs.targetUuid ?? "" }),
      },
    };
  },

  parseHTML() {
    return [{ tag: "div[data-embed-kind]" }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "div",
      mergeAttributes(HTMLAttributes, { class: "tiptap-embed-block" }),
      "",
    ];
  },

  addCommands() {
    return {
      setEmbedPage:
        (name: string) =>
        ({ commands }) =>
          commands.insertContent({
            type: this.name,
            attrs: { targetKind: "page", targetName: name, targetUuid: "" },
          }),
    };
  },

  addNodeView() {
    return ({ node }) => {
      const dom = document.createElement("div");
      dom.className = "tiptap-embed-block";
      dom.setAttribute("contenteditable", "false");
      const target: EmbedTarget =
        node.attrs.targetKind === "block"
          ? { kind: "block", uuid: String(node.attrs.targetUuid ?? "") }
          : { kind: "page", name: String(node.attrs.targetName ?? "") };
      const view = mount(EmbedView as unknown as Component, {
        target: dom,
        props: { target },
      });
      return {
        dom,
        update: (updatedNode) => {
          if (updatedNode.type.name !== node.type.name) return false;
          const sameKind = updatedNode.attrs.targetKind === node.attrs.targetKind;
          const sameName = updatedNode.attrs.targetName === node.attrs.targetName;
          const sameUuid = updatedNode.attrs.targetUuid === node.attrs.targetUuid;
          if (sameKind && sameName && sameUuid) return true;
          return false;
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
          node: {
            attrs: { targetKind?: string; targetName?: string; targetUuid?: string };
          },
        ) {
          const kind = node.attrs.targetKind ?? "page";
          if (kind === "block") {
            const uuid = (node.attrs.targetUuid ?? "").trim();
            state.write(`{{embed ((${uuid}))}}`);
          } else {
            const name = (node.attrs.targetName ?? "").trim();
            state.write(`{{embed [[${name}]]}}`);
          }
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
