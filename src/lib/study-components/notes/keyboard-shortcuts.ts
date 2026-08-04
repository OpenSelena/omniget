import { Extension } from "@tiptap/core";

const STATUS_CYCLE = ["TODO", "DOING", "DONE", "LATER", "WAITING", "CANCELED"];
const STATUS_PATTERN = /^(TODO|DOING|DONE|LATER|NOW|WAITING|CANCELED)\s+/;

function nextStatus(current: string | null): string | null {
  if (!current) return STATUS_CYCLE[0];
  const idx = STATUS_CYCLE.indexOf(current);
  if (idx === -1) return STATUS_CYCLE[0];
  if (idx === STATUS_CYCLE.length - 1) return null;
  return STATUS_CYCLE[idx + 1];
}

export const NotesKeyboardShortcuts = Extension.create({
  name: "notes-keyboard-shortcuts",

  addKeyboardShortcuts() {
    return {
      "Mod-Shift-0": () =>
        this.editor.chain().focus().setParagraph().run(),
      "Mod-Shift-1": () =>
        this.editor.chain().focus().toggleHeading({ level: 1 }).run(),
      "Mod-Shift-2": () =>
        this.editor.chain().focus().toggleHeading({ level: 2 }).run(),
      "Mod-Shift-3": () =>
        this.editor.chain().focus().toggleHeading({ level: 3 }).run(),
      "Mod-Shift-4": () =>
        this.editor.chain().focus().toggleHeading({ level: 4 }).run(),
      "Mod-Shift-5": () =>
        this.editor.chain().focus().toggleHeading({ level: 5 }).run(),
      "Mod-Shift-6": () =>
        this.editor.chain().focus().toggleHeading({ level: 6 }).run(),
      "Mod-Shift-7": () =>
        this.editor.chain().focus().toggleBulletList().run(),
      "Mod-Shift-8": () =>
        this.editor.chain().focus().toggleTaskList().run(),
      "Mod-Shift-9": () =>
        this.editor.chain().focus().toggleOrderedList().run(),
      "Mod-Shift-K": () => {
        const { selection } = this.editor.state;
        const $from = selection.$from;
        const blockDepth = $from.depth;
        if (blockDepth < 1) return false;
        const start = $from.before(blockDepth);
        const end = $from.after(blockDepth);
        return this.editor
          .chain()
          .focus()
          .deleteRange({ from: start, to: end })
          .run();
      },
      "Mod-d": () => {
        const { selection } = this.editor.state;
        const $from = selection.$from;
        const depth = $from.depth;
        if (depth < 1) return false;
        const start = $from.before(depth);
        const end = $from.after(depth);
        const node = this.editor.state.doc.slice(start, end).content;
        return this.editor.chain().focus().insertContentAt(end, node.toJSON()).run();
      },
      "Mod-/": () => {
        const { selection, doc } = this.editor.state;
        const $from = selection.$from;
        const depth = $from.depth;
        if (depth < 1) return false;
        const blockStart = $from.before(depth) + 1;
        const blockEnd = $from.after(depth) - 1;
        const blockNode = doc.nodeAt($from.before(depth));
        if (!blockNode) return false;
        const text = blockNode.textContent;
        const match = text.match(STATUS_PATTERN);
        const currentStatus = match ? match[1] : null;
        const next = nextStatus(currentStatus);
        if (currentStatus && next === null) {
          this.editor
            .chain()
            .focus()
            .deleteRange({ from: blockStart, to: blockStart + match![0].length })
            .run();
          return true;
        }
        if (currentStatus && next) {
          this.editor
            .chain()
            .focus()
            .insertContentAt(
              { from: blockStart, to: blockStart + match![0].length },
              `${next} `,
            )
            .run();
          return true;
        }
        if (next) {
          this.editor
            .chain()
            .focus()
            .insertContentAt(blockStart, `${next} `)
            .run();
          return true;
        }
        return false;
      },
    };
  },
});
