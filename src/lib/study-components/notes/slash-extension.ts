import { Extension, type Editor, type Range } from "@tiptap/core";
import Suggestion, { type SuggestionOptions } from "@tiptap/suggestion";
import {
  filterSlashCommands,
  type FilteredSlashGroup,
  type SlashCommand,
} from "./slash-commands";

export type SlashSuggestionState = {
  open: boolean;
  query: string;
  groups: FilteredSlashGroup[];
  flat: SlashCommand[];
  selectedId: string | null;
  position: { x: number; y: number };
  onPick: ((cmd: SlashCommand) => void) | null;
};

export type SlashListener = (state: SlashSuggestionState) => void;

const closedState: SlashSuggestionState = {
  open: false,
  query: "",
  groups: [],
  flat: [],
  selectedId: null,
  position: { x: 0, y: 0 },
  onPick: null,
};

function flatten(groups: FilteredSlashGroup[]): SlashCommand[] {
  const out: SlashCommand[] = [];
  for (const g of groups) for (const c of g.items) out.push(c);
  return out;
}

export function createSlashExtension(listener: SlashListener) {
  let state: SlashSuggestionState = { ...closedState };
  const emit = () => listener(state);

  const suggestion: Omit<SuggestionOptions<SlashCommand>, "editor"> = {
    char: "/",
    startOfLine: false,
    allowSpaces: false,
    items: ({ query }) => {
      const groups = filterSlashCommands(query);
      return flatten(groups);
    },
    command: ({ editor, range, props }) => {
      props.run(editor, range);
    },
    render: () => {
      let activeEditor: Editor | null = null;
      let activeRange: Range | null = null;
      let groups: FilteredSlashGroup[] = [];
      let flat: SlashCommand[] = [];
      let selectedIndex = 0;

      function pickByIndex(idx: number) {
        const cmd = flat[idx];
        if (!cmd || !activeEditor || !activeRange) return;
        cmd.run(activeEditor, activeRange);
      }

      function rectFromClientRect(rect: DOMRect | null): { x: number; y: number } {
        if (!rect) return { x: 0, y: 0 };
        return { x: rect.left, y: rect.bottom + 4 };
      }

      return {
        onStart: (props) => {
          activeEditor = props.editor;
          activeRange = props.range;
          groups = filterSlashCommands(props.query);
          flat = flatten(groups);
          selectedIndex = 0;
          state = {
            open: true,
            query: props.query,
            groups,
            flat,
            selectedId: flat[0]?.id ?? null,
            position: rectFromClientRect(props.clientRect?.() ?? null),
            onPick: (cmd) => {
              cmd.run(activeEditor!, activeRange!);
            },
          };
          emit();
        },
        onUpdate: (props) => {
          activeEditor = props.editor;
          activeRange = props.range;
          groups = filterSlashCommands(props.query);
          flat = flatten(groups);
          selectedIndex = Math.min(selectedIndex, Math.max(0, flat.length - 1));
          state = {
            open: true,
            query: props.query,
            groups,
            flat,
            selectedId: flat[selectedIndex]?.id ?? null,
            position: rectFromClientRect(props.clientRect?.() ?? null),
            onPick: state.onPick,
          };
          emit();
        },
        onKeyDown: (props) => {
          if (!flat.length) return false;
          const key = props.event.key;
          if (key === "ArrowDown") {
            selectedIndex = (selectedIndex + 1) % flat.length;
            state = { ...state, selectedId: flat[selectedIndex]?.id ?? null };
            emit();
            return true;
          }
          if (key === "ArrowUp") {
            selectedIndex = (selectedIndex - 1 + flat.length) % flat.length;
            state = { ...state, selectedId: flat[selectedIndex]?.id ?? null };
            emit();
            return true;
          }
          if (key === "Enter" || key === "Tab") {
            pickByIndex(selectedIndex);
            return true;
          }
          if (key === "Escape") {
            state = { ...closedState };
            emit();
            return true;
          }
          return false;
        },
        onExit: () => {
          state = { ...closedState };
          emit();
        },
      };
    },
  };

  const extension = Extension.create({
    name: "slash-commands",
    addProseMirrorPlugins() {
      return [
        Suggestion({
          editor: this.editor,
          ...suggestion,
        }),
      ];
    },
  });

  return extension;
}
