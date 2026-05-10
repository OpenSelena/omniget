<script lang="ts">
  import { musicTheme, PRESET_ACCENTS, type MusicThemePreset } from "$lib/study-music/theme-store.svelte";
  import { t } from "$lib/i18n";

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  const PRESETS: MusicThemePreset[] = [
    "default",
    "indigo",
    "spotify",
    "purple",
    "amber",
    "custom",
  ];

  function previewColor(preset: MusicThemePreset): string {
    if (preset === "custom") return musicTheme.accent || "#888";
    if (preset === "default") return "var(--accent)";
    return PRESET_ACCENTS[preset as Exclude<MusicThemePreset, "custom">];
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }
</script>

{#if open}
  <div
    class="overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    onkeydown={onKey}
  >
    <div class="dialog" role="dialog" aria-modal="true" tabindex="-1">
      <header class="head">
        <h3>{$t("study.music.music_theme_title")}</h3>
        <button type="button" class="close" onclick={onClose} aria-label={$t("study.common.close") as string}>×</button>
      </header>
      <div class="body">
        <p class="hint">{$t("study.music.music_theme_subtitle")}</p>

        <section class="row">
          <span class="label">{$t("study.music.music_theme_preset")}</span>
          <div class="presets">
            {#each PRESETS as p (p)}
              <button
                type="button"
                class="preset"
                class:active={musicTheme.preset === p}
                onclick={() => musicTheme.setPreset(p)}
              >
                <span class="swatch" style:background={previewColor(p)}></span>
                <span class="preset-name">{$t(`study.music.music_theme_preset_${p}`)}</span>
              </button>
            {/each}
          </div>
        </section>

        {#if musicTheme.preset === "custom" || musicTheme.preset !== "default"}
          <section class="row">
            <span class="label">{$t("study.music.music_theme_accent")}</span>
            <div class="color-row">
              <input
                type="color"
                class="picker"
                value={musicTheme.accent || PRESET_ACCENTS.indigo}
                oninput={(e) => musicTheme.setAccent((e.currentTarget as HTMLInputElement).value)}
              />
              <input
                type="text"
                class="hex"
                value={musicTheme.accent}
                oninput={(e) => {
                  const v = (e.currentTarget as HTMLInputElement).value;
                  if (/^#[0-9a-f]{6}$/i.test(v)) musicTheme.setAccent(v);
                }}
                placeholder="#6366f1"
              />
            </div>
          </section>
        {/if}

        <section class="row">
          <span class="label">{$t("study.music.music_theme_use_dominant")}</span>
          <div class="dom-toggle">
            <button
              type="button"
              class="seg"
              class:active={!musicTheme.useDominant}
              onclick={() => { if (musicTheme.useDominant) musicTheme.toggleUseDominant(); }}
            >{$t("study.music.music_theme_dominant_off")}</button>
            <button
              type="button"
              class="seg"
              class:active={musicTheme.useDominant}
              onclick={() => { if (!musicTheme.useDominant) musicTheme.toggleUseDominant(); }}
            >{$t("study.music.music_theme_dominant_on")}</button>
          </div>
        </section>

        <div class="actions">
          <button type="button" class="reset" onclick={() => musicTheme.reset()}>
            {$t("study.music.music_theme_reset")}
          </button>
          <button type="button" class="done" onclick={onClose}>
            {$t("study.common.close")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    z-index: 320;
    display: grid;
    place-items: center;
    backdrop-filter: blur(4px);
  }
  .dialog {
    background: rgb(20, 20, 20);
    color: rgba(255, 255, 255, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    width: min(560px, 92vw);
    max-height: 86vh;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .head {
    position: relative;
    padding: 18px 20px 8px;
  }
  .head h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 800;
  }
  .close {
    position: absolute;
    top: 12px;
    right: 14px;
    width: 28px;
    height: 28px;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: rgba(255, 255, 255, 0.5);
    font-size: 18px;
    cursor: pointer;
  }
  .body {
    padding: 8px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  .hint {
    margin: 0;
    color: rgba(255, 255, 255, 0.55);
    font-size: 13px;
    line-height: 1.5;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.5);
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 6px;
  }
  .preset {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.85);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .preset:hover { background: rgba(255, 255, 255, 0.08); }
  .preset.active {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 14%, transparent);
  }
  .swatch {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.4);
  }
  .preset-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .color-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .picker {
    width: 48px;
    height: 36px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    background: transparent;
    cursor: pointer;
  }
  .hex {
    flex: 1;
    padding: 8px 10px;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    color: rgba(255, 255, 255, 0.95);
    font-family: ui-monospace, monospace;
    font-size: 12px;
    outline: none;
  }
  .hex:focus { border-color: var(--accent); }
  .dom-toggle {
    display: inline-flex;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 999px;
    padding: 3px;
    align-self: flex-start;
  }
  .seg {
    padding: 6px 14px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.6);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .seg.active {
    background: var(--accent);
    color: var(--on-accent, white);
  }
  .actions {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }
  .reset {
    padding: 8px 14px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.65);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .reset:hover {
    color: rgba(255, 255, 255, 0.95);
    border-color: rgba(255, 255, 255, 0.25);
  }
  .done {
    padding: 8px 18px;
    border: 0;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent, white);
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }
</style>
