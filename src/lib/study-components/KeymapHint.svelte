<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";

  type Ctx = "chat-list" | "media-grid" | "player";

  let { context, onClose }: {
    context: Ctx;
    onClose: () => void;
  } = $props();

  const shortcuts = $derived.by(() => {
    if (context === "chat-list") {
      return [
        { keys: ["Ctrl", "F"], label: $t("study.library.telegram.keymap_search") },
        { keys: ["Esc"], label: $t("study.library.telegram.keymap_clear") },
        { keys: ["↑", "↓"], label: $t("study.library.telegram.keymap_nav") },
        { keys: ["Enter"], label: $t("study.library.telegram.keymap_open") },
        { keys: ["?"], label: $t("study.library.telegram.keymap_help") },
      ];
    }
    if (context === "media-grid") {
      return [
        { keys: ["Ctrl", "F"], label: $t("study.library.telegram.keymap_search") },
        { keys: ["Ctrl", "A"], label: $t("study.library.telegram.keymap_select_all") },
        { keys: ["Esc"], label: $t("study.library.telegram.keymap_clear") },
        { keys: ["Del"], label: $t("study.library.telegram.keymap_delete") },
        { keys: ["Enter"], label: $t("study.library.telegram.keymap_open_player") },
        { keys: ["Space"], label: $t("study.library.telegram.keymap_play_pause") },
        { keys: ["↑", "↓", "←", "→"], label: $t("study.library.telegram.keymap_nav") },
        { keys: ["?"], label: $t("study.library.telegram.keymap_help") },
      ];
    }
    return [
      { keys: ["J", "K"], label: $t("study.library.telegram.keymap_next_prev") },
      { keys: ["Space"], label: $t("study.library.telegram.keymap_play_pause") },
      { keys: ["F"], label: $t("study.library.telegram.keymap_fullscreen") },
      { keys: ["Esc"], label: $t("study.library.telegram.keymap_close") },
      { keys: ["↑", "↓"], label: $t("study.library.telegram.keymap_volume") },
      { keys: ["M"], label: $t("study.library.telegram.keymap_mute") },
      { keys: ["←", "→"], label: $t("study.library.telegram.keymap_seek") },
    ];
  });

  function onKey(ev: KeyboardEvent) {
    if (ev.key === "Escape") {
      onClose();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="keymap-overlay" onclick={onClose} role="presentation"></div>
<div class="keymap-modal" role="dialog" aria-modal="true" aria-label={$t("study.library.telegram.keymap_title")}>
  <header class="keymap-head">
    <h3>{$t("study.library.telegram.keymap_title")}</h3>
    <button type="button" class="keymap-close" onclick={onClose} aria-label={$t("study.common.close")}>✕</button>
  </header>
  <table class="keymap-table">
    <tbody>
      {#each shortcuts as s, i (i)}
        <tr>
          <td class="keys">
            {#each s.keys as k, j (j)}
              <kbd>{k}</kbd>{#if j < s.keys.length - 1}<span class="sep">/</span>{/if}
            {/each}
          </td>
          <td class="label">{s.label}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .keymap-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 1000;
    cursor: pointer;
    animation: fade-in var(--tg-duration-normal, 200ms) var(--tg-easing-default, ease);
  }
  .keymap-modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(90vw, 480px);
    max-height: 80vh;
    background: var(--surface);
    border: 1px solid var(--content-border);
    border-radius: 12px;
    z-index: 1001;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .keymap-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--content-border);
  }
  .keymap-head h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--secondary);
  }
  .keymap-close {
    background: transparent;
    border: 0;
    font-size: 16px;
    color: var(--tertiary);
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
  }
  .keymap-close:hover {
    background: color-mix(in oklab, var(--secondary) 8%, transparent);
  }
  .keymap-table {
    flex: 1;
    border-collapse: collapse;
    width: 100%;
    overflow-y: auto;
  }
  .keymap-table tr {
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
  }
  .keymap-table tr:last-child {
    border-bottom: 0;
  }
  .keymap-table td {
    padding: 10px 16px;
    font-size: 13px;
  }
  .keymap-table td.keys {
    width: 1%;
    white-space: nowrap;
    color: var(--tertiary);
  }
  .keymap-table td.label {
    color: var(--secondary);
  }
  kbd {
    display: inline-block;
    padding: 2px 6px;
    border: 1px solid var(--content-border);
    border-radius: 4px;
    background: color-mix(in oklab, var(--surface) 95%, var(--secondary));
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    color: var(--secondary);
    margin: 0 2px;
  }
  .sep {
    margin: 0 2px;
    color: var(--tertiary);
  }
  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>
