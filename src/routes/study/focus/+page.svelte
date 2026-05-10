<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { awardXp, bumpCounter } from "$lib/study-gamification";
  import { emitFocusBreakStart } from "$lib/study-focus-bridge";
  import { t } from "$lib/i18n";
  import PageHero from "$lib/study-components/PageHero.svelte";
  import AboutLink from "$lib/study-music-components/AboutLink.svelte";

  type Current = {
    id: number;
    preset: string;
    started_at: string;
    elapsed_minutes: number;
  } | null;

  type Session = {
    id: number;
    preset: string;
    started_at: string;
    ended_at: string | null;
    minutes: number | null;
  };

  type History = {
    sessions: Session[];
    today_minutes: number;
    week_minutes: number;
  };

  const PRESETS: { id: string; labelKey: string; minutes: number }[] = [
    { id: "pomodoro-25", labelKey: "study.focus.preset_pomodoro", minutes: 25 },
    { id: "deep-50", labelKey: "study.focus.preset_deep", minutes: 50 },
    { id: "stopwatch", labelKey: "study.focus.preset_custom", minutes: 0 },
  ];

  type FocusSettings = {
    pomodoro_minutes: number;
    short_break_minutes: number;
    long_break_minutes: number;
    cycles_before_long_break: number;
    notify_end: boolean;
    notify_sound: boolean;
    sound_volume: number;
    auto_pause_player: boolean;
    auto_start_breaks: boolean;
    auto_start_focus: boolean;
  };

  let current = $state<Current>(null);
  let history = $state<History>({ sessions: [], today_minutes: 0, week_minutes: 0 });
  let loading = $state(true);
  let tick = $state(0);
  let poll: number | null = null;

  let settings = $state<FocusSettings>({
    pomodoro_minutes: 25,
    short_break_minutes: 5,
    long_break_minutes: 15,
    cycles_before_long_break: 4,
    notify_end: false,
    notify_sound: true,
    sound_volume: 60,
    auto_pause_player: true,
    auto_start_breaks: false,
    auto_start_focus: false,
  });
  let saveTimer: number | null = null;
  let savingState = $state<"idle" | "saving" | "saved">("idle");

  const activePreset = $derived(
    current ? PRESETS.find((p) => p.id === current!.preset) ?? null : null,
  );

  const elapsedSec = $derived.by(() => {
    void tick;
    if (!current) return 0;
    const startedMs = Date.parse(current.started_at.replace(" ", "T") + "Z");
    const now = Date.now();
    return Math.max(0, Math.floor((now - startedMs) / 1000));
  });

  const remaining = $derived.by(() => {
    void tick;
    if (!current) return null;
    const ap = activePreset;
    if (!ap || ap.minutes === 0) return null;
    const target = ap.minutes * 60;
    return Math.max(0, target - elapsedSec);
  });

  const sessionPct = $derived.by(() => {
    void tick;
    const ap = activePreset;
    if (!ap || ap.minutes === 0 || !current) return 0;
    const target = ap.minutes * 60;
    return Math.min(100, Math.max(0, (elapsedSec / target) * 100));
  });

  const isLowTime = $derived(remaining != null && remaining > 0 && remaining <= 60);
  const isFinished = $derived(remaining === 0);

  let prevFinished = false;
  $effect(() => {
    const finished = isFinished;
    if (finished && !prevFinished && settings.auto_pause_player && current) {
      emitFocusBreakStart({ sessionId: current.id, reason: "session_ended" });
    }
    prevFinished = finished;
  });

  function fmt(sec: number): string {
    const s = Math.floor(sec);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const r = s % 60;
    const pad = (n: number) => String(n).padStart(2, "0");
    if (h > 0) return `${pad(h)}:${pad(m)}:${pad(r)}`;
    return `${pad(m)}:${pad(r)}`;
  }

  async function loadSettings() {
    try {
      const all = await pluginInvoke<{ focus?: Partial<FocusSettings> }>(
        "study",
        "study:settings:get",
      );
      if (all?.focus) {
        settings = { ...settings, ...all.focus };
      }
    } catch (e) {
      console.error("loadSettings failed", e);
    }
  }

  async function saveSettings() {
    if (saveTimer != null) clearTimeout(saveTimer);
    savingState = "saving";
    saveTimer = window.setTimeout(async () => {
      try {
        const all = await pluginInvoke<Record<string, unknown>>(
          "study",
          "study:settings:get",
        );
        const merged = { ...all, focus: settings };
        await pluginInvoke("study", "study:settings:save", { settings: merged });
        savingState = "saved";
        setTimeout(() => (savingState = "idle"), 1500);
      } catch (e) {
        console.error("saveSettings failed", e);
        savingState = "idle";
      }
    }, 350);
  }

  async function refresh() {
    try {
      const [cur, hist] = await Promise.all([
        pluginInvoke<Current>("study", "study:focus:current"),
        pluginInvoke<History>("study", "study:focus:history"),
      ]);
      current = cur;
      history = hist;
    } catch (e) {
      console.error("focus refresh failed", e);
    } finally {
      loading = false;
    }
  }

  async function start(presetId: string) {
    try {
      await pluginInvoke("study", "study:focus:start", { preset: presetId });
      await refresh();
    } catch (e) {
      console.error("focus start failed", e);
    }
  }

  async function stop() {
    try {
      const elapsedMin = current
        ? Math.floor((current.elapsed_minutes ?? 0) + elapsedSec / 60 - (current.elapsed_minutes ?? 0))
        : Math.floor(elapsedSec / 60);
      await pluginInvoke("study", "study:focus:stop");
      await refresh();
      if (elapsedMin >= 1) {
        const xp = elapsedMin;
        void awardXp("focus_session", xp, { minutes: elapsedMin });
        void bumpCounter("focus_minutes", elapsedMin);
      }
    } catch (e) {
      console.error("focus stop failed", e);
    }
  }

  onMount(() => {
    refresh();
    loadSettings();
    loadRpcText();
    poll = window.setInterval(() => {
      tick += 1;
    }, 1000);
  });

  onDestroy(() => {
    if (poll != null) {
      clearInterval(poll);
      poll = null;
    }
  });

  const RPC_TEXT_KEY = "study.focus.rpc_text.v1";

  let rpcCustomText = $state("");

  function loadRpcText() {
    if (typeof localStorage === "undefined") return;
    try {
      rpcCustomText = localStorage.getItem(RPC_TEXT_KEY) ?? "";
    } catch {
      rpcCustomText = "";
    }
  }

  function saveRpcText(value: string) {
    rpcCustomText = value;
    if (typeof localStorage === "undefined") return;
    try {
      if (value.trim()) {
        localStorage.setItem(RPC_TEXT_KEY, value.trim());
      } else {
        localStorage.removeItem(RPC_TEXT_KEY);
      }
    } catch {
      /* ignore */
    }
  }

  function formatDate(iso: string): string {
    try {
      return new Date(iso.replace(" ", "T") + "Z").toLocaleString();
    } catch {
      return iso;
    }
  }

  function presetLabel(id: string): string {
    const p = PRESETS.find((x) => x.id === id);
    return p ? $t(p.labelKey) : id;
  }
