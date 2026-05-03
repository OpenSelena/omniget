<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Editor } from "@tiptap/core";
  import StarterKit from "@tiptap/starter-kit";
  import TaskList from "@tiptap/extension-task-list";
  import TaskItem from "@tiptap/extension-task-item";
  import CodeBlockLowlight from "@tiptap/extension-code-block-lowlight";
  import { Image } from "@tiptap/extension-image";
  import { Table } from "@tiptap/extension-table";
  import { TableRow } from "@tiptap/extension-table-row";
  import { TableHeader } from "@tiptap/extension-table-header";
  import { TableCell } from "@tiptap/extension-table-cell";
  import { Mathematics } from "@tiptap/extension-mathematics";
  import "katex/dist/katex.min.css";
  import { Markdown } from "tiptap-markdown";
  import { createLowlight, common } from "lowlight";
  import { BubbleMenu } from "@tiptap/extension-bubble-menu";
  import { DragHandle } from "@tiptap/extension-drag-handle";
  import { Link } from "@tiptap/extension-link";
  import { Mention } from "@tiptap/extension-mention";
  import { Callout } from "./callout-node";
  import { QueryBlock } from "./query-node";
  import { EmbedBlock } from "./embed-node";
  import { createSlashExtension, type SlashSuggestionState } from "./slash-extension";
  import { NotesKeyboardShortcuts } from "./keyboard-shortcuts";
  import {
    createPageMentionSuggestion,
    createTagMentionSuggestion,
    type MentionSuggestionState,
  } from "./mention-suggestions";
  import SlashMenu from "./SlashMenu.svelte";
  import InlineToolbar from "./InlineToolbar.svelte";
  import MentionPopover from "./MentionPopover.svelte";
  import PagePopover from "./PagePopover.svelte";
  import type { SlashCommand } from "./slash-commands";
  import { notesBlocksUpdate, notesBlocksCreate } from "$lib/notes-bridge";

  type MarkdownStorage = { getMarkdown: () => string };
  function getMarkdownStorage(ed: Editor): MarkdownStorage | null {
    const storage = ed.storage as unknown as Record<string, unknown>;
    const md = storage.markdown;
    if (md && typeof (md as MarkdownStorage).getMarkdown === "function") {
      return md as MarkdownStorage;
    }
    return null;
  }

  type Props = {
    pageId: number;
    aggregateBlockId: number | null;
    initialMarkdown: string;
    onSaved?: (markdown: string, blockId: number) => void;
    onError?: (message: string) => void;
  };

  let { pageId, aggregateBlockId, initialMarkdown, onSaved, onError }: Props = $props();

  let element = $state<HTMLDivElement | undefined>(undefined);
  let bubbleElement = $state<HTMLDivElement | undefined>(undefined);
  let editorState = $state<{ editor: Editor | null }>({ editor: null });
  let isSaving = $state(false);
  let lastSavedAt = $state<number | null>(null);
  let slashState = $state<SlashSuggestionState>({
    open: false,
    query: "",
    groups: [],
    flat: [],
    selectedId: null,
    position: { x: 0, y: 0 },
    onPick: null,
  });

  let pageMentionState = $state<MentionSuggestionState>({
    open: false,
    kind: null,
    items: [],
    selectedIndex: 0,
    position: { x: 0, y: 0 },
    pick: () => {},
  });
  let tagMentionState = $state<MentionSuggestionState>({
    open: false,
    kind: null,
    items: [],
    selectedIndex: 0,
    position: { x: 0, y: 0 },
    pick: () => {},
  });

  let pagePopover = $state<{ name: string; position: { x: number; y: number } } | null>(null);
  let hoverTimeoutOpen: ReturnType<typeof setTimeout> | null = null;
  let hoverTimeoutClose: ReturnType<typeof setTimeout> | null = null;

  const HOVER_OPEN_DELAY = 400;
  const HOVER_CLOSE_DELAY = 200;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingMarkdown = "";
  let resolvedBlockId: number | null = null;

  const lowlight = createLowlight(common);

  async function ensureBlockId(): Promise<number> {
    if (resolvedBlockId !== null) return resolvedBlockId;
    const created = await notesBlocksCreate({ pageId, content: "" });
    resolvedBlockId = created.id;
    return resolvedBlockId;
  }

  function scheduleSave(markdown: string) {
    pendingMarkdown = markdown;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      void flush();
    }, 500);
  }

  async function flush() {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    if (pendingMarkdown === initialMarkdown && lastSavedAt === null) return;
    isSaving = true;
    try {
      const id = await ensureBlockId();
      await notesBlocksUpdate({ id, content: pendingMarkdown });
      lastSavedAt = Date.now();
      onSaved?.(pendingMarkdown, id);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      onError?.(msg);
    } finally {
      isSaving = false;
    }
  }

  $effect(() => {
    resolvedBlockId = aggregateBlockId;
    pendingMarkdown = initialMarkdown;
    lastSavedAt = null;
    const ed = editorState.editor;
    if (ed) {
      const md = getMarkdownStorage(ed);
      const current = md?.getMarkdown() ?? "";
      if (current !== initialMarkdown) {
        ed.commands.setContent(initialMarkdown || "", { emitUpdate: false });
      }
    }
  });

  onMount(() => {
    const ed = new Editor({
      element: element!,
      extensions: [
        StarterKit.configure({
          codeBlock: false,
          heading: { levels: [1, 2, 3, 4, 5, 6] },
        }),
        CodeBlockLowlight.configure({ lowlight }),
        TaskList,
        TaskItem.configure({ nested: true }),
        Image.configure({ inline: false, allowBase64: false }),
        Table.configure({ resizable: true, HTMLAttributes: { class: "tiptap-table" } }),
        TableRow,
        TableHeader,
        TableCell,
        Mathematics,
        Link.configure({
          openOnClick: false,
          autolink: true,
          HTMLAttributes: { rel: "noopener noreferrer", target: "_blank" },
        }),
        Callout,
        QueryBlock,
        EmbedBlock,
        NotesKeyboardShortcuts,
        Mention.extend({ name: "pageMention" }).configure({
          HTMLAttributes: { class: "tiptap-page-mention", "data-mention-kind": "page" },
          renderText: ({ node }) => `[[${node.attrs.label ?? node.attrs.id}]]`,
          suggestion: createPageMentionSuggestion((s) => {
            pageMentionState = s;
          }),
        }),
        Mention.extend({ name: "tagMention" }).configure({
          HTMLAttributes: { class: "tiptap-tag-mention", "data-mention-kind": "tag" },
          renderText: ({ node }) => `#${node.attrs.label ?? node.attrs.id}`,
          suggestion: createTagMentionSuggestion((s) => {
            tagMentionState = s;
          }),
        }),
        BubbleMenu.configure({
          element: bubbleElement!,
          shouldShow: ({ editor: ed, from, to }) => {
            if (!bubbleElement) return false;
            if (from === to) return false;
            if (ed.isActive("codeBlock")) return false;
            return true;
          },
        }),
        DragHandle.configure({
          render: () => {
            const handle = document.createElement("div");
            handle.className = "drag-handle";
            handle.setAttribute("aria-label", "Arraste o bloco");
            handle.title = "Arraste pra reordenar";
            handle.innerHTML = "<span aria-hidden=\"true\">⋮⋮</span>";
            return handle;
          },
        }),
        createSlashExtension((s) => {
          slashState = s;
        }),
        Markdown.configure({
          html: false,
          tightLists: true,
          tightListClass: "tight",
          bulletListMarker: "-",
          linkify: true,
          breaks: false,
          transformPastedText: true,
          transformCopiedText: false,
        }),
      ],
      content: initialMarkdown || "",
      editorProps: {
        attributes: {
          class: "tiptap",
          spellcheck: "false",
        },
      },
      onTransaction: ({ editor }) => {
        editorState = { editor };
      },
      onUpdate: ({ editor }) => {
        const md = getMarkdownStorage(editor);
        if (md) scheduleSave(md.getMarkdown());
      },
    });
    editorState = { editor: ed };

    if (element) {
      element.addEventListener("mouseover", onEditorMouseOver);
      element.addEventListener("mouseout", onEditorMouseOut);
      element.addEventListener("click", onEditorClick);
    }
  });

  function onEditorClick(event: MouseEvent) {
    if (!(event.target instanceof HTMLElement)) return;
    const tag = event.target.closest('[data-mention-kind="tag"]');
    if (tag instanceof HTMLElement) {
      event.preventDefault();
      const name = tag.textContent?.replace(/^#/, "").trim();
      if (!name) return;
      window.location.href = `/study/notes/tag/${encodeURIComponent(name)}`;
    }
  }

  function findMentionTarget(target: EventTarget | null): HTMLElement | null {
    if (!(target instanceof HTMLElement)) return null;
    const node = target.closest('[data-mention-kind="page"]');
    return node instanceof HTMLElement ? node : null;
  }

  function onEditorMouseOver(event: MouseEvent) {
    const target = findMentionTarget(event.target);
    if (!target) return;
    if (hoverTimeoutClose) {
      clearTimeout(hoverTimeoutClose);
      hoverTimeoutClose = null;
    }
    if (hoverTimeoutOpen) clearTimeout(hoverTimeoutOpen);
    hoverTimeoutOpen = setTimeout(() => {
      const name = target.textContent?.replace(/^\[\[|\]\]$/g, "").trim();
      if (!name) return;
      const rect = target.getBoundingClientRect();
      pagePopover = {
        name,
        position: { x: rect.left, y: rect.bottom + 4 },
      };
    }, HOVER_OPEN_DELAY);
  }

  function onEditorMouseOut(event: MouseEvent) {
    const target = findMentionTarget(event.target);
    if (!target) return;
    if (hoverTimeoutOpen) {
      clearTimeout(hoverTimeoutOpen);
      hoverTimeoutOpen = null;
    }
    if (hoverTimeoutClose) clearTimeout(hoverTimeoutClose);
    hoverTimeoutClose = setTimeout(() => {
      pagePopover = null;
    }, HOVER_CLOSE_DELAY);
  }

  function cancelPopoverClose() {
    if (hoverTimeoutClose) {
      clearTimeout(hoverTimeoutClose);
      hoverTimeoutClose = null;
    }
  }

  function schedulePopoverClose() {
    if (hoverTimeoutClose) clearTimeout(hoverTimeoutClose);
    hoverTimeoutClose = setTimeout(() => {
      pagePopover = null;
    }, HOVER_CLOSE_DELAY);
  }

  function navigateToPage(name: string) {
    const url = new URL(window.location.href);
    url.searchParams.set("page", name);
    window.location.href = url.toString();
  }

  onDestroy(() => {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
      void flush();
    }
    if (hoverTimeoutOpen) clearTimeout(hoverTimeoutOpen);
    if (hoverTimeoutClose) clearTimeout(hoverTimeoutClose);
    if (element) {
      element.removeEventListener("mouseover", onEditorMouseOver);
      element.removeEventListener("mouseout", onEditorMouseOut);
      element.removeEventListener("click", onEditorClick);
    }
    editorState.editor?.destroy();
  });

  export function focus() {
    editorState.editor?.commands.focus("end");
  }

  export function getMarkdown(): string {
    const ed = editorState.editor;
    if (!ed) return "";
    return getMarkdownStorage(ed)?.getMarkdown() ?? "";
  }
