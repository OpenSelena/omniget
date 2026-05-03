<script lang="ts">
  import { t } from "$lib/i18n";
  import { AVAILABLE_READING_THEMES, applyReadingTheme } from "$lib/reader-theme";

  let {
    theme = $bindable<string>(),
    paper = $bindable<boolean>(),
    cursor = $bindable<boolean>(),
    onClose,
  }: {
    theme: string;
    paper: boolean;
    cursor: boolean;
    onClose: () => void;
  } = $props();

  function pick(id: string) {
    theme = id;
    applyReadingTheme(id);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="reader-theme-menu" role="menu" tabindex="-1">
  <div class="section-label">{$t("study.read.theme_label")}</div>
  {#each AVAILABLE_READING_THEMES as opt (opt.id)}
    <button
      type="button"
      class:selected={theme === opt.id}
      onclick={() => pick(opt.id)}
    >
      <span class="swatch" style="background: {opt.swatch};"></span>
      <span class="name">{opt.label}</span>
    </button>
  {/each}
  <div class="section-label">{$t("study.read.effects_label")}</div>
  <button type="button" class:selected={paper} onclick={() => (paper = !paper)}>
    <span>{$t("study.read.effect_paper")}</span>
  </button>
  <button type="button" class:selected={cursor} onclick={() => (cursor = !cursor)}>
    <span>{$t("study.read.effect_cursor")}</span>
  </button>
</div>

<style>
  .reader-theme-menu {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    padding: 6px;
    background: var(--popup-bg, var(--primary));
    border: 1px solid var(--content-border);
    border-radius: 8px;
    box-shadow: 0 8px 24px var(--dialog-backdrop, rgba(0, 0, 0, 0.2));
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 200px;
    z-index: 20;
    max-height: 60vh;
    overflow-y: auto;
  }
  .section-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--tertiary);
    padding: 6px 8px 2px;
  }
  button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 8px;
    background: transparent;
    color: var(--secondary);
    border: 1px solid transparent;
    border-radius: 4px;
    font-size: 12px;
    font-family: inherit;
    text-align: left;
    cursor: pointer;
  }
  button:hover {
    background: var(--sidebar-highlight);
  }
  button.selected {
    border-color: var(--accent);
  }
  button.selected::after {
    content: "✓";
    margin-left: auto;
    color: var(--accent);
    font-weight: 500;
  }
  .swatch {
    display: inline-block;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid var(--content-border);
    flex-shrink: 0;
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
