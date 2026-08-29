<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { formatBinding, isMac } from "$lib/platform";
  import { getSettings } from "$lib/stores/settings-store.svelte";
  import {
    getDevices,
    isDevicesLoading,
    refreshDevices,
    setDevice,
    setNoiseSuppression,
    setPttKey,
    setVadThreshold,
    setDucking,
    setRelayOnly,
    setMicTest,
    isMicTesting,
    getMicLevel,
    micLevelStale,
    isMicSilent,
    isPttRegistered,
    refreshPttStatus,
  } from "$lib/stores/omnidisc-voice-store.svelte";

  let settings = $derived(getSettings());
  let voice = $derived(settings?.omnidisc?.voice ?? {});
  let devices = $derived(getDevices());
  let loading = $derived(isDevicesLoading());
  let testing = $derived(isMicTesting());
  let level = $derived(getMicLevel());
  let now = $state(Date.now());
  let stale = $derived(micLevelStale(now));
  let deviceError = $state<string | null>(null);
  let micError = $state<string | null>(null);
  let recording = $state(false);
  let vadDraft = $state<number | null>(null);
  let vadTimer: ReturnType<typeof setTimeout> | null = null;
  let duckDraft = $state<number | null>(null);
  let duckTimer: ReturnType<typeof setTimeout> | null = null;
  let clock: ReturnType<typeof setInterval> | null = null;

  let levelPercent = $derived(Math.round(Math.min(1, Math.max(0, (level.rmsDb + 60) / 60)) * 100));
  let pttLabel = $derived(formatBinding(voice.ptt_key ?? ""));
  let pttRefused = $derived(!!voice.ptt_key && isPttRegistered() === false);
  let vadValue = $derived(vadDraft ?? voice.vad_threshold_db ?? -45);
  let duckValue = $derived(duckDraft ?? voice.ducking_percent ?? 0);
  let relayOnly = $derived(voice.relay_only === true);

  function mapKeyName(key: string): string | null {
    if (key.length === 1 && /[a-zA-Z]/.test(key)) return key.toUpperCase();
    if (key.length === 1 && /[0-9]/.test(key)) return key;
    if (/^F([1-9]|1[0-2])$/.test(key)) return key;
    const map: Record<string, string> = {
      " ": "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
      Enter: "Enter", Tab: "Tab", Backspace: "Backspace", Delete: "Delete",
      Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown", Insert: "Insert",
      "`": "`", "-": "-", "=": "=", "[": "[", "]": "]", "\\": "\\", ";": ";", "'": "'", ",": ",", ".": ".", "/": "/",
    };
    return map[key] ?? null;
  }

  function startRecording(e: MouseEvent) {
    recording = true;
    (e.currentTarget as HTMLButtonElement | null)?.focus();
  }

  function onRecordKey(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      recording = false;
      return;
    }
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
    const keyName = mapKeyName(e.key);
    if (!keyName) return;
    const parts: string[] = [];
    if (e.ctrlKey || e.metaKey) parts.push("CmdOrCtrl");
    if (e.shiftKey) parts.push("Shift");
    if (e.altKey) parts.push("Alt");
    parts.push(keyName);
    recording = false;
    void setPttKey(parts.join("+"));
  }

  async function changeDevice(kind: "input" | "output", e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    deviceError = await setDevice(kind, value === "" ? null : value);
  }

  async function toggleTest() {
    micError = null;
    const err = await setMicTest(!testing);
    if (err) micError = err;
  }

  function onVadInput(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    vadDraft = v;
    if (vadTimer) clearTimeout(vadTimer);
    vadTimer = setTimeout(() => {
      void setVadThreshold(v);
      vadDraft = null;
    }, 500);
  }

  function onDuckInput(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    duckDraft = v;
    if (duckTimer) clearTimeout(duckTimer);
    duckTimer = setTimeout(() => {
      void setDucking(v);
      duckDraft = null;
    }, 400);
  }

  onMount(() => {
    void refreshDevices();
    void refreshPttStatus();
    clock = setInterval(() => (now = Date.now()), 1000);
  });

  onDestroy(() => {
    if (clock) clearInterval(clock);
    if (vadTimer) clearTimeout(vadTimer);
    if (duckTimer) clearTimeout(duckTimer);
    if (testing) void setMicTest(false);
  });