</script>

<div class="editor-host">
  {#if isSaving}
    <span class="save-indicator">salvando…</span>
  {:else if lastSavedAt}
    <span class="save-indicator subtle">salvo</span>
  {/if}
  <div bind:this={element} class="editor-mount"></div>
</div>

<div bind:this={bubbleElement} class="bubble-anchor">
  <InlineToolbar editor={editorState.editor} />
</div>

{#if slashState.open}
  <SlashMenu
    groups={slashState.groups}
    selectedId={slashState.selectedId}
    position={slashState.position}
    onPick={(cmd: SlashCommand) => slashState.onPick?.(cmd)}
  />
{/if}

{#if pageMentionState.open}
  <MentionPopover
    items={pageMentionState.items}
    selectedIndex={pageMentionState.selectedIndex}
    kind={pageMentionState.kind}
    position={pageMentionState.position}
    onPick={(item) => pageMentionState.pick(item)}
  />
{/if}

{#if tagMentionState.open}
  <MentionPopover
    items={tagMentionState.items}
    selectedIndex={tagMentionState.selectedIndex}
    kind={tagMentionState.kind}
    position={tagMentionState.position}
    onPick={(item) => tagMentionState.pick(item)}
  />
{/if}

{#if pagePopover}
  <div
    role="presentation"
    onmouseenter={cancelPopoverClose}
    onmouseleave={schedulePopoverClose}
  >
    <PagePopover
      pageName={pagePopover.name}
      position={pagePopover.position}
      onClose={() => (pagePopover = null)}
      onNavigate={navigateToPage}
    />
  </div>
{/if}

<style>
  .editor-host {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .editor-mount {
    flex: 1;
    min-height: 200px;
    padding: 4px;
  }
  .save-indicator {
    position: absolute;
    top: 4px;
    right: 8px;
    font-size: 10px;
    color: var(--tertiary);
    pointer-events: none;
    z-index: 2;
  }
  .save-indicator.subtle {
    opacity: 0.4;
  }
  .bubble-anchor {
    display: inline-block;
  }
  :global(.drag-handle) {
    position: absolute;
    user-select: none;
    cursor: grab;
    color: var(--tertiary);
    font-size: 14px;
    line-height: 1;
    padding: 4px;
    border-radius: var(--border-radius);
    opacity: 0.55;
    transition: opacity 120ms ease, background 120ms ease;
    z-index: 5;
  }
  :global(.drag-handle:hover),
  :global(.drag-handle:focus) {
    opacity: 1;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    color: var(--accent);
  }
  :global(.drag-handle:active) {
    cursor: grabbing;
  }
</style>
