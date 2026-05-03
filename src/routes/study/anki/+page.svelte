<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import {
    ankiOpen,
    ankiDashboard,
    type AnkiDashboardState,
    type AnkiDeckSummary,
  } from "$lib/anki-bridge";
  import MascotSlot from "$lib/components/MascotSlot.svelte";

  let dashState = $state<AnkiDashboardState | null>(null);
  let loading = $state(true);
  let error = $state("");

  const decksWithDue = $derived(
    dashState
      ? dashState.deck_summary
          .filter((d: AnkiDeckSummary) => d.due > 0 || d.new_count > 0)
          .slice(0, 8)
      : [],
  );

  const totalToReview = $derived(
    dashState ? dashState.due_today_total + dashState.new_today_total : 0,
  );

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    error = "";
    try {
      await ankiOpen();
      dashState = await ankiDashboard();
    } catch (e: any) {
      error = typeof e === "string" ? e : (e?.message ?? "Erro");
    } finally {
      loading = false;
    }
  }

  function startStudy(deckId?: number) {
    if (deckId !== undefined) {
      goto(`/study/anki/study/${deckId}`);
    } else {
      goto("/study/anki/decks");
    }
  }

  const RING_R = 52;
  const RING_C = 2 * Math.PI * RING_R;

  const streakRatio = $derived(
    dashState ? Math.min(1, dashState.streak.reviewed_today / 50) : 0,
  );
  const streakDashoffset = $derived(RING_C * (1 - streakRatio));
  const streakLabel = $derived(
    dashState
      ? dashState.streak.current >= 7 ? "STREAK" : dashState.streak.current > 0 ? "DIAS" : "COMECE"
      : "COMECE",
  );
</script>

