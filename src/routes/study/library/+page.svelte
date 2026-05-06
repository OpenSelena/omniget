<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { goto } from "$app/navigation";
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import { t } from "$lib/i18n";
  import { page } from "$app/stores";
  import { replaceState } from "$app/navigation";
  import SegmentedControl from "$lib/study-components/SegmentedControl.svelte";
  import ConfirmDialog from "$lib/study-components/ConfirmDialog.svelte";
  import LibraryHomeShelves from "$lib/study-components/shelves/LibraryHomeShelves.svelte";
  import SortDropdown from "$lib/study-components/shelves/SortDropdown.svelte";
  import SearchInput from "$lib/study-components/shelves/SearchInput.svelte";
  import type { CatalogSort, CatalogSortDirection } from "$lib/study-bridge";
  import TelegramAuthGate from "$lib/study-components/TelegramAuthGate.svelte";
  import TelegramBrowser from "$lib/study-components/TelegramBrowser.svelte";
  import {
    telegramSessionState,
    studyDefaultOutputDir,
    type TelegramSessionState,
  } from "$lib/study-telegram-bridge";

  type Course = {
    id: number;
    title: string;
    source_path: string;
    thumbnail_path: string | null;
    added_at: string;
    last_scan_at: string;
    favorite: boolean;
    total_lessons: number;
    completed_lessons: number;
    progress_pct: number;
  };

  type SortKey = CatalogSort;
  type ViewMode = "courses" | "browse" | "telegram";
  type CoursesView = "home" | "catalog";

  type RootEntry = {
    path: string;
    enabled: boolean;
    added_at: number;
    is_default: boolean;
  };

  type BrowseFolder = { path: string; name: string; video_count: number };
  type BrowseVideo = {
    path: string;
    name: string;
    stem: string;
    size: number;
    subtitle_path: string | null;
  };
  type BrowseDocument = {
    path: string;
    name: string;
    ext: string;
    size: number;
  };
  type BrowseCrumb = { path: string; name: string };
  type BrowseData = {
    path: string;
    parent: string | null;
    is_root_list: boolean;
    folders: BrowseFolder[];
    videos: BrowseVideo[];
    documents: BrowseDocument[];
    breadcrumb: BrowseCrumb[];
  };

  type LibraryCourse = {
    id: number;
    title: string;
    tags: string[];
    lesson_count: number;
    completed_lessons: number;
    total_duration_ms: number;
    last_opened_at: number | null;
  };

  type TagSummary = { tag: string; course_count: number };
  type PlaylistSummary = {
    id: number;
    name: string;
    description: string | null;
    color: string | null;
    course_count: number;
    updated_at: number;
  };

  let courses = $state<Course[]>([]);
  let loading = $state(true);
  let error = $state("");
  let search = $state("");
  let sortKey = $state<SortKey>("last_watched");
  let sortDirection = $state<CatalogSortDirection>("desc");
  let favoritesOnly = $state(false);
  let coursesView = $state<CoursesView>("home");
  let gridPageSize = $state(30);
  let gridSentinel = $state<HTMLElement | null>(null);

  $effect(() => {
    if (!gridSentinel) return;
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting && gridPageSize < courses.length) {
            gridPageSize = Math.min(gridPageSize + 30, courses.length);
          }
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(gridSentinel);
    return () => observer.disconnect();
  });

  $effect(() => {
    void search;
    void statusFilter;
    void sortKey;
    void sortDirection;
    gridPageSize = 30;
  });

  let allTags = $state<TagSummary[]>([]);
  let courseTagsMap = $state<Map<number, string[]>>(new Map());
  let totalDurationByCourse = $state<Map<number, number>>(new Map());
  let lastOpenedByCourse = $state<Map<number, number>>(new Map());
  let playlists = $state<PlaylistSummary[]>([]);
  let playlistCoursesCache = $state<Map<number, number[]>>(new Map());

  let includeTags = $state<Set<string>>(new Set());
  let excludeTags = $state<Set<string>>(new Set());
  let statusFilter = $state<"all" | "not_started" | "in_progress" | "completed">("all");
  let playlistFilter = $state<number | null>(null);
  let filtersOpen = $state(true);

  let mode = $state<ViewMode>("courses");

  function syncCoursesUrl() {
    const params = new URLSearchParams();
    if (mode === "courses" && coursesView !== "home") {
      params.set("view", coursesView);
    }
    if (sortKey !== "last_watched" || sortDirection !== "desc") {
      params.set("sort", sortKey);
      if (sortDirection !== "desc") params.set("dir", sortDirection);
    }
    if (search.trim().length > 0) {
      params.set("q", search.trim());
    }
    const queryString = params.toString();
    const next = queryString.length > 0 ? `?${queryString}` : "";
    if (typeof window !== "undefined" && window.location.search !== next) {
      try {
        replaceState(`${window.location.pathname}${next}`, {});
      } catch {
        window.history.replaceState(null, "", `${window.location.pathname}${next}`);
      }
    }
  }

  function readUrlParams() {
    if (typeof window === "undefined") return;
    const params = new URLSearchParams(window.location.search);
    const v = params.get("view");
    if (v === "catalog" || v === "home") coursesView = v;
    const s = params.get("sort");
    if (s) sortKey = s as SortKey;
    const d = params.get("dir");
    if (d === "asc" || d === "desc") sortDirection = d;
    const q = params.get("q");
    if (q) search = q;
  }

  $effect(() => {
    void mode;
    void coursesView;
    void sortKey;
    void sortDirection;
    void search;
    syncCoursesUrl();
  });

  function onSortChange(s: CatalogSort, d: CatalogSortDirection) {
    sortKey = s;
    sortDirection = d;
  }

  function onSearchChange(next: string) {
    search = next;
  }

  let rootsOpen = $state(false);
  let roots = $state<RootEntry[]>([]);
  let browsePath = $state<string>("");
  let browseData = $state<BrowseData | null>(null);
  let browseLoading = $state(false);
  let rescanning = $state(false);
  let rescanMsg = $state("");

  const modeOptions = $derived([
    { value: "courses", label: $t("study.library.mode_courses") },
    { value: "browse", label: $t("study.library.mode_browse") },
    { value: "telegram", label: $t("study.library.mode_telegram") },
  ]);

  let tgSession = $state<TelegramSessionState>({ kind: "unauthenticated" });
  let tgChecking = $state(false);
  let tgOutputDir = $state("");
  let downloadsActive = $state(0);

  async function checkTelegramSession() {
    tgChecking = true;
    try {
      tgSession = await telegramSessionState();
      if (tgSession.kind === "authenticated" && !tgOutputDir) {
        try {
          const r = await studyDefaultOutputDir();
          tgOutputDir = r.path;
        } catch {
          /* leave empty; UI tells user */
        }
      }
    } finally {
      tgChecking = false;
    }
  }

  function onTelegramAuthenticated(_e: CustomEvent<{ phone: string }>) {
    void checkTelegramSession();
  }

  function onDownloadEnqueued() {
    goto("/downloads");
  }

  function onTelegramLogout() {
    void checkTelegramSession();
  }

  const playlistMembers = $derived.by(() => {
    if (playlistFilter == null) return null;
    const ids = playlistCoursesCache.get(playlistFilter);
    return ids ? new Set(ids) : new Set<number>();
  });

  const visibleCourses = $derived.by(() => {
    const term = search.trim().toLowerCase();
    const inc = includeTags;
    const exc = excludeTags;
    const members = playlistMembers;
    let list = courses.filter((c) => {
      if (favoritesOnly && !c.favorite) return false;
      if (term && !c.title.toLowerCase().includes(term)) return false;
      if (members && !members.has(c.id)) return false;
      if (statusFilter === "completed" && c.progress_pct < 100) return false;
      if (statusFilter === "not_started" && c.progress_pct > 0) return false;
      if (
        statusFilter === "in_progress" &&
        (c.progress_pct === 0 || c.progress_pct >= 100)
      )
        return false;
      const myTags = courseTagsMap.get(c.id) ?? [];
      if (inc.size > 0 && ![...inc].some((t) => myTags.includes(t)))
        return false;
      if (exc.size > 0 && [...exc].some((t) => myTags.includes(t)))
        return false;
      return true;
    });
    list = [...list].sort((a, b) => sortCompare(a, b, sortKey, sortDirection));
    return list;
  });

  function sortCompare(
    a: Course,
    b: Course,
    key: SortKey,
    dir: CatalogSortDirection,
  ): number {
    const sign = dir === "asc" ? 1 : -1;
    switch (key) {
      case "name":
        return sign * a.title.localeCompare(b.title);
      case "added":
        return sign * a.added_at.localeCompare(b.added_at);
      case "progress":
        return sign * (a.progress_pct - b.progress_pct);
      case "last_watched": {
        const la = lastOpenedByCourse.get(a.id) ?? 0;
        const lb = lastOpenedByCourse.get(b.id) ?? 0;
        return sign * (la - lb);
      }
      case "times_watched":
        return sign * (a.completed_lessons - b.completed_lessons);
      case "watched": {
        const wa = a.completed_lessons >= a.total_lessons && a.total_lessons > 0 ? 1 : 0;
        const wb = b.completed_lessons >= b.total_lessons && b.total_lessons > 0 ? 1 : 0;
        return sign * (wa - wb);
      }
      case "not_watched": {
        const wa = a.completed_lessons === 0 ? 1 : 0;
        const wb = b.completed_lessons === 0 ? 1 : 0;
        return sign * (wa - wb);
      }
      case "platform":
        return sign * a.title.localeCompare(b.title);
      default:
        return 0;
    }
  }

  function toggleIncludeTag(tag: string) {
    excludeTags.delete(tag);
    if (includeTags.has(tag)) includeTags.delete(tag);
    else includeTags.add(tag);
    includeTags = new Set(includeTags);
    excludeTags = new Set(excludeTags);
  }

  function toggleExcludeTag(tag: string) {
    includeTags.delete(tag);
    if (excludeTags.has(tag)) excludeTags.delete(tag);
    else excludeTags.add(tag);
    includeTags = new Set(includeTags);
    excludeTags = new Set(excludeTags);
  }

  function clearTagFilters() {
    includeTags = new Set();
    excludeTags = new Set();
  }

  async function loadEnrichment() {
    try {
      const [lib, tags, pls] = await Promise.all([
        pluginInvoke<LibraryCourse[]>("study", "study:courses:library", { filter: {} }),
        pluginInvoke<TagSummary[]>("study", "study:courses:tags:list_all"),
        pluginInvoke<PlaylistSummary[]>("study", "study:courses:playlists:list"),
      ]);
      const tagMap = new Map<number, string[]>();
      const durMap = new Map<number, number>();
      const openedMap = new Map<number, number>();
      for (const c of lib) {
        tagMap.set(c.id, c.tags);
        durMap.set(c.id, c.total_duration_ms);
        if (c.last_opened_at != null) {
          openedMap.set(c.id, c.last_opened_at);
        }
      }
      courseTagsMap = tagMap;
      totalDurationByCourse = durMap;
      lastOpenedByCourse = openedMap;
      allTags = tags;
      playlists = pls;
    } catch (e) {
      console.error("loadEnrichment failed", e);
    }
  }

  async function selectPlaylist(id: number | null) {
    playlistFilter = id;
    if (id != null && !playlistCoursesCache.has(id)) {
      try {
        const items = await pluginInvoke<{ course_id: number }[]>(
          "study",
          "study:courses:playlists:courses",
          { playlistId: id },
        );
        const map = new Map(playlistCoursesCache);
        map.set(
          id,
          items.map((i) => i.course_id),
        );
        playlistCoursesCache = map;
      } catch (e) {
        console.error("playlist courses failed", e);
      }
    }
  }

  const visibleBrowse = $derived.by<BrowseData | null>(() => {
    if (!browseData) return null;
    const term = search.trim().toLowerCase();
    if (!term) return browseData;
    return {
      ...browseData,
      folders: browseData.folders.filter((f) => f.name.toLowerCase().includes(term)),
      videos: browseData.videos.filter((v) => v.name.toLowerCase().includes(term)),
      documents: (browseData.documents ?? []).filter((d) =>
        d.name.toLowerCase().includes(term),
      ),
    };
  });

  async function loadCourses() {
    try {
      courses = await pluginInvoke<Course[]>("study", "study:courses:list");
      await loadEnrichment();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function loadRoots() {
    try {
      const res = await pluginInvoke<{ roots: RootEntry[] }>(
        "study",
        "study:library:roots_list",
      );
      roots = res.roots ?? [];
    } catch (e) {
      console.error("roots_list failed", e);
    }
  }

  async function loadBrowse(path: string) {
    browseLoading = true;
    try {
      browseData = await pluginInvoke<BrowseData>("study", "study:browse:list", {
        path,
      });
      browsePath = browseData.path;
    } catch (e) {
      console.error("browse failed", e);
      browseData = null;
    } finally {
      browseLoading = false;
    }
  }

  type HealthReport = {
    zero_byte_videos: { lesson_id: number; course_id: number; lesson_title: string; course_title: string; video_path: string }[];
    missing_videos: { lesson_id: number; course_id: number; lesson_title: string; course_title: string; video_path: string }[];
    orphan_descriptions: { dir: string; description_path: string }[];
    total_issues: number;
  };

  let healthReport = $state<HealthReport | null>(null);
  let healthOpen = $state(false);
  let healthLoading = $state(false);
  let cleanupBusy = $state(false);
  let cleanupMsg = $state("");
  let cleanupConfirmOpen = $state(false);
  let cleanupForceConfirmOpen = $state(false);

  async function runCleanupMissing(force: boolean = false) {
    if (cleanupBusy) return;
    cleanupBusy = true;
    cleanupMsg = "";
    try {
      const r = await pluginInvoke<{
        removed_lessons: number;
        removed_courses: number;
        kept_unmounted: number;
      }>("study", "study:library:cleanup_missing", { forceUnmounted: force });
      cleanupMsg = $t("study.library.cleanup_done", {
        lessons: r.removed_lessons,
        courses: r.removed_courses,
        kept: r.kept_unmounted,
      });
      await loadHealth();
      await loadCourses();
    } catch (e) {
      cleanupMsg = e instanceof Error ? e.message : String(e);
    } finally {
      cleanupBusy = false;
      setTimeout(() => (cleanupMsg = ""), 8000);
    }
  }

  function runCleanupSafe() {
    void runCleanupMissing(false);
  }
  function runCleanupForce() {
    void runCleanupMissing(true);
  }

  async function deleteOrphanDescription(path: string) {
    try {
      await pluginInvoke("study", "study:library:delete_orphan_description", {
        path,
      });
      const name = path.split(/[\\/]/).pop() ?? path;
      showToast("success", $t("study.library_health.removed_toast", { name }) as string);
      await loadHealth();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error("delete orphan description failed", e);
      showToast("error", $t("study.library_health.remove_failed_toast", { msg }) as string);
    }
  }

  async function loadHealth(opts?: { announce?: boolean }) {
    try {
      const r = await pluginInvoke<HealthReport>(
        "study",
        "study:library:health",
      );
      healthReport = r;
      if (opts?.announce) {
        if (r.total_issues === 0) {
          showToast("success", $t("study.library_health.no_issues_toast") as string);
        } else {
          showToast(
            "info",
            $t("study.library_health.issues_found_toast", { count: r.total_issues }) as string,
          );
        }
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error("library health failed", e);
      healthReport = null;
      if (opts?.announce) {
        showToast("error", $t("study.library_health.verify_failed_toast", { msg }) as string);
      }
    }
  }

  async function openHealth() {
    healthOpen = true;
    if (!healthReport) {
      healthLoading = true;
      await loadHealth();
      healthLoading = false;
    }
  }

  onMount(async () => {
    readUrlParams();
    await Promise.all([loadCourses(), loadRoots()]);
    loadHealth();
    loading = false;
  });

  $effect(() => {
    const m = mode;
    if (m === "browse" && untrack(() => !browseData)) {
      loadBrowse("");
    }
    if (m === "telegram" && untrack(() => !tgChecking && tgSession.kind !== "authenticated")) {
      void checkTelegramSession();
    }
  });

  async function toggleFavorite(course: Course, ev: MouseEvent) {
    ev.stopPropagation();
    ev.preventDefault();
    try {
      const res = await pluginInvoke<{ id: number; favorite: boolean }>(
        "study",
        "study:courses:toggle_favorite",
        { courseId: course.id },
      );
      courses = courses.map((c) =>
        c.id === res.id ? { ...c, favorite: res.favorite } : c,
      );
    } catch (e) {
      console.error("toggle favorite failed", e);
    }
  }

  async function toggleRoot(r: RootEntry) {
    try {
      const res = await pluginInvoke<{ roots: RootEntry[] }>(
        "study",
        "study:library:roots_toggle",
        { path: r.path, enabled: !r.enabled },
      );
      roots = res.roots ?? [];
    } catch (e) {
      console.error("toggle root failed", e);
    }
  }

  async function removeRoot(r: RootEntry) {
    try {
      const res = await pluginInvoke<{ roots: RootEntry[] }>(
        "study",
        "study:library:roots_remove",
        { path: r.path },
      );
      roots = res.roots ?? [];
    } catch (e) {
      console.error("remove root failed", e);
    }
  }

  async function addRoot() {
    try {
      const dialog = await import("@tauri-apps/plugin-dialog");
      const picked = await dialog.open({ directory: true, multiple: false });
      if (typeof picked !== "string" || !picked) return;
      const res = await pluginInvoke<{ roots: RootEntry[] }>(
        "study",
        "study:library:roots_add",
        { path: picked },
      );
      roots = res.roots ?? [];
    } catch (e) {
      console.error("add root failed", e);
    }
  }

  async function rescan() {
    if (rescanning) return;
    rescanning = true;
    rescanMsg = "";
    try {
      const res = await pluginInvoke<{ courses_found?: number; lessons_found?: number }>(
        "study",
        "study:rescan",
      );
      rescanMsg = $t("study.library.rescan_ok", {
        courses: res?.courses_found ?? 0,
        lessons: res?.lessons_found ?? 0,
      });
      await Promise.all([loadCourses(), loadRoots()]);
      if (mode === "browse") await loadBrowse(browsePath);
    } catch (e) {
      rescanMsg = e instanceof Error ? e.message : String(e);
    } finally {
      rescanning = false;
    }
  }

  function fmtBytes(n: number): string {
    if (!n) return "";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let i = 0;
    let v = n;
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024;
      i += 1;
    }
    return `${v.toFixed(v >= 10 || i === 0 ? 0 : 1)} ${units[i]}`;
  }

  function compactPath(path: string, max = 48): string {
    if (path.length <= max) return path;
    return `…${path.slice(path.length - max + 1)}`;
  }

  async function openPath(path: string) {
    try {
      await invoke("open_path_default", { path });
    } catch (e) {
      console.error("open failed", e);
      try {
        const shell = await import("@tauri-apps/plugin-shell");
        await shell.open(path);
      } catch (e2) {
        console.error("shell fallback failed", e2);
      }
    }
  }

  function openBrowseVideo(v: BrowseVideo) {
    const params = new URLSearchParams({ path: v.path, name: v.name });
    if (v.subtitle_path) params.set("subtitle", v.subtitle_path);
    goto(`/study/watch?${params.toString()}`);
  }

  async function openBrowseDocument(d: BrowseDocument) {
    await openPath(d.path);
  }
</script>

<section class="study-page">
  <header class="head">
    <div class="row-primary">
      <h1 class="page-title">{$t("study.library.title")}</h1>
      <div class="right">
        <button
          type="button"
          class="ghost-btn"
          onclick={() => (rootsOpen = true)}
          aria-label={$t("study.library.roots_button")}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M3 7v13h18V9h-9l-2-2H3z"></path>
          </svg>
          <span>{$t("study.library.roots_button")}</span>
          {#if roots.length > 0}
            <span class="badge">{roots.filter((r) => r.enabled).length}</span>
          {/if}
        </button>
      </div>
    </div>

    <div class="row-secondary">
      <SegmentedControl bind:value={mode} options={modeOptions} ariaLabel="mode" />
      {#if mode !== "courses"}
        <input
          type="search"
          placeholder={$t("study.library.search_placeholder")}
          bind:value={search}
          class="search"
        />
      {:else}
        <SearchInput
          value={search}
          onChange={onSearchChange}
          placeholder={$t("study.library.search_placeholder")}
        />
        <div class="view-toggle" role="tablist" aria-label={$t("study.library.view_mode_aria")}>
          <button
            type="button"
            class="view-btn"
            class:active={coursesView === "home"}
            role="tab"
            aria-selected={coursesView === "home"}
            onclick={() => (coursesView = "home")}
          >
            Início
          </button>
          <button
            type="button"
            class="view-btn"
            class:active={coursesView === "catalog"}
            role="tab"
            aria-selected={coursesView === "catalog"}
            onclick={() => (coursesView = "catalog")}
          >
            Catálogo
          </button>
        </div>
        {#if coursesView === "catalog"}
          <SortDropdown
            value={sortKey}
            direction={sortDirection}
            onChange={onSortChange}
          />
          <label class="fav-toggle">
            <input type="checkbox" bind:checked={favoritesOnly} />
            <span>{$t("study.library.favorites_only")}</span>
          </label>
        {/if}
      {/if}
    </div>
  </header>

  {#if healthReport && healthReport.total_issues > 0}
    {@const parts = [
      healthReport.zero_byte_videos.length > 0
        ? $t("study.library.health_banner_part_empty", { count: healthReport.zero_byte_videos.length })
        : null,
      healthReport.missing_videos.length > 0
        ? $t("study.library.health_banner_part_missing", { count: healthReport.missing_videos.length })
        : null,
      healthReport.orphan_descriptions.length > 0
        ? $t("study.library.health_banner_part_orphan", { count: healthReport.orphan_descriptions.length })
        : null,
    ].filter(Boolean)}
    <button
      class="health-banner"
      type="button"
      onclick={openHealth}
      aria-label={$t("study.library.health_banner_aria")}
    >
      <span class="health-dot" aria-hidden="true"></span>
      <span class="health-text">
        <strong>
          {healthReport.total_issues === 1
            ? $t("study.library.health_banner_one")
            : $t("study.library.health_banner_many", { count: healthReport.total_issues })}
        </strong>
        {#if parts.length > 0}
          <span class="health-parts">{parts.join(" · ")}</span>
        {/if}
      </span>
      <span class="health-cta">{$t("study.library.health_banner_cta")}</span>
    </button>
  {/if}

  {#if mode === "courses" && coursesView === "home"}
    <section class="home-shelves">
      <LibraryHomeShelves />
    </section>
  {/if}

  {#if mode === "courses" && coursesView === "catalog"}
    <div class="status-row">
      <SegmentedControl
        bind:value={statusFilter}
        options={[
          { value: "all", label: $t("study.library.status_all") },
          { value: "in_progress", label: $t("study.library.status_in_progress") },
          { value: "not_started", label: $t("study.library.status_not_started") },
          { value: "completed", label: $t("study.library.status_completed") },
        ]}
        ariaLabel={$t("study.library.status_aria")}
      />
      <span class="result-count-inline">
        {visibleCourses.length} de {courses.length}
      </span>
    </div>
    <div class="content-cols">
      <aside class="filters" class:open={filtersOpen}>
        <button
          type="button"
          class="filters-toggle"
          onclick={() => (filtersOpen = !filtersOpen)}
        >
          <span>{$t("study.library.filters")}</span>
          <span class="count" aria-hidden="true">
            {includeTags.size + excludeTags.size +
              (statusFilter !== "all" ? 1 : 0) +
              (playlistFilter != null ? 1 : 0)}
          </span>
        </button>

        {#if filtersOpen}
          <section class="filter-section">
            <h3>{$t("study.library.filter_status")}</h3>
            <div class="status-buttons">
              {#each [
                { v: "all", label: $t("study.library.status_all") },
                { v: "not_started", label: $t("study.library.status_not_started") },
                { v: "in_progress", label: $t("study.library.status_in_progress") },
                { v: "completed", label: $t("study.library.status_completed") },
              ] as opt (opt.v)}
                <button
                  type="button"
                  class="status-btn"
                  class:active={statusFilter === opt.v}
                  onclick={() => (statusFilter = opt.v as typeof statusFilter)}
                >
                  {opt.label}
                </button>
              {/each}
            </div>
          </section>

          {#if playlists.length > 0}
            <section class="filter-section">
              <h3>{$t("study.library.filter_playlist")}</h3>
              <div class="playlist-list">
                <button
                  type="button"
                  class="pl-row"
                  class:active={playlistFilter == null}
                  onclick={() => selectPlaylist(null)}
                >
                  <span>{$t("study.library.all_courses")}</span>
                  <span class="pl-count">{courses.length}</span>
                </button>
                {#each playlists as p (p.id)}
                  <button
                    type="button"
                    class="pl-row"
                    class:active={playlistFilter === p.id}
                    onclick={() => selectPlaylist(p.id)}
                    style:--pl-color={p.color ?? "var(--accent)"}
                  >
                    <span class="pl-dot"></span>
                    <span>{p.name}</span>
                    <span class="pl-count">{p.course_count}</span>
                  </button>
                {/each}
              </div>
              <a href="/study/library/playlists" class="manage">{$t("study.library.manage_playlists")}</a>
            </section>
          {:else}
            <section class="filter-section">
              <h3>Playlist</h3>
              <a href="/study/library/playlists" class="empty-cta">
                + Criar primeira playlist
              </a>
            </section>
          {/if}

          {#if allTags.length > 0}
            <section class="filter-section">
              <header class="tags-head">
                <h3>{$t("study.library.filter_tags")}</h3>
                {#if includeTags.size + excludeTags.size > 0}
                  <button
                    type="button"
                    class="clear-link"
                    onclick={clearTagFilters}
                  >{$t("study.library.clear_filters")}</button>
                {/if}
              </header>
              <p class="hint">{$t("study.library.tags_hint")}</p>
              <div class="tag-grid">
                {#each allTags.slice(0, 40) as t (t.tag)}
                  {@const inc = includeTags.has(t.tag)}
                  {@const exc = excludeTags.has(t.tag)}
                  <button
                    type="button"
                    class="tag-pill"
                    class:include={inc}
                    class:exclude={exc}
                    onclick={(e) =>
                      e.shiftKey
                        ? toggleExcludeTag(t.tag)
                        : toggleIncludeTag(t.tag)}
                  >
                    {#if exc}<span class="prefix">−</span>{/if}
                    {#if inc}<span class="prefix">+</span>{/if}
                    #{t.tag}
                    <span class="t-count">{t.course_count}</span>
                  </button>
                {/each}
              </div>
            </section>
          {/if}
        {/if}
      </aside>

      <div class="content-area">
    {#if loading}
      <p class="muted">{$t("study.common.loading")}</p>
    {:else if error}
      <p class="error">{error}</p>
    {:else if visibleCourses.length === 0}
      <p class="muted">{$t("study.library.empty")}</p>
    {:else}
      <p class="result-count">
        {visibleCourses.length} de {courses.length} cursos
      </p>
      <div class="grid">
        {#each visibleCourses.slice(0, gridPageSize) as c (c.id)}
          <a class="card course-card" href="/study/course/{c.id}">
            <div
              class="thumb course-thumb"
              style={c.thumbnail_path
                ? `background-image: url('${convertFileSrc(c.thumbnail_path)}');`
                : ""}
            >
              {#if !c.thumbnail_path}
                <svg viewBox="0 0 64 64" width="40" height="40" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <path d="M12 10h40a2 2 0 0 1 2 2v36a2 2 0 0 1-2 2H12a2 2 0 0 1-2-2V12a2 2 0 0 1 2-2z"/>
                  <path d="M10 22h44"/>
                  <path d="M22 10v12"/>
                  <path d="M42 10v12"/>
                </svg>
              {/if}
              <span class="kind-badge">{$t("study.library.kind_course")}</span>
              <div class="progress" style="--pct: {c.progress_pct}%"></div>
            </div>
            <div class="meta">
              <div class="title-row">
                <span class="title">{c.title}</span>
                <button
                  type="button"
                  class="star"
                  class:active={c.favorite}
                  onclick={(e) => toggleFavorite(c, e)}
                  aria-label={c.favorite
                    ? $t("study.library.favorite_on")
                    : $t("study.library.favorite_off")}
                >
                  {c.favorite ? "★" : "☆"}
                </button>
              </div>
              <div class="sub">
                <span>{$t("study.library.lessons_count", { count: c.total_lessons })}</span>
                <span>·</span>
                <span>{$t("study.library.progress_pct", { pct: Math.round(c.progress_pct) })}</span>
              </div>
            </div>
          </a>
        {/each}
      </div>
      {#if gridPageSize < visibleCourses.length}
        <div class="grid-sentinel" bind:this={gridSentinel} aria-hidden="true">
          <span class="muted">{$t("study.library.loading_more")}</span>
        </div>
      {/if}
    {/if}
      </div>
    </div>
  {:else if mode === "browse"}
    {#if browseLoading && !browseData}
      <p class="muted">{$t("study.common.loading")}</p>
    {:else if !browseData}
      <p class="muted">{$t("study.library.browse_empty_roots")}</p>
    {:else}
      {#if browseData.breadcrumb.length > 0 || !browseData.is_root_list}
        <nav class="crumbs" aria-label="breadcrumb">
          <button type="button" class="crumb" onclick={() => loadBrowse("")}>
            {$t("study.library.browse_breadcrumb_root")}
          </button>
          {#each browseData.breadcrumb as cr, i (cr.path)}
            <span class="crumb-sep" aria-hidden="true">›</span>
            {#if i < browseData.breadcrumb.length - 1}
              <button type="button" class="crumb" onclick={() => loadBrowse(cr.path)}>
                {cr.name}
              </button>
            {:else}
              <span class="crumb-current">{cr.name}</span>
            {/if}
          {/each}
        </nav>
      {/if}

      {#if visibleBrowse && (visibleBrowse.folders.length > 0 || visibleBrowse.videos.length > 0 || (visibleBrowse.documents?.length ?? 0) > 0)}
        <div class="grid">
          {#each visibleBrowse.folders as f (f.path)}
            <button type="button" class="card folder-card" onclick={() => loadBrowse(f.path)}>
              <div class="thumb folder-thumb">
                <svg viewBox="0 0 64 64" width="56" height="56" fill="currentColor" aria-hidden="true">
                  <path d="M6 14a4 4 0 0 1 4-4h14l6 6h24a4 4 0 0 1 4 4v30a4 4 0 0 1-4 4H10a4 4 0 0 1-4-4V14z" opacity="0.9"/>
                  <path d="M6 22h52v2H6z" opacity="0.3"/>
                </svg>
                <span class="kind-badge">{$t("study.library.folder")}</span>
              </div>
              <div class="meta">
                <div class="title-row">
                  <span class="title">{f.name}</span>
                </div>
                <div class="sub">
                  <span>{$t("study.library.videos_count", { count: f.video_count })}</span>
                </div>
              </div>
            </button>
          {/each}
          {#each visibleBrowse.videos as v (v.path)}
            <button type="button" class="card video-card" onclick={() => openBrowseVideo(v)}>
              <div class="thumb video-thumb">
                <svg viewBox="0 0 64 64" width="56" height="56" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <rect x="6" y="12" width="52" height="40" rx="4" fill="currentColor" opacity="0.12"/>
                  <rect x="6" y="12" width="52" height="40" rx="4"/>
                  <path d="M27 24l14 8-14 8z" fill="currentColor"/>
                </svg>
                <span class="kind-badge">{$t("study.library.video")}</span>
              </div>
              <div class="meta">
                <div class="title-row">
                  <span class="title">{v.name}</span>
                </div>
                <div class="sub">
                  {#if v.size}<span>{fmtBytes(v.size)}</span>{/if}
                  {#if v.subtitle_path}<span>·</span><span>CC</span>{/if}
                </div>
              </div>
            </button>
          {/each}
          {#each (visibleBrowse.documents ?? []) as d (d.path)}
            <button type="button" class="card doc-card" data-ext={d.ext} onclick={() => openBrowseDocument(d)}>
              <div class="thumb doc-thumb">
                <svg viewBox="0 0 64 64" width="52" height="64" fill="none" aria-hidden="true">
                  <path d="M14 4h28l10 10v42a4 4 0 0 1-4 4H14a4 4 0 0 1-4-4V8a4 4 0 0 1 4-4z" fill="currentColor" opacity="0.08"/>
                  <path d="M14 4h28l10 10v42a4 4 0 0 1-4 4H14a4 4 0 0 1-4-4V8a4 4 0 0 1 4-4z" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/>
                  <path d="M42 4v10h10" stroke="currentColor" stroke-width="2" stroke-linejoin="round" fill="currentColor" fill-opacity="0.15"/>
                </svg>
                <span class="ext-stamp">{d.ext.toUpperCase()}</span>
              </div>
              <div class="meta">
                <div class="title-row">
                  <span class="title">{d.name}</span>
                </div>
                <div class="sub">
                  {#if d.size}<span>{fmtBytes(d.size)}</span>{/if}
                </div>
              </div>
            </button>
          {/each}
        </div>
      {:else}
        <p class="muted">
          {browseData.is_root_list
            ? $t("study.library.browse_empty_roots")
            : $t("study.library.browse_empty")}
        </p>
      {/if}
    {/if}
  {:else if mode === "telegram"}
    {#if tgChecking}
      <p class="muted">{$t("study.common.loading")}</p>
    {:else if tgSession.kind === "plugin_missing"}
      <div class="tg-empty">
        <h2>{$t("study.library.telegram.plugin_missing_title")}</h2>
        <p class="muted">{$t("study.library.telegram.plugin_missing_body")}</p>
      </div>
    {:else if tgSession.kind === "unauthenticated"}
      <TelegramAuthGate on:authenticated={onTelegramAuthenticated} />
    {:else}
      <TelegramBrowser
        outputDir={tgOutputDir}
        onDownloadEnqueued={onDownloadEnqueued}
        onLogout={onTelegramLogout}
      />
    {/if}
  {/if}
</section>

{#if mode === "telegram" && tgSession.kind === "authenticated"}
  <button
    type="button"
    class="downloads-fab"
    class:has-active={downloadsActive > 0}
    onclick={() => goto("/downloads")}
    aria-label={$t("study.library.telegram.downloads.toggle")}
    title={$t("study.library.telegram.downloads_view_panel")}
  >
    <span aria-hidden="true">↓</span>
    {#if downloadsActive > 0}
      <span class="badge">{downloadsActive}</span>
    {/if}
  </button>
{/if}

{#if healthOpen}
  <div
    class="drawer-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) healthOpen = false;
    }}
    onkeydown={(e) => {
      if (e.key === "Escape") healthOpen = false;
    }}
  >
    <div class="health-drawer" role="dialog" aria-modal="true" aria-labelledby="health-title" tabindex="-1">
      <header class="drawer-head">
        <h2 id="health-title">{$t("study.library.health_title")}</h2>
        <button
          type="button"
          class="ghost-btn"
          onclick={() => {
            healthLoading = true;
            loadHealth({ announce: true }).finally(() => (healthLoading = false));
          }}
          disabled={healthLoading}
        >
          {healthLoading ? $t("study.library.health_checking") : $t("study.library.health_recheck")}
        </button>
        <button class="close-btn" onclick={() => (healthOpen = false)} aria-label={$t("study.common.close")}>×</button>
      </header>

      {#if !healthReport}
        <p class="muted small">{$t("study.common.loading")}</p>
      {:else if healthReport.total_issues === 0}
        <p class="health-ok">
          <span class="health-ok-dot" aria-hidden="true"></span>
          {$t("study.library.health_ok")}
        </p>
      {:else}
        {#if healthReport.zero_byte_videos.length > 0}
          <section class="health-section">
            <h3>
              {$t("study.library.health_zero_byte")}
              <span class="health-count">{healthReport.zero_byte_videos.length}</span>
            </h3>
            <p class="muted small">{$t("study.library.health_zero_byte_hint")}</p>
            <ul class="health-list">
              {#each healthReport.zero_byte_videos as v (v.lesson_id)}
                <li>
                  <strong>{v.course_title}</strong> · {v.lesson_title}
                  <code class="path">{v.video_path}</code>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if healthReport.missing_videos.length > 0}
          <section class="health-section">
            <h3>
              {$t("study.library.health_missing")}
              <span class="health-count">{healthReport.missing_videos.length}</span>
            </h3>
            <p class="muted small">{$t("study.library.health_missing_hint")}</p>
            <div class="health-actions">
              <button
                type="button"
                class="ghost-btn"
                disabled={cleanupBusy}
                onclick={() => (cleanupConfirmOpen = true)}
              >
                {cleanupBusy ? $t("study.library.cleanup_busy") : $t("study.library.cleanup_btn")}
              </button>
              <button
                type="button"
                class="ghost-btn danger"
                disabled={cleanupBusy}
                onclick={() => (cleanupForceConfirmOpen = true)}
                title={$t("study.library.cleanup_force_hint")}
              >
                {$t("study.library.cleanup_force_btn")}
              </button>
              {#if cleanupMsg}
                <span class="muted small">{cleanupMsg}</span>
              {/if}
            </div>
            <ul class="health-list">
              {#each healthReport.missing_videos as v (v.lesson_id)}
                <li>
                  <strong>{v.course_title}</strong> · {v.lesson_title}
                  <code class="path">{v.video_path}</code>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if healthReport.orphan_descriptions.length > 0}
          <section class="health-section">
            <h3>
              {$t("study.library.health_orphan_desc")}
              <span class="health-count">{healthReport.orphan_descriptions.length}</span>
            </h3>
            <p class="muted small">{$t("study.library.health_orphan_hint")}</p>
            <ul class="health-list">
              {#each healthReport.orphan_descriptions as o, i (i)}
                <li>
                  <code class="path">{o.dir}</code>
                  <button
                    type="button"
                    class="link danger"
                    onclick={() => deleteOrphanDescription(o.description_path)}
                    title={o.description_path}
                  >
                    {$t("study.library.cleanup_orphan_delete")}
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
      {/if}
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:open={cleanupConfirmOpen}
  title={$t("study.library.cleanup_confirm_title")}
  message={$t("study.library.cleanup_confirm_message", {
    count: healthReport?.missing_videos.length ?? 0,
  })}
  confirmLabel={$t("study.library.cleanup_btn")}
  variant="danger"
  onConfirm={runCleanupSafe}
/>

<ConfirmDialog
  bind:open={cleanupForceConfirmOpen}
  title={$t("study.library.cleanup_force_confirm_title")}
  message={$t("study.library.cleanup_force_confirm_message", {
    count: healthReport?.missing_videos.length ?? 0,
  })}
  confirmLabel={$t("study.library.cleanup_force_btn")}
  variant="danger"
  onConfirm={runCleanupForce}
/>

{#if rootsOpen}
  <div
    class="drawer-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) rootsOpen = false;
    }}
    onkeydown={(e) => {
      if (e.key === "Escape") rootsOpen = false;
    }}
  >
    <div class="drawer" role="dialog" aria-modal="true" tabindex="-1">
      <header class="drawer-head">
        <h3>{$t("study.library.roots_button")}</h3>
        <button type="button" class="icon-btn" aria-label="close" onclick={() => (rootsOpen = false)}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
            <path d="M18 6L6 18"></path>
            <path d="M6 6l12 12"></path>
          </svg>
        </button>
      </header>
      <p class="drawer-hint">{$t("study.library.roots_hint")}</p>

      {#if roots.length === 0}
        <p class="muted drawer-empty">{$t("study.library.roots_empty")}</p>
      {:else}
        <ul class="roots-list">
          {#each roots as r (r.path)}
            <li class="root-row" class:disabled={!r.enabled}>
              <label class="root-toggle">
                <input
                  type="checkbox"
                  checked={r.enabled}
                  onchange={() => toggleRoot(r)}
                />
              </label>
              <div class="root-info">
                <span class="root-path" title={r.path}>{compactPath(r.path)}</span>
                {#if r.is_default}
                  <span class="root-tag">{$t("study.library.roots_default")}</span>
                {/if}
              </div>
              {#if !r.is_default}
                <button
                  type="button"
                  class="root-remove"
                  onclick={() => removeRoot(r)}
                  aria-label={$t("study.library.roots_remove")}
                  title={$t("study.library.roots_remove")}
                >×</button>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}

      <div class="drawer-actions">
        <button type="button" class="btn" onclick={addRoot}>
          + {$t("study.library.roots_add_btn")}
        </button>
        <button type="button" class="btn primary" onclick={rescan} disabled={rescanning}>
          {rescanning ? $t("study.library.rescanning") : $t("study.library.rescan_now")}
        </button>
      </div>
      {#if rescanMsg}
        <p class="drawer-msg">{rescanMsg}</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .home-shelves {
    margin: 0 0 24px;
  }
  .status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .result-count-inline {
    font-size: 12px;
    color: color-mix(in oklab, currentColor 60%, transparent);
    font-variant-numeric: tabular-nums;
  }
  .grid-sentinel {
    padding: 24px;
    text-align: center;
    font-size: 12px;
  }
  .view-toggle {
    display: inline-flex;
    border-radius: 8px;
    background: color-mix(in oklab, var(--content-bg) 80%, var(--accent) 4%);
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    padding: 2px;
    gap: 2px;
  }
  .view-btn {
    padding: 6px 12px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background var(--tg-duration-fast, 150ms);
  }
  .view-btn:hover:not(.active) {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .view-btn.active {
    background: var(--accent);
    color: var(--accent-contrast, white);
  }
  .content-cols {
    display: grid;
    grid-template-columns: 240px 1fr;
    gap: 16px;
  }
  @media (max-width: 760px) {
    .content-cols {
      grid-template-columns: 1fr;
    }
    .filters {
      max-height: 220px;
      overflow-y: auto;
    }
  }
  .filters {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 12px;
    background: var(--surface);
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    height: fit-content;
    position: sticky;
    top: 12px;
  }
  .filters-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 6px;
    border: 0;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .filters-toggle .count {
    background: var(--accent);
    color: var(--on-accent, var(--on-cta, white));
    border-radius: 999px;
    padding: 1px 7px;
    font-size: 10px;
    font-weight: 700;
    text-transform: none;
    letter-spacing: 0;
    min-width: 18px;
    text-align: center;
  }
  .filter-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .filter-section h3 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--tertiary);
  }
  .tags-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0;
    margin: 0;
  }
  .tags-head h3 {
    margin: 0;
  }
  .clear-link {
    border: 0;
    background: transparent;
    color: var(--accent);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
    padding: 0;
  }
  .clear-link:hover {
    text-decoration: underline;
  }
  .filter-section .hint {
    margin: 0;
    color: var(--tertiary);
    font-size: 10px;
  }

  .status-buttons {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .status-btn {
    padding: 5px 8px;
    border: 0;
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--secondary);
    font: inherit;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .status-btn:hover {
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .status-btn.active {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    color: var(--accent);
    font-weight: 600;
  }

  .playlist-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pl-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border: 0;
    border-radius: var(--border-radius);
    background: transparent;
    color: var(--secondary);
    font: inherit;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .pl-row:hover {
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .pl-row.active {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    color: var(--text);
    font-weight: 600;
  }
  .pl-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--pl-color, var(--accent));
    flex-shrink: 0;
  }
  .pl-row > span:nth-of-type(1):not(.pl-dot) {
    flex: 1;
  }
  .pl-row > span:not(.pl-dot) {
    flex: 1;
  }
  .pl-count {
    flex: 0 !important;
    color: var(--tertiary);
    font-weight: 500;
    font-size: 11px;
  }
  .manage,
  .empty-cta {
    color: var(--accent);
    text-decoration: none;
    font-size: 11px;
    padding: 4px 6px;
  }
  .manage:hover,
  .empty-cta:hover {
    text-decoration: underline;
  }

  .tag-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tag-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    background: transparent;
    color: var(--secondary);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .tag-pill:hover {
    border-color: var(--accent);
  }
  .tag-pill.include {
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    color: var(--accent);
    border-color: var(--accent);
    font-weight: 600;
  }
  .tag-pill.exclude {
    background: color-mix(
      in oklab,
      var(--error, var(--accent)) 14%,
      transparent
    );
    color: var(--error, var(--accent));
    border-color: var(--error, var(--accent));
    text-decoration: line-through;
  }
  .tag-pill .prefix {
    font-weight: 700;
  }
  .t-count {
    color: var(--tertiary);
    font-weight: 500;
    font-size: 10px;
  }

  .content-area {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .result-count {
    margin: 0;
    color: var(--tertiary);
    font-size: 12px;
  }

  .study-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    width: 100%;
    max-width: 1200px;
    margin-inline: auto;
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: var(--padding);
  }
  .row-primary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }
  .right {
    display: flex;
    gap: 0.5rem;
  }
  h1 {
    font-size: 22px;
    font-weight: 500;
    margin: 0;
    letter-spacing: -0.5px;
    color: var(--secondary);
  }
  .ghost-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.45rem 0.75rem;
    background: transparent;
    border: 1px solid var(--content-border);
    border-radius: var(--border-radius);
    color: var(--secondary);
    cursor: pointer;
    font-family: inherit;
    font-size: 13px;
    font-weight: 500;
    transition:
      border-color 150ms ease,
      background 150ms ease;
  }
  .ghost-btn:hover {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .ghost-btn .badge {
    padding: 0 0.4rem;
    border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 20%, transparent);
    color: var(--accent);
    font-size: 11px;
    font-family: var(--font-mono, "IBM Plex Mono", monospace);
  }
  .row-secondary {
    display: flex;
    flex-wrap: wrap;
    gap: var(--padding);
    align-items: center;
  }
  .search {
    flex: 1 1 220px;
    padding: 8px 12px;
    border-radius: var(--border-radius);
    background: var(--input-bg);
    border: 1px solid var(--input-border);
    color: var(--secondary);
    font-size: 13px;
    font-family: inherit;
  }
  .search:focus-visible {
    outline: var(--focus-ring, 2px solid var(--accent));
    outline-offset: 2px;
  }
  .sort {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--tertiary);
  }
  .sort select {
    background: var(--input-bg);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    color: var(--secondary);
    padding: 6px 8px;
    font-size: 13px;
    font-family: inherit;
  }
  .fav-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--tertiary);
    cursor: pointer;
  }
  .muted {
    color: var(--tertiary);
    font-size: 14px;
  }
  .error {
    color: var(--error);
    font-size: 14px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: calc(var(--padding) * 1.5);
  }
  .card {
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--elev-1);
    overflow: hidden;
    text-decoration: none;
    color: inherit;
    cursor: pointer;
    font-family: inherit;
    transition: transform var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out);
    text-align: left;
    padding: 0;
    transition:
      border-color 150ms ease,
      transform 150ms ease;
  }
  .card:hover {
    border-color: var(--accent);
    transform: translateY(-1px);
  }
  .card:focus-visible {
    outline: var(--focus-ring, 2px solid var(--accent));
    outline-offset: 2px;
  }
  .thumb {
    aspect-ratio: 16 / 9;
    background: var(--input-bg);
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--tertiary);
  }
  .course-thumb {
    background-size: cover;
    background-position: center;
    background-color: color-mix(in oklab, var(--accent) 6%, var(--input-bg));
    color: var(--accent);
  }
  .course-thumb svg {
    opacity: 0.4;
  }
  .kind-badge {
    position: absolute;
    top: 8px;
    left: 8px;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    background: color-mix(in oklab, var(--primary) 70%, transparent);
    color: var(--secondary);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 500;
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }
  .folder-thumb {
    background:
      radial-gradient(
        circle at 30% 30%,
        color-mix(in oklab, var(--accent) 15%, transparent),
        color-mix(in oklab, var(--accent) 4%, var(--input-bg))
      );
    color: var(--accent);
  }
  .video-thumb {
    background:
      radial-gradient(
        circle at 50% 50%,
        color-mix(in oklab, var(--accent) 12%, transparent),
        color-mix(in oklab, var(--primary) 50%, var(--input-bg))
      );
    color: var(--accent);
  }
  .doc-thumb {
    position: relative;
    background:
      linear-gradient(
        135deg,
        color-mix(in oklab, var(--accent) 10%, var(--input-bg)),
        var(--input-bg)
      );
    color: var(--tertiary);
  }
  .doc-card[data-ext="pdf"] .doc-thumb {
    color: var(--red, #dc2626);
  }
  .doc-card[data-ext="epub"] .doc-thumb,
  .doc-card[data-ext="mobi"] .doc-thumb,
  .doc-card[data-ext="azw3"] .doc-thumb {
    color: var(--blue, #2563eb);
  }
  .doc-card[data-ext="djvu"] .doc-thumb {
    color: var(--orange, #ea580c);
  }
  .doc-card[data-ext="cbz"] .doc-thumb,
  .doc-card[data-ext="cbr"] .doc-thumb {
    color: var(--green, #16a34a);
  }
  .ext-stamp {
    position: absolute;
    bottom: 14px;
    left: 50%;
    transform: translateX(-50%);
    padding: 0.15rem 0.6rem;
    background: currentColor;
    color: var(--text);
    font-family: var(--font-mono, "IBM Plex Mono", monospace);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    border-radius: 3px;
  }
  .doc-card[data-ext="pdf"] .ext-stamp {
    color: white;
  }
  .progress {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: color-mix(in oklab, var(--content-border) 60%, transparent);
  }
  .progress::after {
    content: "";
    position: absolute;
    inset: 0;
    width: var(--pct, 0%);
    background: var(--accent);
  }
  .meta {
    padding: calc(var(--padding) * 1.2);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .title-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 8px;
  }
  .title {
    color: var(--secondary);
    font-size: 14px;
    font-weight: 500;
    line-height: 1.3;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .star {
    background: transparent;
    border: none;
    color: var(--tertiary);
    font-size: 16px;
    cursor: pointer;
    padding: 0;
    line-height: 1;
  }
  .star.active {
    color: var(--accent);
  }
  .star:focus-visible {
    outline: var(--focus-ring, 2px solid var(--accent));
    outline-offset: 2px;
    border-radius: 3px;
  }
  .sub {
    display: flex;
    gap: 6px;
    color: var(--tertiary);
    font-size: 12px;
  }

  .crumbs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    font-size: 13px;
    color: var(--tertiary);
  }
  .crumb {
    background: transparent;
    border: 0;
    padding: 0.2rem 0.5rem;
    border-radius: var(--border-radius);
    color: var(--tertiary);
    cursor: pointer;
    font-family: inherit;
    font-size: 13px;
    transition:
      color 150ms ease,
      background 150ms ease;
  }
  .crumb:hover {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .crumb-current {
    padding: 0.2rem 0.5rem;
    color: var(--secondary);
    font-weight: 500;
  }
  .crumb-sep {
    opacity: 0.6;
  }

  .drawer-overlay {
    position: fixed;
    inset: 0;
    background: var(--dialog-backdrop, rgba(0, 0, 0, 0.45));
    backdrop-filter: blur(8px) saturate(140%);
    -webkit-backdrop-filter: blur(8px) saturate(140%);
    display: flex;
    justify-content: flex-end;
    z-index: 1100;
    animation: fade-in 180ms ease-out;
  }
  .drawer {
    width: 420px;
    max-width: 100%;
    height: 100%;
    background: var(--button-elevated);
    border-left: 1px solid var(--content-border);
    display: flex;
    flex-direction: column;
    padding: calc(var(--padding) * 2);
    gap: 0.75rem;
    animation: slide-in 260ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .drawer-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .drawer-head h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
    color: var(--secondary);
  }
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: 0;
    color: var(--tertiary);
    cursor: pointer;
    border-radius: var(--border-radius);
    font-family: inherit;
  }
  .icon-btn:hover {
    color: var(--secondary);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .drawer-hint {
    margin: 0;
    font-size: 12px;
    color: var(--tertiary);
    line-height: 1.4;
  }
  .drawer-empty {
    padding: 1rem 0;
  }
  .roots-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    overflow-y: auto;
    flex: 1;
  }
  .root-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.6rem;
    background: var(--input-bg);
    border: 1px solid var(--content-border);
    border-radius: var(--border-radius);
    transition: opacity 150ms ease;
  }
  .root-row.disabled {
    opacity: 0.55;
  }
  .root-toggle input {
    cursor: pointer;
  }
  .root-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .root-path {
    color: var(--secondary);
    font-family: var(--font-mono, "IBM Plex Mono", monospace);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .root-tag {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--tertiary);
  }
  .root-remove {
    width: 22px;
    height: 22px;
    border: 0;
    background: transparent;
    color: var(--tertiary);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    border-radius: 50%;
    font-family: inherit;
  }
  .root-remove:hover {
    color: var(--error);
    background: color-mix(in oklab, var(--error) 10%, transparent);
  }
  .drawer-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .btn {
    padding: 0.5rem 0.85rem;
    border-radius: var(--border-radius);
    background: transparent;
    border: 1px solid var(--content-border);
    color: var(--secondary);
    cursor: pointer;
    font-family: inherit;
    font-size: 13px;
    font-weight: 500;
    transition: border-color 150ms ease;
  }
  .btn:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .btn.primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: var(--accent);
  }
  .btn.primary:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 85%, black);
  }
  .drawer-msg {
    margin: 0;
    font-size: 12px;
    color: var(--tertiary);
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  @keyframes slide-in {
    from {
      transform: translateX(100%);
    }
    to {
      transform: translateX(0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .drawer-overlay,
    .drawer,
    .card {
      animation: none;
      transition: none;
    }
  }

  .health-banner {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border: 1px solid color-mix(in oklab, var(--input-border) 40%, transparent);
    background: color-mix(in oklab, var(--surface, var(--primary)) 60%, transparent);
    border-radius: var(--border-radius);
    color: var(--secondary);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    text-align: left;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .health-banner:hover {
    background: color-mix(in oklab, var(--surface, var(--primary)) 90%, transparent);
    border-color: color-mix(in oklab, var(--input-border) 70%, transparent);
  }
  .health-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--warning, var(--accent));
    flex-shrink: 0;
  }
  .health-text {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }
  .health-text strong {
    font-weight: 500;
  }
  .health-parts {
    color: var(--tertiary);
    font-size: 12px;
  }
  .health-cta {
    color: var(--accent);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }
  .health-count {
    display: inline-block;
    margin-left: 8px;
    padding: 1px 8px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--input-border) 50%, transparent);
    color: var(--secondary);
    font-size: 11px;
    font-weight: 500;
    vertical-align: middle;
  }

  .health-drawer {
    width: min(720px, 90vw);
    max-height: 85vh;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
    border-radius: var(--border-radius);
    padding: calc(var(--padding) * 1.25);
    display: flex;
    flex-direction: column;
    gap: 12px;
    box-shadow: 0 20px 50px color-mix(in oklab, black 35%, transparent);
  }
  .health-drawer .drawer-head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .health-drawer .drawer-head h2 {
    margin: 0;
    flex: 1;
    font-size: 16px;
  }
  .close-btn {
    background: transparent;
    border: 0;
    color: var(--tertiary);
    cursor: pointer;
    font-size: 18px;
    line-height: 1;
    padding: 0 6px;
  }
  .close-btn:hover {
    color: var(--text);
  }
  .health-ok {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin: 0;
    padding: 12px;
    color: var(--secondary);
    font-size: 14px;
  }
  .health-ok-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--success, var(--accent));
  }
  .health-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 8px;
    border-top: 1px solid
      color-mix(in oklab, var(--input-border) 50%, transparent);
  }
  .health-section h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
  }
  .health-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 320px;
    overflow-y: auto;
  }
  .health-list li {
    padding: 6px 8px;
    border-radius: 4px;
    background: color-mix(in oklab, var(--input-border) 18%, transparent);
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .health-list .path {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px;
    color: var(--tertiary);
    overflow-x: auto;
    white-space: nowrap;
  }
  .tg-empty {
    display: flex;
    flex-direction: column;
    gap: var(--padding);
    align-items: center;
    justify-content: center;
    padding: calc(var(--padding) * 4) var(--padding);
    text-align: center;
    min-height: 40vh;
  }
  .downloads-fab {
    position: fixed;
    right: 16px;
    bottom: 16px;
    width: 44px;
    height: 44px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    border: none;
    cursor: pointer;
    font-size: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.18);
    z-index: 49;
  }
  .downloads-fab .badge {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--success);
    color: var(--on-accent);
    font-size: 10px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .downloads-fab.has-active {
    animation: fab-pulse 1.6s ease-in-out infinite;
  }
  @keyframes fab-pulse {
    0%, 100% { box-shadow: 0 4px 16px rgba(0,0,0,0.18); }
    50% { box-shadow: 0 4px 24px color-mix(in oklab, var(--accent) 50%, transparent); }
  }
  @media (prefers-reduced-motion: reduce) {
    .downloads-fab.has-active {
      animation: none;
    }
  }
  .tg-empty h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--secondary);
  }
</style>
