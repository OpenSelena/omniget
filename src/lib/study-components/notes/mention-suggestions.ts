import type { Editor, Range } from "@tiptap/core";
import type { SuggestionOptions } from "@tiptap/suggestion";
import {
  notesPagesList,
  notesTagsAutocomplete,
  type PageSummary,
  type TagSummary,
} from "$lib/notes-bridge";

export type MentionSuggestionState = {
  open: boolean;
  kind: "page" | "tag" | null;
  items: MentionItem[];
  selectedIndex: number;
  position: { x: number; y: number };
  pick: (item: MentionItem) => void;
};

export type MentionItem = {
  id: string;
  label: string;
  hint?: string;
};

export type MentionListener = (state: MentionSuggestionState) => void;

const closedState: MentionSuggestionState = {
  open: false,
  kind: null,
  items: [],
  selectedIndex: 0,
  position: { x: 0, y: 0 },
  pick: () => {},
};

let cachedPages: PageSummary[] | null = null;
let cachedPagesAt = 0;
const PAGE_CACHE_TTL = 30_000;

async function getPages(): Promise<PageSummary[]> {
  const now = Date.now();
  if (cachedPages && now - cachedPagesAt < PAGE_CACHE_TTL) return cachedPages;
  cachedPages = await notesPagesList();
  cachedPagesAt = now;
  return cachedPages;
}

function fuzzyFilterPages(pages: PageSummary[], query: string): PageSummary[] {
  if (!query.trim()) return pages.slice(0, 12);
  const q = query.toLowerCase();
  const scored: { page: PageSummary; score: number }[] = [];
  for (const p of pages) {
    const name = p.name.toLowerCase();
    const title = (p.title ?? "").toLowerCase();
    let score = 0;
    if (name === q || title === q) score = 1000;
    else if (name.startsWith(q) || title.startsWith(q)) score = 500;
    else if (name.includes(q) || title.includes(q)) score = 100;
    else continue;
    score += Math.max(0, 50 - name.length);
    scored.push({ page: p, score });
  }
  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, 12).map((s) => s.page);
}

function rectFromClientRect(rect: DOMRect | null): { x: number; y: number } {
  if (!rect) return { x: 0, y: 0 };
  return { x: rect.left, y: rect.bottom + 4 };
}

export function createPageMentionSuggestion(
  listener: MentionListener,
): Omit<SuggestionOptions<MentionItem>, "editor"> {
  let activeEditor: Editor | null = null;
  let activeRange: Range | null = null;
  let items: MentionItem[] = [];
  let selectedIndex = 0;

  function pickItem(item: MentionItem) {
    if (!activeEditor || !activeRange) return;
    activeEditor
      .chain()
      .focus()
      .insertContentAt(activeRange, `[[${item.label}]] `)
      .run();
  }

  function emit(open: boolean, position: { x: number; y: number }) {
    listener({
      open,
      kind: open ? "page" : null,
      items,
      selectedIndex,
      position,
      pick: pickItem,
    });
  }

  return {
    char: "[[",
    startOfLine: false,
    allowSpaces: true,
    items: async ({ query }) => {
      const pages = await getPages();
      return fuzzyFilterPages(pages, query).map((p) => ({
        id: String(p.id),
        label: p.title ?? p.name,
        hint: p.name,
      }));
    },
    command: ({ editor, range, props }) => {
      const start = range.from - 2;
      editor
        .chain()
        .focus()
        .insertContentAt({ from: start, to: range.to }, `[[${props.label}]] `)
        .run();
    },
    render: () => ({
      onStart: (props) => {
        activeEditor = props.editor;
        activeRange = props.range;
        items = props.items as MentionItem[];
        selectedIndex = 0;
        emit(items.length > 0, rectFromClientRect(props.clientRect?.() ?? null));
      },
      onUpdate: (props) => {
        activeEditor = props.editor;
        activeRange = props.range;
        items = props.items as MentionItem[];
        selectedIndex = Math.min(selectedIndex, Math.max(0, items.length - 1));
        emit(items.length > 0, rectFromClientRect(props.clientRect?.() ?? null));
      },
      onKeyDown: (props) => {
        if (!items.length) return false;
        const key = props.event.key;
        if (key === "ArrowDown") {
          selectedIndex = (selectedIndex + 1) % items.length;
          emit(true, listenerLastPosition);
          return true;
        }
        if (key === "ArrowUp") {
          selectedIndex = (selectedIndex - 1 + items.length) % items.length;
          emit(true, listenerLastPosition);
          return true;
        }
        if (key === "Enter" || key === "Tab") {
          pickItem(items[selectedIndex]);
          return true;
        }
        if (key === "Escape") {
          listener({ ...closedState });
          return true;
        }
        return false;
      },
      onExit: () => {
        listener({ ...closedState });
      },
    }),
  };
}

let listenerLastPosition = { x: 0, y: 0 };

export function createTagMentionSuggestion(
  listener: MentionListener,
): Omit<SuggestionOptions<MentionItem>, "editor"> {
  let activeEditor: Editor | null = null;
  let activeRange: Range | null = null;
  let items: MentionItem[] = [];
  let selectedIndex = 0;

  function pickItem(item: MentionItem) {
    if (!activeEditor || !activeRange) return;
    activeEditor
      .chain()
      .focus()
      .insertContentAt(activeRange, `#${item.label} `)
      .run();
  }

  function emit(open: boolean, position: { x: number; y: number }) {
    listenerLastPosition = position;
    listener({
      open,
      kind: open ? "tag" : null,
      items,
      selectedIndex,
      position,
      pick: pickItem,
    });
  }

  return {
    char: "#",
    startOfLine: false,
    allowSpaces: false,
    items: async ({ query }) => {
      try {
        const rows: TagSummary[] = await notesTagsAutocomplete({
          query,
          limit: 10,
        });
        return rows.map((t) => ({
          id: String(t.page_id),
          label: t.name,
          hint: `${t.ref_count} ref`,
        }));
      } catch {
        return [];
      }
    },
    command: ({ editor, range, props }) => {
      editor
        .chain()
        .focus()
        .insertContentAt(range, `#${props.label} `)
        .run();
    },
    render: () => ({
      onStart: (props) => {
        activeEditor = props.editor;
        activeRange = props.range;
        items = props.items as MentionItem[];
        selectedIndex = 0;
        emit(items.length > 0, rectFromClientRect(props.clientRect?.() ?? null));
      },
      onUpdate: (props) => {
        activeEditor = props.editor;
        activeRange = props.range;
        items = props.items as MentionItem[];
        selectedIndex = Math.min(selectedIndex, Math.max(0, items.length - 1));
        emit(items.length > 0, rectFromClientRect(props.clientRect?.() ?? null));
      },
      onKeyDown: (props) => {
        if (!items.length) return false;
        const key = props.event.key;
        if (key === "ArrowDown") {
          selectedIndex = (selectedIndex + 1) % items.length;
          emit(true, listenerLastPosition);
          return true;
        }
        if (key === "ArrowUp") {
          selectedIndex = (selectedIndex - 1 + items.length) % items.length;
          emit(true, listenerLastPosition);
          return true;
        }
        if (key === "Enter" || key === "Tab") {
          pickItem(items[selectedIndex]);
          return true;
        }
        if (key === "Escape") {
          listener({ ...closedState });
          return true;
        }
        return false;
      },
      onExit: () => {
        listener({ ...closedState });
      },
    }),
  };
}

export function invalidatePageCache() {
  cachedPages = null;
}
