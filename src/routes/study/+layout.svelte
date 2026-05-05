<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { t } from "$lib/i18n";
  import {
    onGamificationToast,
    type GamificationToast,
  } from "$lib/study-gamification";
  import { emitFocusBreakStart } from "$lib/study-focus-bridge";
  import { STUDY_NOTES_ENABLED, STUDY_ANKI_ENABLED } from "$lib/study-feature-flags";

  const SUBNAV_FULL: { href: string; labelKey: string; icon: string; match: string }[] = [
    { href: "/study", labelKey: "study.hub.title", icon: "M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z M9 22V12h6v10", match: "exact" },
    { href: "/study/library", labelKey: "study.hub.library", icon: "M4 6h16M4 12h16M4 18h16", match: "prefix" },
    { href: "/study/anki", labelKey: "study.hub.anki", icon: "M4 5h16v14H4z M4 9h16", match: "prefix" },
    { href: "/study/focus", labelKey: "study.hub.focus", icon: "M12 6v6l4 2 M12 22a10 10 0 1 1 0-20 10 10 0 0 1 0 20z", match: "prefix" },
    { href: "/study/progress", labelKey: "study.hub.progress", icon: "M3 17l6-6 4 4 8-8", match: "prefix" },
    { href: "/study/achievements", labelKey: "study.hub.achievements", icon: "M8 21h8 M12 17v4 M7 4h10v6a5 5 0 0 1-10 0V4z M3 5v3a4 4 0 0 0 4 4 M21 5v3a4 4 0 0 1-4 4", match: "prefix" },
    { href: "/study/notes", labelKey: "study.hub.notes", icon: "M4 4h12l4 4v12H4z M14 4v5h5", match: "prefix" },
    { href: "/study/read", labelKey: "study.hub.read", icon: "M4 4h6a3 3 0 0 1 3 3v13a2 2 0 0 0-2-2H4z M20 4h-6a3 3 0 0 0-3 3v13a2 2 0 0 1 2-2h7z", match: "prefix" },
    { href: "/study/settings", labelKey: "study.hub.settings", icon: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M19 12a7 7 0 1 1-14 0 7 7 0 0 1 14 0z", match: "prefix" },
  ];

  const SUBNAV = SUBNAV_FULL.filter((item) => {
    if (item.href === "/study/notes" && !STUDY_NOTES_ENABLED) return false;
    if (item.href === "/study/anki" && !STUDY_ANKI_ENABLED) return false;
    return true;
  });

  function isActive(item: (typeof SUBNAV)[number], path: string): boolean {
    if (item.match === "exact") return path === item.href;
    return path === item.href || path.startsWith(item.href + "/");
  }

  type SearchResult = {
    courses: { id: number; title: string }[];
    lessons: { id: number; title: string; course_id: number; course_title: string }[];
    notes: {
      id: number;
      uuid: string;
      page_id: number;
      page_name: string;
      body: string;
      snippet: string;
      updated_at: number;
    }[];
  };

  let { children } = $props();

  let unlisteners: UnlistenFn[] = [];
  let rescanScheduled = false;

  let paletteOpen = $state(false);
  let term = $state("");
  let results = $state<SearchResult>({
    courses: [],
    lessons: [],
    notes: [],
  });
  let searchTimer: number | null = null;
  let searching = $state(false);
  let paletteInput = $state<HTMLInputElement | null>(null);

  type XpState = {
    xp: number;
    level: number;
    xp_to_next: number;
    level_progress_pct: number;
    updated_at: number;
  };
  let xpState = $state<XpState | null>(null);

  async function refreshXpState() {
    try {
      xpState = await pluginInvoke<XpState>("study", "study:gamification:state");
    } catch (e) {
      console.error("xp state fetch failed", e);
    }
  }

  const PRESET_MINUTES: Record<string, number> = {
    "pomodoro-25": 25,
    "deep-50": 50,
    "stopwatch": 0,
  };

  type FocusCurrent = {
    id: number;
    preset: string;
    started_at: string;
    elapsed_minutes: number;
  } | null;

  let focusPoll: number | null = null;
  let lastFiredSessionId: number | null = null;
  let autoPausePlayer = true;

  async function refreshAutoPauseSetting() {
    try {
      const all = await pluginInvoke<{
        focus?: { auto_pause_player?: boolean };
      }>("study", "study:settings:get");
      autoPausePlayer = all?.focus?.auto_pause_player ?? true;
    } catch {}
  }

  async function pollFocusForBreak() {
    try {
      const cur = await pluginInvoke<FocusCurrent>("study", "study:focus:current");
      if (!cur) return;
      const target = PRESET_MINUTES[cur.preset] ?? 0;
      if (target <= 0) return;
      const startedMs = Date.parse(cur.started_at.replace(" ", "T") + "Z");
      const elapsedSec = Math.max(0, Math.floor((Date.now() - startedMs) / 1000));
      if (elapsedSec >= target * 60 && lastFiredSessionId !== cur.id) {
        lastFiredSessionId = cur.id;
        if (autoPausePlayer) {
          emitFocusBreakStart({ sessionId: cur.id, reason: "session_ended" });
        }
      }
    } catch {}
  }

  const totalResults = $derived(
    results.courses.length + results.lessons.length + results.notes.length,
  );

  type PaletteTab = "search" | "jump" | "action";
  let paletteTab = $state<PaletteTab>("search");

  type JumpPage = {
    id: number;
    name: string;
    title: string | null;
    updated_at: number;
  };
  let jumpRecents = $state<JumpPage[]>([]);

  const RECENT_ACTIONS_KEY = "study:palette:recent_actions";
  const MAX_RECENT_ACTIONS = 8;

  function loadRecentActions(): string[] {
    try {
      const raw = localStorage.getItem(RECENT_ACTIONS_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed) ? parsed.filter((s) => typeof s === "string") : [];
    } catch {
      return [];
    }
  }

  function saveRecentAction(id: string) {
    try {
      const current = loadRecentActions().filter((s) => s !== id);
      current.unshift(id);
      const trimmed = current.slice(0, MAX_RECENT_ACTIONS);
      localStorage.setItem(RECENT_ACTIONS_KEY, JSON.stringify(trimmed));
      recentActions = trimmed;
    } catch {}
  }

  let recentActions = $state<string[]>([]);

  type PaletteAction = {
    id: string;
    label: string;
    hint: string;
    icon: string;
    run: () => void | Promise<void>;
  };

  async function loadJumpRecents() {
    if (!STUDY_NOTES_ENABLED) return;
    try {
      const pages = await (
        await import("$lib/notes-bridge")
      ).notesPagesList();
      jumpRecents = pages
        .slice()
        .sort((a, b) => b.updated_at - a.updated_at)
        .slice(0, 10)
        .map((p) => ({
          id: p.id,
          name: p.name,
          title: p.title,
          updated_at: p.updated_at,
        }));
    } catch (e) {
      console.error("loadJumpRecents failed", e);
    }
  }

  async function actionRebuildFts() {
    try {
      await pluginInvoke("study", "study:notes:search:rebuild");
    } catch (e) {
      console.error("rebuild fts failed", e);
    }
  }

  async function actionVacuum() {
    try {
      await pluginInvoke("study", "study:library:vacuum");
    } catch (e) {
      console.error("vacuum failed", e);
    }
  }

  async function actionCreatePage() {
    const name = window.prompt("Nome da nova página:");
    if (!name || !name.trim()) return;
    try {
      const r = await (
        await import("$lib/notes-bridge")
      ).notesPagesCreate({ name: name.trim() });
      void goto(`/study/notes?page=${encodeURIComponent(name.trim())}`);
      void r;
    } catch (e) {
      console.error("create page failed", e);
    }
  }

  async function actionExportGraph() {
    try {
      const { notesExportGraphJson } = await import("$lib/notes-bridge");
      const r = await notesExportGraphJson();
      const blob = new Blob([r.json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `notes-graph-${Date.now()}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e) {
      console.error("export graph failed", e);
    }
  }

  function makeActions(): PaletteAction[] {
    const notesActions: PaletteAction[] = STUDY_NOTES_ENABLED
      ? [
          {
            id: "create-page",
            label: $t("study.palette.action_create_page_label"),
            hint: $t("study.palette.action_create_page_hint"),
            icon: "+",
            run: () => {
              closePalette();
              void actionCreatePage();
            },
          },
          {
            id: "open-daily",
            label: $t("study.palette.action_open_daily_label"),
            hint: "Ctrl+J",
            icon: "📅",
            run: () => {
              closePalette();
              void openJournalToday();
            },
          },
          {
            id: "open-templates",
            label: $t("study.palette.action_open_templates_label"),
            hint: "/study/notes/templates",
            icon: "▤",
            run: () => {
              closePalette();
              void goto("/study/notes/templates");
            },
          },
          {
            id: "open-graph",
            label: $t("study.palette.action_open_graph_label"),
            hint: $t("study.palette.action_open_graph_hint"),
            icon: "◈",
            run: () => {
              closePalette();
              void goto("/study/notes/graph");
            },
          },
          {
            id: "rebuild-fts",
            label: $t("study.palette.action_rebuild_fts_label"),
            hint: $t("study.palette.action_rebuild_fts_hint"),
            icon: "↻",
            run: () => {
              closePalette();
              void actionRebuildFts();
            },
          },
          {
            id: "export-graph",
            label: $t("study.palette.action_export_graph_label"),
            hint: $t("study.palette.action_export_graph_hint"),
            icon: "↓",
            run: () => {
              closePalette();
              void actionExportGraph();
            },
          },
        ]
      : [];

    return [
      ...notesActions,
      {
        id: "open-settings",
        label: $t("study.palette.action_open_settings_label"),
        hint: "/study/settings",
        icon: "⚙",
        run: () => {
          closePalette();
          void goto("/study/settings");
        },
      },
      {
        id: "vacuum",
        label: $t("study.palette.action_vacuum_label"),
        hint: $t("study.palette.action_vacuum_hint"),
        icon: "▽",
        run: () => {
          closePalette();
          void actionVacuum();
        },
      },
    ];
  }

  const paletteActions = $derived(makeActions());
  const sortedActions = $derived.by(() => {
    const recent = recentActions;
    const all = paletteActions;
    const byId = new Map(all.map((a) => [a.id, a]));
    const ordered: PaletteAction[] = [];
    for (const id of recent) {
      const a = byId.get(id);
      if (a) {
        ordered.push(a);
        byId.delete(id);
      }
    }
    for (const a of byId.values()) ordered.push(a);
    return ordered;
  });

  function runAction(action: PaletteAction) {
    saveRecentAction(action.id);
    void action.run();
  }

  function scheduleRescan() {
    if (rescanScheduled) return;
    rescanScheduled = true;
    setTimeout(async () => {
      rescanScheduled = false;
      try {
        await pluginInvoke("study", "study:rescan");
      } catch (e) {
        console.error("study rescan failed", e);
      }
    }, 1500);
  }

  function openPalette() {
    paletteOpen = true;
    paletteTab = "search";
    recentActions = loadRecentActions();
    void refreshXpState();
    void loadJumpRecents();
    tick().then(() => paletteInput?.focus());
  }

  function closePalette() {
    paletteOpen = false;
    term = "";
    results = { courses: [], lessons: [], notes: [] };
  }

  async function openJournalToday() {
    if (!STUDY_NOTES_ENABLED) return;
    try {
      const r = await pluginInvoke<{ journal_day: number }>(
        "study",
        "study:notes:journal:today",
      );
      void goto(`/study/notes/journal?day=${r.journal_day}`);
    } catch (e) {
      console.error("openJournalToday failed", e);
    }
  }

  function onGlobalKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && (e.key === "k" || e.key === "K")) {
      e.preventDefault();
      paletteOpen ? closePalette() : openPalette();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && (e.key === "j" || e.key === "J") && !e.shiftKey) {
      e.preventDefault();
      void openJournalToday();
      return;
    }
    if (paletteOpen && e.key === "Escape") {
      e.preventDefault();
      closePalette();
    }
  }

  function runSearch() {
    if (searchTimer != null) clearTimeout(searchTimer);
    const q = term.trim();
    if (!q) {
      results = { courses: [], lessons: [], notes: [] };
      searching = false;
      return;
    }
    searching = true;
    searchTimer = window.setTimeout(async () => {
      try {
        const r = await pluginInvoke<SearchResult>("study", "study:search", {
          term: q,
        });
        results = r;
      } catch (e) {
        console.error("search failed", e);
      } finally {
        searching = false;
      }
    }, 200);
  }

  $effect(() => {
    void term;
    runSearch();
  });

  function goCourse(id: number) {
    closePalette();
    goto(`/study/course/${id}`);
  }
  function goLesson(courseId: number, lessonId: number) {
    closePalette();
    goto(`/study/course/${courseId}/lesson/${lessonId}`);
  }

  let gamificationToasts = $state<{ id: number; toast: GamificationToast }[]>([]);
  let nextToastId = 0;

  function pushToast(toast: GamificationToast) {
    const id = ++nextToastId;
    gamificationToasts = [...gamificationToasts, { id, toast }];
    setTimeout(() => {
      gamificationToasts = gamificationToasts.filter((t) => t.id !== id);
    }, 4000);
    if (toast.kind === "level_up" && paletteOpen) {
      void refreshXpState();
    }
  }

  onMount(() => {
    const events = [
      "download-complete",
      "udemy-download-complete",
      "thinkific-download-complete",
      "teachable-download-complete",
      "generic-download-complete",
    ];
    Promise.all(
      events.map((name) => listen(name, () => scheduleRescan())),
    ).then((fns) => {
      unlisteners = fns;
    });
    window.addEventListener("keydown", onGlobalKey);
    const unsubGamif = onGamificationToast(pushToast);
    void refreshAutoPauseSetting();
    void pollFocusForBreak();
    focusPoll = window.setInterval(() => {
      void pollFocusForBreak();
    }, 10_000);
    return () => {
      for (const fn of unlisteners) fn();
      unlisteners = [];
      window.removeEventListener("keydown", onGlobalKey);
      unsubGamif();
      if (focusPoll != null) {
        clearInterval(focusPoll);
        focusPoll = null;
      }
    };
  });

  onDestroy(() => {
    if (searchTimer != null) clearTimeout(searchTimer);
  });
</script>

<nav class="subnav" aria-label="study sections">
  {#each SUBNAV as item (item.href)}
    {@const active = isActive(item, $page.url.pathname)}
    <a
      href={item.href}
      class="subnav-tab"
      class:active
      aria-current={active ? "page" : undefined}
    >
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d={item.icon}></path>
      </svg>
      <span>{$t(item.labelKey)}</span>
    </a>
  {/each}
</nav>

{@render children()}

{#if gamificationToasts.length > 0}
  <div class="gamif-stack" role="status" aria-live="polite">
    {#each gamificationToasts as item (item.id)}
      <div class="gamif-toast" data-kind={item.toast.kind}>
        <span class="gamif-icon">
          {item.toast.kind === "level_up" ? "⬆️" : "🏆"}
        </span>
        <span class="gamif-text">{item.toast.text}</span>
      </div>
    {/each}
  </div>
{/if}

{#if paletteOpen}
  <div
    class="palette-backdrop"
    role="presentation"
    onclick={closePalette}
    onkeydown={(e) => { if (e.key === "Escape") closePalette(); }}
  >
    <div
      class="palette"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => { if (e.key === "Escape") { e.stopPropagation(); closePalette(); } }}
    >
      <div class="palette-tabs" role="tablist" aria-label={$t("study.palette.aria_modes")}>
        <button
          type="button"
          class="palette-tab"
          class:active={paletteTab === "search"}
          role="tab"
          aria-selected={paletteTab === "search"}
          onclick={() => (paletteTab = "search")}
        >{$t("study.palette.tab_search")}</button>
        {#if STUDY_NOTES_ENABLED}
          <button
            type="button"
            class="palette-tab"
            class:active={paletteTab === "jump"}
            role="tab"
            aria-selected={paletteTab === "jump"}
            onclick={() => (paletteTab = "jump")}
          >{$t("study.palette.tab_jump")}</button>
        {/if}
        <button
          type="button"
          class="palette-tab"
          class:active={paletteTab === "action"}
          role="tab"
          aria-selected={paletteTab === "action"}
          onclick={() => (paletteTab = "action")}
        >{$t("study.palette.tab_action")}</button>
      </div>

      {#if paletteTab === "search"}
        <input
          bind:this={paletteInput}
          type="search"
          bind:value={term}
          placeholder={$t("study.search.placeholder")}
          autocomplete="off"
        />
        <div class="palette-body">
          {#if term.trim() === ""}
            <p class="muted">{$t("study.search.hint")}</p>
          {:else if searching}
            <p class="muted">{$t("study.common.loading")}</p>
          {:else if totalResults === 0}
            <p class="muted">{$t("study.search.no_results", { term })}</p>
          {:else}
            {#if results.courses.length > 0}
              <section>
                <h4>{$t("study.search.courses")}</h4>
                <ul>
                  {#each results.courses as c (c.id)}
                    <li>
                      <button type="button" onclick={() => goCourse(c.id)}>
                        📚 {c.title}
                      </button>
                    </li>
                  {/each}
                </ul>
              </section>
            {/if}
            {#if results.lessons.length > 0}
              <section>
                <h4>{$t("study.search.lessons")}</h4>
                <ul>
                  {#each results.lessons as l (l.id)}
                    <li>
                      <button
                        type="button"
                        onclick={() => goLesson(l.course_id, l.id)}
                      >
                        ▶ {l.title}
                        <small>{l.course_title}</small>
                      </button>
                    </li>
                  {/each}
                </ul>
              </section>
            {/if}
            {#if STUDY_NOTES_ENABLED && results.notes.length > 0}
              <section>
                <h4>{$t("study.search.notes")}</h4>
                <ul>
                  {#each results.notes as n (n.id)}
                    <li>
                      <button
                        type="button"
                        onclick={() => {
                          closePalette();
                          goto(
                            `/study/notes?page=${encodeURIComponent(n.page_name)}`,
                          );
                        }}
                      >
                        ✏ {n.snippet || n.body.slice(0, 80)}
                        <small>{n.page_name}</small>
                      </button>
                    </li>
                  {/each}
                </ul>
              </section>
            {/if}
          {/if}
        </div>
      {:else if paletteTab === "jump"}
        <div class="palette-body">
          <section>
            <h4>{$t("study.palette.heading_shortcuts")}</h4>
            <ul>
              <li>
                <button
                  type="button"
                  onclick={() => {
                    closePalette();
                    void openJournalToday();
                  }}
                >
                  📅 {$t("study.palette.shortcut_daily")}
                  <small>Ctrl+J</small>
                </button>
              </li>
              <li>
                <button
                  type="button"
                  onclick={() => {
                    closePalette();
                    void goto("/study/notes/journal");
                  }}
                >
                  📆 {$t("study.palette.shortcut_journal_history")}
                </button>
              </li>
              <li>
                <button
                  type="button"
                  onclick={() => {
                    closePalette();
                    void goto("/study/notes/graph");
                  }}
                >
                  ◈ {$t("study.palette.shortcut_graph")}
                </button>
              </li>
              <li>
                <button
                  type="button"
                  onclick={() => {
                    closePalette();
                    void goto("/study/notes/templates");
                  }}
                >
                  ▤ {$t("study.palette.shortcut_templates")}
                </button>
              </li>
            </ul>
          </section>
          <section>
            <h4>{$t("study.palette.heading_recent_pages")}</h4>
            {#if jumpRecents.length === 0}
              <p class="muted">{$t("study.palette.empty_pages")}</p>
            {:else}
              <ul>
                {#each jumpRecents as p (p.id)}
                  <li>
                    <button
                      type="button"
                      onclick={() => {
                        closePalette();
                        void goto(`/study/notes?page=${encodeURIComponent(p.name)}`);
                      }}
                    >
                      ✏ {p.title ?? p.name}
                      <small>{p.name}</small>
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </section>
        </div>
      {:else}
        <div class="palette-body">
          <section>
            <h4>{$t("study.palette.heading_actions")}</h4>
            <ul>
              {#each sortedActions as a (a.id)}
                <li>
                  <button type="button" onclick={() => runAction(a)}>
                    <span class="palette-action-icon">{a.icon}</span>
                    {a.label}
                    <small>{a.hint}</small>
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        </div>
      {/if}
      {#if xpState}
        <footer class="palette-footer">
          <span class="palette-level">L{xpState.level}</span>
          <div
            class="palette-bar"
            role="progressbar"
            aria-valuenow={xpState.level_progress_pct}
            aria-valuemin="0"
            aria-valuemax="100"
            aria-label={$t("study.palette.aria_xp_progress", { pct: xpState.level_progress_pct, level: xpState.level + 1 })}
          >
            <div
              class="palette-bar-fill"
              style="width: {xpState.level_progress_pct}%"
            ></div>
          </div>
          <span class="palette-hint"
            >{xpState.level_progress_pct}% até L{xpState.level + 1} ·
            {xpState.xp_to_next} XP</span
          >
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 1000;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 80px;
  }
  .palette {
    width: min(600px, 90vw);
    max-height: 70vh;
    background: var(--button-elevated);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.35);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .palette input {
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--input-border);
    color: var(--secondary);
    padding: 14px 16px;
    font-size: 15px;
    outline: none;
  }
  .palette-tabs {
    display: flex;
    gap: 4px;
    padding: 6px 6px 0;
    border-bottom: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
  }
  .palette-tab {
    padding: 6px 12px;
    border: 0;
    border-radius: var(--border-radius) var(--border-radius) 0 0;
    background: transparent;
    color: var(--tertiary);
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 120ms ease, border-color 120ms ease;
  }
  .palette-tab:hover {
    color: var(--text);
  }
  .palette-tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .palette-action-icon {
    display: inline-block;
    width: 18px;
    text-align: center;
    margin-right: 6px;
    color: var(--accent);
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .palette-body {
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .palette-body section h4 {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--tertiary);
    margin: 6px 10px 4px;
  }
  .palette-body ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .palette-body li button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: none;
    padding: 8px 10px;
    border-radius: 6px;
    text-align: left;
    color: var(--secondary);
    font-size: 13px;
    cursor: pointer;
  }
  .palette-body li button:hover {
    background: var(--sidebar-highlight);
  }
  .palette-body li button small {
    color: var(--tertiary);
    font-size: 11px;
    margin-left: auto;
  }
  .muted {
    color: var(--tertiary);
    font-size: 13px;
    padding: 10px;
    text-align: center;
  }

  .palette-footer {
    display: flex;
    align-items: center;
    gap: 10px;
    border-top: 1px solid var(--input-border);
    padding: 8px 14px;
    font-size: 11px;
    color: var(--tertiary);
  }
  .palette-level {
    font-weight: 600;
    color: var(--secondary);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .palette-bar {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--input-border);
    overflow: hidden;
  }
  .palette-bar-fill {
    height: 100%;
    background: var(--accent, #4a9eff);
    transition: width 0.2s ease;
  }
  @media (prefers-reduced-motion: reduce) {
    .palette-bar-fill {
      transition: none;
    }
  }
  .palette-hint {
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .subnav {
    display: flex;
    gap: 0.25rem;
    padding: 0.5rem 0 1rem;
    margin: 0 auto;
    max-width: 960px;
    width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
    margin-bottom: calc(var(--padding) * 2);
  }
  .subnav::-webkit-scrollbar {
    display: none;
  }
  .subnav-tab {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.5rem 0.85rem;
    color: var(--tertiary);
    text-decoration: none;
    font-size: 13px;
    font-weight: 500;
    border-radius: var(--border-radius);
    white-space: nowrap;
    transition:
      color 150ms ease,
      background 150ms ease;
  }
  .subnav-tab:hover {
    color: var(--secondary);
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .subnav-tab.active {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 10%, transparent);
  }
  .subnav-tab svg {
    flex-shrink: 0;
  }

  .gamif-stack {
    position: fixed;
    bottom: 20px;
    right: 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 200;
    pointer-events: none;
  }
  .gamif-toast {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 10px;
    border: 1px solid color-mix(in oklab, var(--accent) 35%, transparent);
    background: var(--surface, var(--bg-elevated, var(--bg)));
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    box-shadow: 0 8px 24px color-mix(in oklab, black 22%, transparent);
    animation: gamif-slide 240ms ease-out;
    pointer-events: auto;
  }
  .gamif-toast[data-kind="level_up"] {
    border-color: var(--success, var(--accent));
    background: color-mix(in oklab, var(--success, var(--accent)) 10%, var(--surface, var(--bg)));
  }
  .gamif-toast[data-kind="achievement"] {
    border-color: var(--warning, var(--accent));
    background: color-mix(in oklab, var(--warning, var(--accent)) 10%, var(--surface, var(--bg)));
  }
  .gamif-icon {
    font-size: 18px;
  }
  @keyframes gamif-slide {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .gamif-toast {
      animation: none;
    }
  }
</style>