</script>

<div class="voice-settings">
  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <label class="setting-label" for="od-voice-input">{$t("omnidisc.voice.input_device")}</label>
        {#if !loading && devices.inputs.length === 0}
          <span class="setting-path">{$t("omnidisc.voice.no_input_devices")}</span>
        {/if}
      </div>
      <select id="od-voice-input" class="select" value={voice.input_device ?? ""} onchange={(e) => void changeDevice("input", e)} disabled={loading}>
        <option value="">{$t("omnidisc.voice.device_default")}</option>
        {#each devices.inputs as d (d.id)}
          <option value={d.id}>{d.name}</option>
        {/each}
      </select>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <label class="setting-label" for="od-voice-output">{$t("omnidisc.voice.output_device")}</label>
        {#if !loading && devices.outputs.length === 0}
          <span class="setting-path">{$t("omnidisc.voice.no_output_devices")}</span>
        {/if}
      </div>
      <select id="od-voice-output" class="select" value={voice.output_device ?? ""} onchange={(e) => void changeDevice("output", e)} disabled={loading}>
        <option value="">{$t("omnidisc.voice.device_default")}</option>
        {#each devices.outputs as d (d.id)}
          <option value={d.id}>{d.name}</option>
        {/each}
      </select>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t("omnidisc.voice.mic_test")}</span>
        <span class="setting-path">{$t("omnidisc.voice.mic_test_hint")}</span>
        <div class="meter-wrap">
          <div class="meter" role="meter" aria-label={$t("omnidisc.voice.mic_level")} aria-valuemin="0" aria-valuemax="100" aria-valuenow={testing ? levelPercent : 0}>
            <div class="meter-fill" class:hot={levelPercent > 85} style:width={`${testing ? levelPercent : 0}%`}></div>
          </div>
          {#if testing && (stale || isMicSilent())}
            <span class="meter-note">{$t("omnidisc.voice.mic_silent")}</span>
          {/if}
          {#if micError}
            <span class="meter-note bad">{translateBackendError(micError, $t)}</span>
          {/if}
        </div>
      </div>
      <div class="row-actions">
        <button type="button" class="button" class:active={testing} aria-pressed={testing} onclick={toggleTest}>
          {testing ? $t("omnidisc.voice.mic_test_stop") : $t("omnidisc.voice.mic_test")}
        </button>
        <button type="button" class="button ghost" onclick={() => void refreshDevices()} disabled={loading}>{$t("omnidisc.voice.device_refresh")}</button>
      </div>
    </div>
    {#if deviceError}
      <p class="error" role="alert">{translateBackendError(deviceError, $t)}</p>
    {/if}
  </div>

  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t("omnidisc.voice.noise_suppression")}</span>
        <span class="setting-path">{$t("omnidisc.voice.noise_suppression_desc")}</span>
      </div>
      <button
        class="toggle"
        class:on={voice.noise_suppression !== false}
        role="switch"
        aria-checked={voice.noise_suppression !== false}
        aria-label={$t("omnidisc.voice.noise_suppression")}
        onclick={() => void setNoiseSuppression(!(voice.noise_suppression !== false))}
      ><span class="toggle-knob"></span></button>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t("omnidisc.voice.ptt_key")}</span>
        <span class="setting-path">{$t("omnidisc.voice.ptt_key_desc")}</span>
        {#if pttRefused}
          <span class="setting-path refused" role="status">
            {$t("omnidisc.voice.ptt_refused")}
            {isMac() ? $t("omnidisc.voice.ptt_permission_hint") : $t("omnidisc.voice.ptt_conflict_hint")}
          </span>
        {/if}
      </div>
      <div class="row-actions">
        <button
          type="button"
          class="button key"
          class:recording
          onclick={startRecording}
          onkeydown={onRecordKey}
          onblur={() => (recording = false)}
          aria-label={$t("omnidisc.voice.ptt_key")}
        >
          {#if recording}{$t("omnidisc.voice.ptt_recording")}{:else if pttLabel}{pttLabel}{:else}{$t("omnidisc.voice.ptt_record")}{/if}
        </button>
        {#if voice.ptt_key}
          <button type="button" class="button ghost" onclick={() => void setPttKey("")}>{$t("omnidisc.voice.ptt_clear")}</button>
        {/if}
      </div>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <label class="setting-label" for="od-voice-vad">{$t("omnidisc.voice.vad_threshold")}</label>
        <span class="setting-path">{$t("omnidisc.voice.vad_threshold_desc")}</span>
      </div>
      <div class="vad">
        <input id="od-voice-vad" type="range" min="-70" max="-20" step="1" value={vadValue} oninput={onVadInput} />
        <span class="vad-value">{vadValue} dB</span>
      </div>
    </div>
    <div class="divider"></div>
    <div class="setting-row">
      <div class="setting-col">
        <label class="setting-label" for="od-voice-duck">{$t("omnidisc.voice.ducking")}</label>
        <span class="setting-path">{$t("omnidisc.voice.ducking_desc")}</span>
      </div>
      <div class="vad">
        <input id="od-voice-duck" type="range" min="0" max="100" step="5" value={duckValue} oninput={onDuckInput} />
        <span class="vad-value">
          {duckValue === 0 ? $t("omnidisc.voice.ducking_off") : $t("omnidisc.voice.ducking_value", { percent: duckValue })}
        </span>
      </div>
    </div>
  </div>

  <div class="card">
    <div class="setting-row">
      <div class="setting-col">
        <span class="setting-label">{$t("omnidisc.voice.relay_only")}</span>
        <span class="setting-path">{$t("omnidisc.voice.relay_only_desc")}</span>
      </div>
      <button
        class="toggle"
        class:on={relayOnly}
        role="switch"
        aria-checked={relayOnly}
        aria-label={$t("omnidisc.voice.relay_only")}
        onclick={() => void setRelayOnly(!relayOnly)}
      ><span class="toggle-knob"></span></button>
    </div>
  </div>
</div>

<style>
  .voice-settings {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .select {
    max-width: 240px;
    min-width: 160px;
  }

  .row-actions {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: var(--space-2);
  }

  .setting-path.refused {
    color: var(--warning);
    white-space: normal;
    overflow: visible;
    text-overflow: clip;
    line-height: 1.45;
    max-width: 52ch;
    margin-top: var(--space-1);
  }

  .button.active {
    background: var(--accent);
    color: var(--on-accent);
    border-color: var(--accent);
  }

  .button.ghost {
    background: transparent;
  }

  .button.key {
    min-width: 140px;
    font-variant-numeric: tabular-nums;
  }

  .button.key.recording {
    border-color: var(--accent);
    color: var(--accent);
  }

  .meter-wrap {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-top: var(--space-2);
    max-width: 320px;
  }

  .meter {
    height: 8px;
    border-radius: var(--radius-full);
    background: var(--fill-2);
    overflow: hidden;
  }

  .meter-fill {
    height: 100%;
    width: 0;
    background: var(--success);
    border-radius: var(--radius-full);
    transition: width 80ms linear;
  }

  .meter-fill.hot {
    background: var(--warning);
  }

  .meter-note {
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.45;
  }

  .meter-note.bad {
    color: var(--danger);
  }

  .error {
    margin: var(--space-2) 0 0;
    font-size: var(--text-xs);
    color: var(--danger);
  }

  .vad {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .vad input {
    width: 160px;
  }

  .vad-value {
    min-width: 72px;
    text-align: right;
    font-size: var(--text-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  @media (prefers-reduced-motion: reduce) {
    .meter-fill {
      transition: none;
    }
  }
</style>