{#if loading}
  <div class="state-row">
    <span class="loader"></span>
  </div>
{:else if error}
  <div class="surface-card">
    <p class="error-text">{error}</p>
    <button type="button" class="btn-cta" onclick={load}>Tentar novamente</button>
  </div>
{:else if dashState}
  <div class="grid">
    <section class="hero surface-card">
      <header class="hero-head">
        <span class="eyebrow">Hoje</span>
        {#if dashState.pending.total > 0}
          <a class="pill pill-cta" href="/study/anki/sync">
            <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
              <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
              <path d="M21 4v4h-4" />
              <path d="M3 20v-4h4" />
            </svg>
            {dashState.pending.total} pendentes
          </a>
        {/if}
      </header>

      <div class="hero-body">
        <div class="hero-text">
          <h1 class="page-title hero-title">{dashState.motd}</h1>
          <p class="hero-counts">
            <strong>{dashState.due_today_total}</strong>
            <span class="dim">due</span>
            <span class="sep">·</span>
            <strong>{dashState.new_today_total}</strong>
            <span class="dim">novos</span>
            <span class="sep">·</span>
            <strong>{dashState.learning_total}</strong>
            <span class="dim">aprendendo</span>
          </p>

          <div class="hero-actions">
            {#if totalToReview > 0}
              <button type="button" class="btn-cta lg" onclick={() => startStudy()}>
                Estudar agora · {totalToReview} cards
              </button>
            {:else}
              <button type="button" class="btn-outline lg" onclick={() => goto("/study/anki/decks")}>
                Abrir decks
              </button>
            {/if}
          </div>
        </div>

        <div class="hero-mascot">
          <MascotSlot
            slug={dashState.pet.slug}
            state={totalToReview === 0 ? "jumping" : "idle"}
            scale={0.75}
            fallback="none"
          />
          <span class="pet-name">{dashState.pet.display_name}</span>
        </div>
      </div>
    </section>

    <aside class="streak-pane">
      <section class="surface-card streak-card">
        <header class="streak-head">
          <h2 class="section-title">Streak</h2>
          <MascotSlot
            slug={dashState.pet.slug}
            state={dashState.streak.current > 0 ? "waving" : "idle"}
            scale={0.4}
            fallback="none"
          />
        </header>

        <div class="streak-ring-wrap">
          <svg viewBox="0 0 120 120" width="140" height="140" class="ring" aria-hidden="true">
            <circle cx="60" cy="60" r={RING_R} fill="none" stroke="var(--surface-mut)" stroke-width="8" />
            <circle
              cx="60"
              cy="60"
              r={RING_R}
              fill="none"
              stroke="var(--accent)"
              stroke-width="8"
              stroke-linecap="round"
              stroke-dasharray={RING_C}
              stroke-dashoffset={streakDashoffset}
              transform="rotate(-90 60 60)"
            />
          </svg>
          <div class="ring-center">
            <span class="ring-current">{dashState.streak.current}</span>
            <span class="ring-label">{streakLabel}</span>
          </div>
        </div>

        <div class="streak-meta">
          <span><strong>{dashState.streak.reviewed_today}</strong> hoje</span>
          <span class="sep">·</span>
          <span><strong>{dashState.streak.longest}</strong> recorde</span>
        </div>

        <ul class="milestones">
          {#each dashState.streak.milestones.slice(0, 4) as m (m.days)}
            <li class:done={m.achieved}>
              <span class="m-mark">{m.achieved ? "✓" : "○"}</span>
              <span class="m-label">{m.label}</span>
            </li>
          {/each}
        </ul>
      </section>
    </aside>

    {#if decksWithDue.length > 0}
      <section class="surface-card decks-block">
        <header class="block-head">
          <h2 class="section-title">Decks pendentes</h2>
          <a class="link" href="/study/anki/decks">Ver todos →</a>
        </header>

        <ul class="deck-list">
          {#each decksWithDue as d (d.deck_id)}
            <li>
              <button type="button" class="deck-row" onclick={() => startStudy(d.deck_id)}>
                <span class="deck-name" style:padding-left="{d.level * 12}px">
                  {d.name}
                </span>
                <span class="deck-pills">
                  {#if d.due > 0}<span class="pill pill-info">{d.due} due</span>{/if}
                  {#if d.new_count > 0}<span class="pill pill-success">{d.new_count} novos</span>{/if}
                  <span class="deck-total">{d.total}</span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {:else}
      <section class="surface-card empty-pane">
        <MascotSlot
          slug={dashState.pet.slug}
          state="jumping"
          scale={1}
          fallback="check"
          fallbackLabel="Tudo zerado"
        />
        <h2 class="empty-title">Tudo zerado</h2>
        <p class="empty-desc">Você revisou todos os cards de hoje. Volte amanhã ou crie novos.</p>
        <button type="button" class="btn-cta" onclick={() => goto("/study/anki/decks")}>
          Abrir decks
        </button>
      </section>
    {/if}
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 280px;
    gap: var(--space-4);
    align-items: start;
  }

  .hero {
    grid-column: 1;
  }

  .streak-pane {
    grid-column: 2;
    grid-row: 1 / span 2;
  }

  .decks-block,
  .empty-pane {
    grid-column: 1;
  }

  .surface-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-5);
  }

  .hero-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-3);
  }

  .eyebrow {
    margin: 0;
  }

  .hero-body {
    display: flex;
    align-items: center;
    gap: var(--space-5);
  }

  .hero-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 0;
  }

  .hero-title {
    margin: 0;
    font-size: var(--text-2xl);
    line-height: var(--leading-2xl);
  }

  .hero-counts {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-base);
  }

  .hero-counts strong {
    color: var(--text);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .dim {
    color: var(--text-dim);
  }

  .sep {
    margin: 0 4px;
    color: var(--text-dim);
    opacity: 0.5;
  }

  .hero-actions {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
    margin-top: var(--space-2);
  }

  .hero-mascot {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
  }

  .pet-name {
    font-size: var(--text-xs);
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    border-radius: var(--radius-full);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-decoration: none;
  }

  .pill-cta {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent);
  }

  .pill-info {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent);
  }

  .pill-success {
    background: color-mix(in srgb, var(--success) 14%, transparent);
    color: var(--success);
  }

  .btn-cta {
    background: var(--accent);
    color: var(--on-accent);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 9px 16px;
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .btn-cta:hover {
    background: var(--accent-lo);
  }

  .btn-cta.lg {
    padding: 12px 22px;
    font-size: var(--text-md);
  }

  .btn-outline {
    background: transparent;
    color: var(--text);
    border: 1px solid var(--border-hi);
    border-radius: var(--radius-sm);
    padding: 9px 16px;
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    transition: border-color var(--duration-fast) var(--ease-out);
  }

  .btn-outline:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .btn-outline.lg {
    padding: 12px 22px;
    font-size: var(--text-md);
  }

  .streak-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .streak-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .streak-head .section-title {
    margin: 0;
  }

  .streak-ring-wrap {
    position: relative;
    width: 140px;
    height: 140px;
    margin: var(--space-2) auto 0;
  }

  .ring {
    position: absolute;
    inset: 0;
  }

  .ring-center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
  }

  .ring-flame {
    font-size: 16px;
    line-height: 1;
  }

  .ring-current {
    font-family: var(--font-display);
    font-size: var(--text-3xl);
    font-weight: 700;
    line-height: 1;
    color: var(--accent);
  }

  .ring-label {
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
  }

  .streak-meta {
    text-align: center;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .streak-meta strong {
    color: var(--text);
    font-weight: 600;
  }

  .milestones {
    list-style: none;
    margin: var(--space-2) 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .milestones li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-dim);
  }

  .milestones li.done {
    color: var(--success);
  }

  .m-mark {
    width: 14px;
    text-align: center;
    font-weight: 700;
  }

  .block-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: var(--space-3);
  }

  .block-head .section-title {
    margin: 0;
  }

  .link {
    color: var(--accent);
    text-decoration: none;
    font-size: var(--text-sm);
  }
  .link:hover {
    text-decoration: underline;
  }

  .deck-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .deck-row {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text);
    font-family: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out);
  }

  .deck-row:hover {
    background: var(--surface-hi);
    border-color: var(--border);
  }

  .deck-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .deck-pills {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .deck-total {
    font-size: var(--text-xs);
    color: var(--text-dim);
    min-width: 36px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .empty-pane {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-7) var(--space-5);
    text-align: center;
  }

  .empty-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 600;
  }

  .empty-desc {
    margin: 0;
    color: var(--text-muted);
    max-width: 360px;
  }

  .state-row {
    display: flex;
    justify-content: center;
    padding: var(--space-7);
  }

  .loader {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 3px solid var(--surface-hi);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-text {
    color: var(--error);
    margin: 0 0 var(--space-3);
  }

  @media (max-width: 1080px) {
    .grid {
      grid-template-columns: minmax(0, 1fr) 240px;
    }
  }

  @media (max-width: 920px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }
    .streak-pane {
      grid-column: 1;
      grid-row: auto;
    }
  }

  @media (max-width: 640px) {
    .hero-body {
      flex-direction: column-reverse;
      align-items: stretch;
    }
    .hero-mascot {
      flex-direction: row;
      justify-content: center;
    }
  }
</style>
