<script lang="ts">
  type Props = {
    value: string;
    onChange: (next: string) => void;
    placeholder?: string;
    debounceMs?: number;
    globalShortcut?: boolean;
  };

  let {
    value,
    onChange,
    placeholder = "Buscar cursos…",
    debounceMs = 250,
    globalShortcut = true,
  }: Props = $props();

  let local = $state("");
  let input = $state<HTMLInputElement | null>(null);
  let timer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const incoming = value;
    if (incoming !== local) {
      local = incoming;
    }
  });

  function flush(next: string) {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      onChange(next);
    }, debounceMs);
  }

  function onInputChange(e: Event) {
    const next = (e.target as HTMLInputElement).value;
    local = next;
    flush(next);
  }

  function clear() {
    local = "";
    if (timer) clearTimeout(timer);
    onChange("");
    input?.focus();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape" && local.length > 0) {
      e.preventDefault();
      clear();
    }
  }

  function onGlobalKey(e: KeyboardEvent) {
    if (!globalShortcut) return;
    if (e.key !== "/") return;
    const target = e.target as HTMLElement | null;
    if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
      return;
    }
    e.preventDefault();
    input?.focus();
    input?.select();
  }

  $effect(() => {
    if (!globalShortcut) return;
    document.addEventListener("keydown", onGlobalKey);
    return () => document.removeEventListener("keydown", onGlobalKey);
  });

  $effect(() => {
    return () => {
      if (timer) clearTimeout(timer);
    };
  });
</script>

<div class="search-wrap">
  <span class="icon" aria-hidden="true">
    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="11" cy="11" r="7" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
  </span>
  <input
    type="search"
    bind:this={input}
    value={local}
    oninput={onInputChange}
    onkeydown={onKey}
    {placeholder}
    aria-label={placeholder}
    class="input"
  />
  {#if local.length > 0}
    <button type="button" class="clear" onclick={clear} aria-label="Limpar busca">
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  {:else if globalShortcut}
    <kbd class="hint" aria-hidden="true">/</kbd>
  {/if}
</div>

<style>
  .search-wrap {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: color-mix(in oklab, var(--content-bg) 80%, var(--accent) 3%);
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 8px;
    transition: border-color var(--tg-duration-fast, 150ms);
    min-width: 240px;
  }

  .search-wrap:focus-within {
    border-color: color-mix(in oklab, var(--accent) 60%, transparent);
  }

  .icon {
    color: color-mix(in oklab, currentColor 60%, transparent);
    flex: 0 0 auto;
  }

  .input {
    flex: 1 1 auto;
    border: none;
    background: transparent;
    color: inherit;
    font-size: 13px;
    outline: none;
    padding: 0;
    min-width: 0;
  }

  .input::-webkit-search-cancel-button {
    display: none;
  }

  .clear {
    background: transparent;
    border: none;
    color: color-mix(in oklab, currentColor 50%, transparent);
    cursor: pointer;
    padding: 2px;
    border-radius: 4px;
    display: grid;
    place-items: center;
    transition: color var(--tg-duration-fast, 150ms);
  }

  .clear:hover {
    color: var(--accent);
  }

  .hint {
    font-family: ui-monospace, monospace;
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 4px;
    background: color-mix(in oklab, var(--content-bg) 70%, var(--content-border) 30%);
    color: color-mix(in oklab, currentColor 60%, transparent);
    border: 1px solid color-mix(in oklab, var(--content-border) 80%, transparent);
  }
</style>
