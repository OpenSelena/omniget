import { Node, mergeAttributes, wrappingInputRule } from "@tiptap/core";

export type CalloutKind = "note" | "tip" | "important" | "warning" | "caution";

const VALID_KINDS: ReadonlyArray<CalloutKind> = [
  "note",
  "tip",
  "important",
  "warning",
  "caution",
];

const ICON: Record<CalloutKind, string> = {
  note: "ℹ︎",
  tip: "💡",
  important: "❗",
  warning: "⚠︎",
  caution: "🚨",
};

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    callout: {
      setCallout: (kind?: CalloutKind) => ReturnType;
      toggleCallout: (kind?: CalloutKind) => ReturnType;
      unsetCallout: () => ReturnType;
    };
  }
}

export const Callout = Node.create({
  name: "callout",
  group: "block",
  content: "block+",
  defining: true,

  addAttributes() {
    return {
      kind: {
        default: "note" as CalloutKind,
        parseHTML: (element) => {
          const raw = element.getAttribute("data-callout-kind");
          return VALID_KINDS.includes(raw as CalloutKind) ? raw : "note";
        },
        renderHTML: (attributes) => {
          const kind = VALID_KINDS.includes(attributes.kind)
            ? attributes.kind
            : "note";
          return { "data-callout-kind": kind };
        },
      },
    };
  },

  parseHTML() {
    return [{ tag: "aside[data-callout-kind]" }];
  },

  renderHTML({ HTMLAttributes }) {
    const kind = (HTMLAttributes["data-callout-kind"] || "note") as CalloutKind;
    return [
      "aside",
      mergeAttributes(HTMLAttributes, { class: "tiptap-callout" }),
      ["span", { class: "tiptap-callout-icon", contenteditable: "false" }, ICON[kind]],
      ["div", { class: "tiptap-callout-body" }, 0],
    ];
  },

  addCommands() {
    return {
      setCallout:
        (kind = "note" as CalloutKind) =>
        ({ commands }) =>
          commands.wrapIn(this.name, { kind }),
      toggleCallout:
        (kind = "note" as CalloutKind) =>
        ({ commands, editor }) => {
          if (editor.isActive(this.name)) return commands.lift(this.name);
          return commands.wrapIn(this.name, { kind });
        },
      unsetCallout:
        () =>
        ({ commands }) =>
          commands.lift(this.name),
    };
  },

  addInputRules() {
    return VALID_KINDS.map((kind) =>
      wrappingInputRule({
        find: new RegExp(`^>\\s*\\[!${kind.toUpperCase()}\\]\\s$`, "i"),
        type: this.type,
        getAttributes: () => ({ kind }),
      }),
    );
  },
});
