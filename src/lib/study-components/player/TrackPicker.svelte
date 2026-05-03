<script lang="ts" generics="T extends { path: string; lang: string; label: string; format: string; default: boolean }">
  type Props = {
    label: string;
    icon?: "captions" | "audio";
    tracks: T[];
    selectedLang: string | null;
    onSelect: (lang: string | null) => void;
    showOff?: boolean;
    settingsHref?: string;
  };

  let {
    label,
    icon = "captions",
    tracks,
    selectedLang,
    onSelect,
    showOff = true,
    settingsHref,
  }: Props = $props();

  let open = $state(false);
  let trigger = $state<HTMLButtonElement | null>(null);
  let menu = $state<HTMLDivElement | null>(null);

  const currentLabel = $derived.by(() => {
    if (selectedLang == null) return showOff ? "Off" : tracks[0]?.label ?? "—";
    const found = tracks.find((t) => t.lang === selectedLang);
    return found?.label ?? "—";
  });

  function pick(lang: string | null) {
    onSelect(lang);
    open = false;
    trigger?.focus();
  }

  function onDocClick(e: MouseEvent) {
    if (!open) return;
    const target = e.target as Node;
    if (trigger?.contains(target) || menu?.contains(target)) return;
    open = false;
  }

  function onKey(e: KeyboardEvent) {
    if (open && e.key === "Escape") {
      open = false;
      trigger?.focus();
    }
  }

  $effect(() => {
    if (!open) return;
    document.addEventListener("click", onDocClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("click", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="picker">
  <button
    type="button"
    class="trigger"
    bind:this={trigger}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={label}
    title={label}
    onclick={() => (open = !open)}
  >
    {#if icon === "captions"}
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <rect x="3" y="5" width="18" height="14" rx="2" />
        <path d="M7 12h3M14 12h3M7 16h6" />
      </svg>
    {:else}
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M11 5L6 9H2v6h4l5 4z" />
        <path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07" />
      </svg>
    {/if}
    <span class="lbl">{currentLabel}</span>
  </button>
  {#if open}
    <div class="menu" role="listbox" aria-label={label} bind:this={menu}>
      {#if showOff}
        {@const isOff = selectedLang == null}
        <button
          type="button"
          role="option"
          class="opt"
          class:selected={isOff}
          aria-selected={isOff}
          onclick={() => pick(null)}
        >
          <span class="check">{isOff ? "✓" : ""}</span>
          <span>Off</span>
        </button>
        <div class="divider"></div>
      {/if}
      {#each tracks as t (t.lang + t.path)}
        {@const sel = t.lang === selectedLang}
        <button
          type="button"
          role="option"
          class="opt"
          class:selected={sel}
          aria-selected={sel}
          onclick={() => pick(t.lang)}
        >
          <span class="check">{sel ? "✓" : ""}</span>
          <span>{t.label}</span>
          <span class="fmt">{t.format.toUpperCase()}</span>
        </button>
      {/each}
      {#if settingsHref}
        <div class="divider"></div>
        <a class="opt cfg" href={settingsHref}>
          <span class="check"></span>
          <span>Configurações…</span>
        </a>
      {/if}
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
    display: inline-block;
  }

  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: color-mix(in oklab, var(--content-bg) 80%, var(--accent) 4%);
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 8px;
    color: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: background var(--tg-duration-fast, 150ms);
  }

  .trigger:hover {
    background: color-mix(in oklab, var(--accent) 10%, var(--content-bg) 90%);
  }

  .lbl {
    font-weight: 500;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu {
    position: absolute;
    bottom: calc(100% + 6px);
    right: 0;
    min-width: 200px;
    background: var(--content-bg);
    border: 1px solid color-mix(in oklab, var(--content-border) 80%, transparent);
    border-radius: 10px;
    box-shadow: 0 8px 24px color-mix(in oklab, black 30%, transparent);
    padding: 4px;
    z-index: 60;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    width: 100%;
    text-decoration: none;
  }

  .opt:hover {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }

  .opt.selected {
    color: var(--accent);
    font-weight: 600;
  }

  .check {
    width: 14px;
    color: var(--accent);
    text-align: center;
    flex: 0 0 auto;
  }

  .fmt {
    margin-left: auto;
    font-size: 10px;
    color: color-mix(in oklab, currentColor 50%, transparent);
    font-weight: 500;
    letter-spacing: 0.04em;
  }

  .cfg {
    color: color-mix(in oklab, currentColor 70%, transparent);
    font-size: 12px;
  }

  .divider {
    height: 1px;
    background: color-mix(in oklab, var(--content-border) 50%, transparent);
    margin: 4px 0;
  }
</style>