</script>

<section
  class="study-page"
  data-mode={current ? "active" : "idle"}
  data-low-time={isLowTime ? "1" : "0"}
  data-finished={isFinished ? "1" : "0"}
>
  {#if !current}
    <PageHero title={$t("study.focus.title")} subtitle={$t("study.focus.placeholder")} />
  {/if}

  {#if loading}
    <p class="muted">{$t("study.common.loading")}</p>
  {:else if current}
    <div class="session">
      <div class="timer-stage">
        <div class="timer-display">
          {#if remaining != null}
            <span class="timer-big">{fmt(remaining)}</span>
          {:else}
            <span class="timer-big">{fmt(elapsedSec)}</span>
          {/if}
        </div>
        <div class="session-meta">
          <span class="status-dot" aria-hidden="true"></span>
          <span class="session-preset">{presetLabel(current.preset)}</span>
        </div>
        {#if rpcCustomText.trim()}
          <div class="session-intent">
            <span class="intent-label">{$t("study.focus.intent_for")}</span>
            <input
              type="text"
              class="intent-input intent-inline"
              value={rpcCustomText}
              oninput={(e) => saveRpcText((e.currentTarget as HTMLInputElement).value)}
              placeholder={$t("study.focus.intent_placeholder") as string}
              maxlength="48"
            />
          </div>
        {/if}
        <div class="session-actions">
          <button type="button" class="btn-stop" onclick={stop}>
            {$t("study.focus.stop")}
          </button>
        </div>
      </div>
      <div class="session-progress" aria-hidden="true">
        <div class="session-progress-fill" style="width: {sessionPct}%"></div>
      </div>
    </div>
  {:else}
    <div class="card-hero-start">
      <span class="hero-eyebrow">{$t("study.focus.start_now")}</span>

      <div class="intent-row">
        <label class="intent-label-block" for="focus-intent">
          {$t("study.focus.intent_label")}
        </label>
        <div class="intent-input-wrap">
          <span class="intent-prefix">{$t("study.focus.intent_prefix")}</span>
          <input
            id="focus-intent"
            type="text"
            class="intent-input"
            value={rpcCustomText}
            oninput={(e) => saveRpcText((e.currentTarget as HTMLInputElement).value)}
            placeholder={$t("study.focus.intent_placeholder") as string}
            autocomplete="off"
            maxlength="48"
          />
          {#if rpcCustomText}
            <button
              type="button"
              class="intent-clear"
              onclick={() => saveRpcText("")}
              aria-label={$t("study.common.clear") as string}
            >×</button>
          {/if}
        </div>
        <p class="intent-hint">{$t("study.focus.intent_hint")}</p>
      </div>

      <div class="presets-row">
        {#each PRESETS as p (p.id)}
          <button type="button" class="preset" onclick={() => start(p.id)}>
            <span class="preset-time">
              {#if p.minutes > 0}
                <span class="preset-num">{p.minutes}</span>
                <span class="preset-unit">min</span>
              {:else}
                <span class="preset-free" aria-label={$t("study.focus.unlimited_aria") as string}>∞</span>
              {/if}
            </span>
            <span class="preset-label">{$t(p.labelKey)}</span>
          </button>
        {/each}
      </div>
      {#if history.today_minutes > 0}
        <div class="today-strip">
          <span class="strip-label">{$t("study.focus.today_progress")}</span>
          <span class="strip-value">{history.today_minutes}min</span>
        </div>
      {/if}
    </div>

    <div class="settings-accordions">
      <span class="settings-status" class:saving={savingState === "saving"} class:saved={savingState === "saved"}>
        {savingState === "saving"
          ? $t("study.focus.settings_saving")
          : savingState === "saved"
            ? $t("study.focus.settings_saved")
            : ""}
      </span>

      <details class="acc">
        <summary>
          <span class="acc-title">{$t("study.focus.acc_cycles_title")}</span>
          <span class="acc-summary">
            {$t("study.focus.acc_cycles_summary", {
              focus: settings.pomodoro_minutes,
              break: settings.short_break_minutes,
              cycles: settings.cycles_before_long_break,
            })}
          </span>
        </summary>
        <div class="acc-body">
          <label class="field">
            <span>{$t("study.focus.acc_cycles_focus")}</span>
            <input
              type="number"
              min="1"
              max="180"
              bind:value={settings.pomodoro_minutes}
              oninput={saveSettings}
            />
          </label>
          <label class="field">
            <span>{$t("study.focus.acc_cycles_short")}</span>
            <input
              type="number"
              min="1"
              max="60"
              bind:value={settings.short_break_minutes}
              oninput={saveSettings}
            />
          </label>
          <label class="field">
            <span>{$t("study.focus.acc_cycles_long")}</span>
            <input
              type="number"
              min="1"
              max="60"
              bind:value={settings.long_break_minutes}
              oninput={saveSettings}
            />
          </label>
          <label class="field">
            <span>{$t("study.focus.acc_cycles_count")}</span>
            <input
              type="number"
              min="2"
              max="10"
              bind:value={settings.cycles_before_long_break}
              oninput={saveSettings}
            />
          </label>
        </div>
      </details>

      <details class="acc">
        <summary>
          <span class="acc-title">{$t("study.focus.acc_breaks_title")}</span>
          <span class="acc-summary">
            {settings.auto_start_breaks
              ? $t("study.focus.acc_breaks_auto_yes")
              : $t("study.focus.acc_breaks_auto_no")} ·
            {settings.auto_start_focus
              ? $t("study.focus.acc_breaks_focus_yes")
              : $t("study.focus.acc_breaks_focus_no")} ·
            {settings.auto_pause_player
              ? $t("study.focus.acc_breaks_player_yes")
              : $t("study.focus.acc_breaks_player_no")}
          </span>
        </summary>
        <div class="acc-body">
          <label class="check">
            <input
              type="checkbox"
              bind:checked={settings.auto_start_breaks}
              onchange={saveSettings}
            />
            <span>{$t("study.focus.acc_breaks_check_breaks")}</span>
          </label>
          <label class="check">
            <input
              type="checkbox"
              bind:checked={settings.auto_start_focus}
              onchange={saveSettings}
            />
            <span>{$t("study.focus.acc_breaks_check_focus")}</span>
          </label>
          <label class="check">
            <input
              type="checkbox"
              bind:checked={settings.auto_pause_player}
              onchange={saveSettings}
            />
            <span>{$t("study.focus.acc_breaks_check_player")}</span>
          </label>
        </div>
      </details>

      <details class="acc">
        <summary>
          <span class="acc-title">{$t("study.focus.acc_sounds_title")}</span>
          <span class="acc-summary">
            {settings.notify_end
              ? $t("study.focus.acc_sounds_notify_on")
              : $t("study.focus.acc_sounds_notify_off")} ·
            {settings.notify_sound
              ? $t("study.focus.acc_sounds_sound_on", { n: settings.sound_volume })
              : $t("study.focus.acc_sounds_sound_off")}
          </span>
        </summary>
        <div class="acc-body">
          <label class="check">
            <input
              type="checkbox"
              bind:checked={settings.notify_end}
              onchange={saveSettings}
            />
            <span>{$t("study.focus.acc_sounds_check_notify")}</span>
          </label>
          <label class="check">
            <input
              type="checkbox"
              bind:checked={settings.notify_sound}
              onchange={saveSettings}
            />
            <span>{$t("study.focus.acc_sounds_check_sound")}</span>
          </label>
          <label class="field">
            <span>{$t("study.focus.acc_sounds_volume", { n: settings.sound_volume })}</span>
            <input
              type="range"
              min="0"
              max="100"
              bind:value={settings.sound_volume}
              oninput={saveSettings}
              disabled={!settings.notify_sound}
            />
          </label>
        </div>
      </details>
    </div>

    {#if history.sessions.length > 0}
      <details class="history-section" open={history.sessions.length <= 5}>
        <summary>
          <span class="summary-label">{$t("study.focus.history")}</span>
          <span class="history-count">{history.sessions.length}</span>
        </summary>
        <ul class="list">
          {#each history.sessions as s (s.id)}
            <li class="row">
              <span class="preset-tag">{presetLabel(s.preset)}</span>
              <span class="minutes">{s.minutes ?? 0}min</span>
              <span class="date">{formatDate(s.started_at)}</span>
            </li>
          {/each}
        </ul>
      </details>
    {/if}
  {/if}

  <footer class="study-footer">
    <AboutLink variant="inline" />
  </footer>
</section>

<style>
  .study-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 1.5);
    width: 100%;
    max-width: 900px;
    margin-inline: auto;
  }
  .study-footer {
    display: flex;
    justify-content: center;
    margin-top: var(--space-5, 20px);
    padding-top: 12px;
    border-top: 1px solid color-mix(in oklab, var(--border) 50%, transparent);
  }

  .study-page[data-mode="active"] {
    min-height: 60vh;
    justify-content: center;
    transition: background-color 10s ease;
  }
  .study-page[data-mode="active"][data-low-time="1"] {
    background: color-mix(in oklab, var(--warning, var(--accent)) 5%, transparent);
  }

  .muted {
    color: var(--tertiary);
    font-size: 14px;
    text-align: center;
  }

  .card-hero-start {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    padding: var(--space-5);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--elev-1);
  }
  .hero-eyebrow {
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .intent-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
    position: relative;
  }
  .intent-label-block {
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 500;
  }
  .intent-input-wrap {
    display: flex;
    align-items: center;
    gap: 0;
    background: var(--input-bg, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 4px 4px 4px 14px;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .intent-input-wrap:focus-within {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 4%, var(--surface));
  }
  .intent-prefix {
    color: var(--text-muted);
    font-size: 14px;
    font-weight: 500;
    padding-right: 6px;
    flex-shrink: 0;
  }
  .intent-input {
    flex: 1;
    border: 0;
    background: transparent;
    color: var(--text);
    font-family: inherit;
    font-size: 14px;
    padding: 8px 0;
    outline: none;
  }
  .intent-input::placeholder {
    color: color-mix(in oklab, var(--text-muted) 70%, transparent);
  }
  .intent-clear {
    width: 26px;
    height: 26px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    color: var(--text-muted);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    flex-shrink: 0;
  }
  .intent-clear:hover {
    background: color-mix(in oklab, var(--text-muted) 12%, transparent);
    color: var(--text);
  }
  .intent-suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--elev-2, 0 8px 24px rgba(0, 0, 0, 0.2));
    z-index: 5;
  }
  .suggestion {
    padding: 5px 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: transparent;
    color: var(--text-muted);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: color 120ms ease, border-color 120ms ease, background 120ms ease;
  }
  .suggestion:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .intent-hint {
    margin: 2px 4px 0;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }
  .session-intent {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    color: var(--text-muted);
    font-size: 13px;
  }
  .intent-label {
    text-transform: lowercase;
  }
  .intent-input.intent-inline {
    border: 0;
    background: transparent;
    padding: 2px 0;
    border-bottom: 1px dashed color-mix(in oklab, var(--text-muted) 40%, transparent);
    color: var(--text);
    font-weight: 600;
    text-align: center;
    min-width: 200px;
  }
  .intent-input.intent-inline:focus {
    border-bottom-color: var(--accent);
  }
  .presets-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: calc(var(--padding) * 1.5);
  }
  .preset {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: calc(var(--padding) * 2) var(--padding);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--secondary);
    cursor: pointer;
    transition: border-color 150ms ease, transform 150ms ease, background 150ms ease;
  }
  .preset:hover {
    border-color: var(--accent);
    transform: translateY(-1px);
  }
  .preset:focus-visible {
    outline: 2px solid var(--focus-ring, var(--accent));
    outline-offset: 2px;
  }
  .preset:active {
    transform: translateY(0);
    background: color-mix(in oklab, var(--accent) 8%, var(--bg));
  }
  .preset-time {
    font-family: var(--font-mono, monospace);
    color: var(--secondary);
    line-height: 1;
    display: inline-flex;
    align-items: baseline;
    gap: 3px;
  }
  .preset-num {
    font-size: 32px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .preset-unit {
    font-size: 11px;
    font-weight: 400;
    color: var(--tertiary);
    text-transform: uppercase;
  }
  .preset-free {
    font-size: 36px;
    color: var(--accent);
    font-weight: 500;
  }
  .preset-label {
    font-size: 12px;
    color: var(--tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .today-strip {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding-top: calc(var(--padding) * 1.5);
    border-top: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    font-size: 13px;
  }
  .strip-label {
    color: var(--tertiary);
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.8px;
  }
  .strip-value {
    color: var(--secondary);
    font-family: var(--font-mono, monospace);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }

  .session {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--padding) * 3);
  }
  .timer-stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--padding) * 2);
  }
  .timer-display {
    padding: calc(var(--padding) * 2) calc(var(--padding) * 4);
  }
  .timer-big {
    font-size: 5rem;
    font-family: var(--font-mono, monospace);
    font-weight: 500;
    color: var(--secondary);
    line-height: 1;
    letter-spacing: -2px;
    font-variant-numeric: tabular-nums;
    transition: color 600ms ease;
  }
  .study-page[data-low-time="1"] .timer-big {
    color: var(--warning, var(--accent));
  }
  .study-page[data-finished="1"] .timer-big {
    animation: pulse 1.2s ease-in-out 2;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .session-meta {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--secondary);
    font-size: 13px;
  }
  .status-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    animation: dot-pulse 2s ease-in-out infinite;
  }
  @keyframes dot-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
  .session-preset {
    text-transform: uppercase;
    font-size: 11px;
    letter-spacing: 0.8px;
    color: var(--tertiary);
  }
  .session-actions {
    display: flex;
    gap: 12px;
  }
  .btn-stop {
    background: transparent;
    border: 1px solid var(--input-border);
    color: var(--secondary);
    border-radius: var(--border-radius);
    padding: 8px 24px;
    font-size: 13px;
    cursor: pointer;
    transition: border-color 150ms ease, color 150ms ease;
  }
  .btn-stop:hover {
    border-color: var(--error);
    color: var(--error);
  }
  .btn-stop:focus-visible {
    outline: 2px solid var(--focus-ring, var(--accent));
    outline-offset: 2px;
  }

  .session-progress {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: color-mix(in oklab, var(--input-border) 50%, transparent);
    z-index: 10;
    pointer-events: none;
  }
  .session-progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 1s linear, background-color 600ms ease;
  }
  .study-page[data-low-time="1"] .session-progress-fill {
    background: var(--warning, var(--accent));
  }

  .history-section {
    margin-top: calc(var(--padding) * 0.5);
  }
  .history-section summary {
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--secondary);
    list-style: none;
  }
  .history-section summary::-webkit-details-marker {
    display: none;
  }
  .history-section summary::before {
    content: "▸";
    color: var(--tertiary);
    transition: transform 150ms ease;
    display: inline-block;
  }
  .history-section[open] summary::before {
    transform: rotate(90deg);
  }
  .summary-label {
    flex: 0 0 auto;
  }
  .history-count {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    color: var(--tertiary);
    padding: 1px 8px;
    background: var(--button-elevated);
    border-radius: 999px;
    font-variant-numeric: tabular-nums;
  }
  .list {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row {
    display: grid;
    grid-template-columns: auto 80px 1fr;
    gap: var(--padding);
    padding: 8px 12px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--button-elevated);
    font-size: 12px;
    color: var(--tertiary);
    align-items: center;
  }
  .preset-tag {
    color: var(--secondary);
    font-weight: 500;
  }
  .minutes {
    font-family: var(--font-mono, monospace);
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }
  .date {
    justify-self: end;
  }

  @media (prefers-reduced-motion: reduce) {
    .study-page[data-mode="active"],
    .timer-big,
    .session-progress-fill,
    .preset,
    .history-section summary::before,
    .btn-stop {
      transition: none;
    }
    .study-page[data-finished="1"] .timer-big,
    .status-dot {
      animation: none;
    }
  }
  .settings-accordions {
    display: flex;
    flex-direction: column;
    gap: 4px;
    position: relative;
    margin-top: calc(var(--padding) * 2);
  }
  .settings-status {
    position: absolute;
    top: -22px;
    right: 0;
    font-size: 11px;
    color: var(--tertiary);
    transition: opacity 200ms ease;
  }
  .settings-status.saving {
    color: var(--tertiary);
  }
  .settings-status.saved {
    color: var(--success, var(--accent));
  }
  .acc {
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    background: var(--surface);
    overflow: hidden;
    transition: border-color 150ms ease;
  }
  .acc[open] {
    border-color: color-mix(in oklab, var(--accent) 40%, var(--input-border));
  }
  .acc summary {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    cursor: pointer;
    list-style: none;
  }
  .acc summary::-webkit-details-marker {
    display: none;
  }
  .acc summary::before {
    content: "›";
    color: var(--tertiary);
    font-size: 16px;
    transition: transform 180ms ease;
    flex-shrink: 0;
  }
  .acc[open] summary::before {
    transform: rotate(90deg);
  }
  .acc summary:hover {
    background: color-mix(in oklab, var(--accent) 5%, transparent);
  }
  .acc-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .acc-summary {
    color: var(--text);
    font-size: var(--text-sm);
    margin-left: auto;
    text-align: right;
  }
  .acc-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 0 16px 16px;
    border-top: 1px solid color-mix(in oklab, var(--input-border) 50%, transparent);
    padding-top: 12px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--tertiary);
  }
  .field input[type="number"] {
    width: 100px;
    padding: 6px 8px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .field input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
  }
  .field input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--secondary);
    cursor: pointer;
  }
  .check input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }
</style>
