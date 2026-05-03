<script lang="ts">
  import {
    studyContinueWatching,
    studyLibraryByType,
    studyCoursesRecentlyAdded,
    studyNotificationsList,
    type ContinueWatchingItem,
    type LibraryByTypeGroup,
    type RecentlyAddedCourse,
    type NotificationFull,
  } from "$lib/study-bridge";
  import CourseCard from "./CourseCard.svelte";
  import Shelf from "./Shelf.svelte";

  const CW_CACHE_KEY = "study.shelves.cw.v1";
  const CW_CACHE_TTL_MS = 30_000;

  let cwItems = $state<ContinueWatchingItem[]>([]);
  let platformGroups = $state<LibraryByTypeGroup[]>([]);
  let subjectGroups = $state<LibraryByTypeGroup[]>([]);
  let recentItems = $state<RecentlyAddedCourse[]>([]);
  let withNewsItems = $state<{ course_id: number; course_title: string; thumbnail: string | null; count: number }[]>([]);
  let cwLoading = $state(true);
  let platformLoading = $state(true);
  let subjectLoading = $state(true);
  let recentLoading = $state(true);
  let newsLoading = $state(true);
  let error = $state<string | null>(null);

  type CachedShelves = { items: ContinueWatchingItem[]; ts: number };

  function readCache(): CachedShelves | null {
    try {
      const raw = localStorage.getItem(CW_CACHE_KEY);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as CachedShelves;
      if (Date.now() - parsed.ts > CW_CACHE_TTL_MS) return null;
      return parsed;
    } catch {
      return null;
    }
  }

  function writeCache(items: ContinueWatchingItem[]) {
    try {
      localStorage.setItem(
        CW_CACHE_KEY,
        JSON.stringify({ items, ts: Date.now() } satisfies CachedShelves),
      );
    } catch {
      void 0;
    }
  }

  async function loadShelves() {
    const cached = readCache();
    if (cached) {
      cwItems = cached.items;
      cwLoading = false;
    }

    const [cwResult, platformResult, subjectResult, recentResult, newsResult] =
      await Promise.allSettled([
        studyContinueWatching(12),
        studyLibraryByType({ grouping: "platform", groupLimit: 8 }),
        studyLibraryByType({ grouping: "subject", groupLimit: 8 }),
        studyCoursesRecentlyAdded({ limit: 8, days: 14 }),
        studyNotificationsList({ unreadOnly: true }),
      ]);

    if (cwResult.status === "fulfilled") {
      cwItems = cwResult.value;
      writeCache(cwResult.value);
    } else if (!cached) {
      error = String(cwResult.reason);
    }
    cwLoading = false;

    if (platformResult.status === "fulfilled") {
      platformGroups = platformResult.value;
    }
    platformLoading = false;

    if (subjectResult.status === "fulfilled") {
      subjectGroups = subjectResult.value;
    }
    subjectLoading = false;

    if (recentResult.status === "fulfilled") {
      recentItems = recentResult.value;
    }
    recentLoading = false;

    if (newsResult.status === "fulfilled") {
      const grouped = new Map<number, { course_id: number; course_title: string; thumbnail: string | null; count: number }>();
      for (const n of newsResult.value as NotificationFull[]) {
        const cur = grouped.get(n.course_id);
        if (cur) cur.count += 1;
        else grouped.set(n.course_id, { course_id: n.course_id, course_title: n.course_title, thumbnail: null, count: 1 });
      }
      withNewsItems = [...grouped.values()].slice(0, 8);
    }
    newsLoading = false;
  }

  $effect(() => {
    void loadShelves();
  });

  let unlistenStateChanged: (() => void) | null = null;

  $effect(() => {
    let cancelled = false;
    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        if (cancelled) return;
        const unlisten = await listen("study:library:state-changed", () => {
          void loadShelves();
        });
        if (cancelled) {
          unlisten();
        } else {
          unlistenStateChanged = unlisten;
        }
      } catch {
        void 0;
      }
    })();
    return () => {
      cancelled = true;
      unlistenStateChanged?.();
      unlistenStateChanged = null;
    };
  });

  const showPlatform = $derived(platformGroups.length >= 2);
  const showSubject = $derived(subjectGroups.length >= 1);
  const cwEmpty = $derived(!cwLoading && cwItems.length === 0);
  const allEmpty = $derived(
    !cwLoading &&
      !platformLoading &&
      !subjectLoading &&
      cwItems.length === 0 &&
      platformGroups.length === 0 &&
      subjectGroups.length === 0,
  );
</script>

