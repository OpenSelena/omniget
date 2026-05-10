<script lang="ts">
  type Card = {
    href: string;
    title: string;
    desc: string;
    iconPath: string;
    badge?: string;
  };

  const cards: Card[] = [
    {
      href: "/misc/studio",
      title: "Studio",
      desc: "Record screen + system audio. Replay buffer to clip what just happened.",
      iconPath: "M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2z M12 8a4 4 0 1 1-4 4 4 4 0 0 1 4-4z",
      badge: "BETA",
    },
    {
      href: "/misc/file-clip",
      title: "File clip",
      desc: "Pick a video on disk and extract a portion of it. Output goes to your downloads queue.",
      iconPath: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z M14 2v6h6 M16 13H8 M16 17H8 M10 9H9H8",
    },
    {
      href: "/misc/models",
      title: "Models",
      desc: "Download AI models for transcription, translation, and chat. All run locally — nothing uploaded.",
      iconPath: "M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z M3.27 6.96 12 12.01l8.73-5.05 M12 22.08V12",
      badge: "NEW",
    },
    {
      href: "/misc/library",
      title: "Library",
      desc: "All media you've imported, recorded, or transcribed. Search across titles, transcripts, and notes.",
      iconPath: "M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20",
      badge: "NEW",
    },
  ];
</script>

<section class="misc-hub">
  <header class="head">
    <h1>Misc</h1>
    <p class="subtitle">Utilities and side tools.</p>
  </header>

  <div class="grid">
    {#each cards as c (c.href)}
      <a class="card" href={c.href} aria-label={c.title}>
        <div class="icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d={c.iconPath} />
          </svg>
        </div>
        <div class="meta">
          <div class="title-row">
            <span class="title">{c.title}</span>
            {#if c.badge}
              <span class="badge">{c.badge}</span>
            {/if}
          </div>
          <span class="desc">{c.desc}</span>
        </div>
      </a>
    {/each}
  </div>
</section>

<style>
  .misc-hub {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    width: 100%;
    max-width: 980px;
    margin-inline: auto;
    padding: 16px 20px 80px;
  }

  .head h1 {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 500;
    letter-spacing: -0.5px;
    color: var(--secondary);
  }

  .subtitle {
    margin: 0;
    font-size: 13px;
    color: var(--tertiary);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 12px;
  }

  .card {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 16px 18px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
    text-decoration: none;
    color: inherit;
    transition: border-color 150ms ease, transform 100ms ease, background 150ms ease;
  }

  .card:hover,
  .card:focus-visible {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 5%, var(--surface));
    transform: translateY(-1px);
    outline: none;
  }

  .card:focus-visible {
    box-shadow: 0 0 0 2px color-mix(in oklab, var(--accent) 40%, transparent);
  }

  .icon {
    flex-shrink: 0;
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--accent);
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .title {
    font-size: 14px;
    font-weight: 500;
    color: var(--secondary);
  }

  .badge {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.5px;
    padding: 2px 6px;
    border-radius: 4px;
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    color: var(--accent);
    text-transform: uppercase;
  }

  .desc {
    font-size: 12px;
    color: var(--tertiary);
    line-height: 1.4;
  }

  @media (prefers-reduced-motion: reduce) {
    .card {
      transition: none;
    }
    .card:hover,
    .card:focus-visible {
      transform: none;
    }
  }
</style>
