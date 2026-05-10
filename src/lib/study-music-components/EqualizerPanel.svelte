<script lang="ts">
  import {
    musicPlayer,
    EQ_BAND_FREQUENCIES,
    EQ_PRESETS,
    type EqPreset,
  } from "$lib/study-music/player-store.svelte";
  import { t } from "$lib/i18n";

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  const PRESETS: EqPreset[] = [
    "flat",
    "bass",
    "vocal",
    "treble",
    "rock",
    "pop",
    "classical",
    "electronic",
    "custom",
  ];

  function fmtFreq(hz: number) {
    return hz >= 1000 ? `${hz / 1000}k` : `${hz}`;
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
        <h3>{$t("study.music.eq_title")}</h3>
        <button
          type="button"
          class="close"
          onclick={onClose}
          aria-label={$t("study.common.close") as string}
        >×</button>
      </header>

      <div class="enable-row">
        <label class="toggle">
          <input
            type="checkbox"
            checked={musicPlayer.eqEnabled}
            onchange={(e) =>
              musicPlayer.setEqEnabled((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>{$t("study.music.eq_enable")}</span>
        </label>
      </div>

      <div class="presets">
        <span class="presets-label">{$t("study.music.eq_preset")}</span>
        <div class="preset-row">
          {#each PRESETS as p (p)}
            <button
              type="button"
              class="preset-btn"
              class:active={musicPlayer.eqPreset === p}
              onclick={() => musicPlayer.setEqPreset(p)}
            >{$t(`study.music.eq_preset_${p}`)}</button>
          {/each}
        </div>
      </div>

      <div class="bands">
        {#each EQ_BAND_FREQUENCIES as freq, i (freq)}
          {@const value = musicPlayer.eqGains[i] ?? 0}
          <div class="band">
            <span class="gain">{value > 0 ? "+" : ""}{value.toFixed(1)} dB</span>
            <input
              type="range"
              orient="vertical"
              min="-12"
              max="12"
              step="0.5"
              value={value}
              disabled={!musicPlayer.eqEnabled}
              oninput={(e) =>
                musicPlayer.setEqGain(i, Number((e.currentTarget as HTMLInputElement).value))}
            />
            <span class="freq">{fmtFreq(freq)} Hz</span>
          </div>
        {/each}
      </div>

      <footer class="foot">
        <button
          type="button"
          class="reset-btn"
          onclick={() => musicPlayer.setEqPreset("flat")}
        >{$t("study.music.eq_reset")}</button>
        <button
          type="button"
          class="done-btn"
          onclick={onClose}
        >{$t("study.common.close")}</button>
      </footer>
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
    width: min(540px, 92vw);
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
  .close:hover { color: white; background: rgba(255, 255, 255, 0.08); }
  .enable-row {
    padding: 8px 20px 0;
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.85);
  }
  .toggle input {
    width: 14px;
    height: 14px;
    accent-color: var(--accent);
  }
  .presets {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 20px 4px;
  }
  .presets-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.5);
  }
  .preset-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .preset-btn {
    padding: 5px 12px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.65);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    text-transform: capitalize;
  }
  .preset-btn:hover {
    color: rgba(255, 255, 255, 0.95);
    border-color: rgba(255, 255, 255, 0.25);
  }
  .preset-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent, white);
  }
  .bands {
    display: flex;
    align-items: stretch;
    justify-content: space-around;
    padding: 18px 20px;
    gap: 8px;
  }
  .band {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    flex: 1;
  }
  .gain {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.65);
    font-variant-numeric: tabular-nums;
    min-height: 16px;
  }
  .band input[type="range"] {
    writing-mode: vertical-lr;
    direction: rtl;
    appearance: slider-vertical;
    -webkit-appearance: slider-vertical;
    width: 18px;
    height: 160px;
    accent-color: var(--accent);
  }
  .freq {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.45);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 14px 20px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.02);
  }
  .reset-btn {
    padding: 8px 14px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.65);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .reset-btn:hover {
    color: rgba(255, 255, 255, 0.95);
    border-color: rgba(255, 255, 255, 0.25);
  }
  .done-btn {
    padding: 8px 16px;
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