{#if error && cwItems.length === 0 && !cwLoading}
  <p class="error" role="alert">Falha ao carregar biblioteca: {error}</p>
{/if}

{#if allEmpty}
  <div class="empty-state">
    <svg viewBox="0 0 64 64" width="64" height="64" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <rect x="10" y="14" width="44" height="36" rx="4" />
      <path d="M22 26h20M22 32h20M22 38h12" />
    </svg>
    <h3>Sua biblioteca está vazia</h3>
    <p>Adicione uma pasta com cursos para começar.</p>
    <a href="/study/library?mode=browse" class="cta">Adicionar pasta com cursos</a>
  </div>
{:else}
  <div class="shelves">
    <Shelf
      title="Continue de onde parou"
      eyebrow="Recente"
      isLoading={cwLoading}
      isEmpty={cwEmpty}
    >
      {#snippet empty()}
        <div class="hint">Nenhum curso em progresso ainda. Comece uma aula para aparecer aqui.</div>
      {/snippet}
      {#each cwItems as item (item.course_id)}
        {@const href =
          item.last_lesson_id != null
            ? `/study/course/${item.course_id}/lesson/${item.last_lesson_id}`
            : `/study/course/${item.course_id}`}
        {@const eyebrowText = item.next_lesson_title
          ? `Próxima: ${item.next_lesson_title}`
          : item.last_lesson_title
            ? `Em: ${item.last_lesson_title}`
            : item.course_title}
        <CourseCard
          courseId={item.course_id}
          title={item.course_title}
          thumbnail={item.course_thumbnail}
          progressPct={item.progress_pct}
          notificationCount={item.notification_count}
          eyebrow={eyebrowText}
          {href}
        />
      {/each}
    </Shelf>

    {#if withNewsItems.length > 0}
      <Shelf title="Com novidades" eyebrow="Aulas novas" isLoading={newsLoading}>
        {#each withNewsItems as item (item.course_id)}
          <CourseCard
            courseId={item.course_id}
            title={item.course_title}
            thumbnail={item.thumbnail}
            notificationCount={item.count}
            eyebrow={`${item.count} ${item.count === 1 ? "aula nova" : "aulas novas"}`}
          />
        {/each}
      </Shelf>
    {/if}

    {#if recentItems.length > 0}
      <Shelf title="Recém-adicionados" eyebrow="Novos" isLoading={recentLoading}>
        {#each recentItems as item (item.id)}
          <CourseCard
            courseId={item.id}
            title={item.title}
            thumbnail={item.thumbnail_path}
            progressPct={item.progress_pct ? item.progress_pct / 100 : 0}
            tags={item.tags}
            eyebrow={item.platform ?? null}
          />
        {/each}
      </Shelf>
    {/if}

    {#if showPlatform}
      {#each platformGroups as group (group.key)}
        <Shelf
          title={prettyPlatform(group.label)}
          eyebrow="Plataforma"
          isLoading={platformLoading}
          isEmpty={group.items.length === 0}
          seeMoreHref={`/study/library?mode=courses&platform=${encodeURIComponent(group.key)}`}
        >
          {#each group.items as course (course.id)}
            <CourseCard
              courseId={course.id}
              title={course.title}
              thumbnail={course.thumbnail_path}
              eyebrow={prettyPlatform(group.label)}
            />
          {/each}
        </Shelf>
      {/each}
    {/if}

    {#if showSubject}
      {#each subjectGroups as group (group.key)}
        <Shelf
          title={group.label}
          eyebrow="Matéria"
          isLoading={subjectLoading}
          isEmpty={group.items.length === 0}
          seeMoreHref={`/study/library?mode=courses&subject=${group.subject_id ?? group.key}`}
        >
          {#each group.items as course (course.id)}
            <CourseCard
              courseId={course.id}
              title={course.title}
              thumbnail={course.thumbnail_path}
              eyebrow={group.label}
            />
          {/each}
        </Shelf>
      {/each}
    {/if}
  </div>
{/if}

<script module lang="ts">
  const PLATFORM_LABELS: Record<string, string> = {
    udemy: "Udemy",
    hotmart: "Hotmart",
    kiwify: "Kiwify",
    gumroad: "Gumroad",
    teachable: "Teachable",
    kajabi: "Kajabi",
    skool: "Skool",
    greatcourses: "The Great Courses",
    thinkific: "Thinkific",
    rocketseat: "Rocketseat",
    generic: "Outras plataformas",
  };

  function prettyPlatform(key: string): string {
    return PLATFORM_LABELS[key.toLowerCase()] ?? key;
  }
</script>

<style>
  .shelves {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
  }

  .error {
    padding: 12px 16px;
    border-radius: 8px;
    background: color-mix(in oklab, var(--error, #dc2626) 12%, transparent);
    color: var(--error, #dc2626);
    font-size: 14px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 64px 24px;
    text-align: center;
  }

  .empty-state svg {
    color: color-mix(in oklab, currentColor 30%, transparent);
  }

  .empty-state h3 {
    font-size: 20px;
    font-weight: 600;
    margin: 0;
  }

  .empty-state p {
    color: color-mix(in oklab, currentColor 60%, transparent);
    margin: 0;
  }

  .empty-state .cta {
    margin-top: 12px;
    padding: 10px 20px;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-contrast, white);
    font-weight: 600;
    text-decoration: none;
  }

  .hint {
    color: color-mix(in oklab, currentColor 60%, transparent);
    font-size: 13px;
    padding: 8px 4px;
  }
</style>
