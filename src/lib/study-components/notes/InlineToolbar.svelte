<script lang="ts">
  import type { Editor } from "@tiptap/core";

  type Props = {
    editor: Editor | null;
  };

  let { editor }: Props = $props();
  let element = $state<HTMLDivElement | undefined>(undefined);

  export function getElement(): HTMLDivElement | undefined {
    return element;
  }

  function isActive(name: string, attrs?: Record<string, unknown>): boolean {
    if (!editor) return false;
    return editor.isActive(name, attrs);
  }

  function toggleBold() {
    editor?.chain().focus().toggleBold().run();
  }
  function toggleItalic() {
    editor?.chain().focus().toggleItalic().run();
  }
  function toggleStrike() {
    editor?.chain().focus().toggleStrike().run();
  }
  function toggleCode() {
    editor?.chain().focus().toggleCode().run();
  }
  function setOrUnsetLink() {
    if (!editor) return;
    if (isActive("link")) {
      editor.chain().focus().unsetLink().run();
      return;
    }
    const url = window.prompt("URL:");
    if (!url) return;
    editor.chain().focus().setLink({ href: url, target: "_blank" }).run();
  }
</script>

<div bind:this={element} class="inline-toolbar" role="toolbar" aria-label="Inline formatting">
  <button
    type="button"
    class="tb-btn"
    class:active={isActive("bold")}
    title="Bold (Ctrl+B)"
    onmousedown={(e) => {
      e.preventDefault();
      toggleBold();
    }}
  >
    <strong>B</strong>
  </button>
  <button
    type="button"
    class="tb-btn"
    class:active={isActive("italic")}
    title="Italic (Ctrl+I)"
    onmousedown={(e) => {
      e.preventDefault();
      toggleItalic();
    }}
  >
    <em>I</em>
  </button>
  <button
    type="button"
    class="tb-btn"
    class:active={isActive("strike")}
    title="Strike"
    onmousedown={(e) => {
      e.preventDefault();
      toggleStrike();
    }}
  >
    <s>S</s>
  </button>
  <button
    type="button"
    class="tb-btn"
    class:active={isActive("code")}
    title="Code"
    onmousedown={(e) => {
      e.preventDefault();
      toggleCode();
    }}
  >
    <span class="mono">&lt;/&gt;</span>
  </button>
  <button
    type="button"
    class="tb-btn"
    class:active={isActive("link")}
    title="Link"
    onmousedown={(e) => {
      e.preventDefault();
      setOrUnsetLink();
    }}
  >
    🔗
  </button>
</div>

<style>
  .inline-toolbar {
    display: inline-flex;
    gap: 2px;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    box-shadow: 0 8px 24px color-mix(in oklab, black 24%, transparent);
  }
  .tb-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .tb-btn:hover {
    background: color-mix(in oklab, var(--accent) 10%, transparent);
  }
  .tb-btn.active {
    background: color-mix(in oklab, var(--accent) 20%, transparent);
    color: var(--accent);
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 11px;
  }
</style>
