<script lang="ts">
  import { t } from "$lib/i18n";
  import { getSettings } from "$lib/stores/settings-store.svelte";

  interface PowerItem {
    id: string;
    icon: string;
    title: string;
    descKey: string;
    href: string;
    enabled: boolean;
  }

  let rawSettings = $derived(getSettings());
  let settings = $derived(rawSettings as any);

  let powers = $derived<PowerItem[]>([
    {
      id: "music",
      icon: "music",
      title: $t("music.title"),
      descKey: "superpowers.music_desc",
      href: "/music",
      enabled: settings?.music?.enabled ?? true,
    },
    {
      id: "video",
      icon: "video",
      title: $t("video.title"),
      descKey: "superpowers.video_desc",
      href: "/video",
      enabled: settings?.video?.enabled ?? true,
    },
    {
      id: "global",
      icon: "global",
      title: $t("global_dl.title"),
      descKey: "superpowers.global_desc",
      href: "/global",
      enabled: true,
    },
    {
      id: "tube",
      icon: "tube",
      title: $t("tube.title"),
      descKey: "superpowers.tube_desc",
      href: "/youtube",
      enabled: settings?.tube?.enabled ?? true,
    },
    {
      id: "read",
      icon: "read",
      title: $t("read.title"),
      descKey: "superpowers.read_desc",
      href: "/read",
      enabled: settings?.read?.enabled ?? true,
    },
    {
      id: "league",
      icon: "league",
      title: "League of Legends",
      descKey: "superpowers.league_desc",
      href: "/league",
      enabled: settings?.league?.enabled ?? true,
    },
    {
      id: "telegram",
      icon: "telegram",
      title: "Telegram",
      descKey: "superpowers.telegram_desc",
      href: "/telegram",
      enabled: settings?.telegram?.enabled ?? true,
    },
    {
      id: "washmybelly",
      icon: "washmybelly",
      title: "washmybelly",
      descKey: "superpowers.washmybelly_desc",
      href: "/washmybelly",
      enabled: settings?.washmybelly?.enabled ?? true,
    },
  ]);
</script>

<section class="powers">
  <header class="powers-head">
    <div class="nav-art">
      <span class="icon-superpowers">⚡</span>
    </div>
    <div>
      <h1>{$t("nav.superpowers")}</h1>
      <p class="powers-subtitle">{$t("superpowers.subtitle")}</p>
    </div>
  </header>

  <div class="powers-grid">
    {#each powers as power (power.id)}
      <article class="power-card" class:off={!power.enabled}>
        <div class="nav-art">
          <span class="icon-{power.icon}"></span>
        </div>
        <div class="power-copy">
          <h2>{power.title}</h2>
          <p>{$t(power.descKey)}</p>
        </div>
        {#if power.enabled}
          <a class="btn btn-primary power-action" href={power.href}>
            {$t("superpowers.open")}
          </a>
        {:else}
          <p class="power-off">
            <span class="power-off-badge">{$t("superpowers.disabled")}</span>
            {$t("superpowers.disabled_hint")}
          </p>
          <a
            class="btn btn-secondary power-action"
            href={`/settings?tab=integrations&section=${power.id}`}
          >
            {$t("superpowers.open_settings")}
          </a>
        {/if}
      </article>
    {/each}
  </div>
</section>

<style>
  .powers {
    display: flex;
    flex-direction: column;
    gap: var(--space-6, 24px);
    width: 100%;
    max-width: 1040px;
    margin-inline: auto;
    padding: var(--space-4, 16px) var(--space-5, 20px) var(--space-9, 36px);
  }
  .powers-head {
    display: flex;
    align-items: center;
    gap: var(--space-4, 16px);
  }
  .powers-head h1 {
    margin: 0 0 var(--space-1, 4px);
    font-family: var(--font-display, sans-serif);
    font-size: var(--text-2xl, 24px);
    font-weight: 700;
    letter-spacing: var(--track-tight, -0.02em);
    color: var(--text);
  }
  .powers-subtitle {
    margin: 0;
    color: var(--text-dim, var(--gray));
    font-size: var(--text-base, 14px);
  }
  .powers-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: var(--space-4, 16px);
  }
  .power-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3, 12px);
    padding: var(--space-5, 20px);
    border-radius: 14px;
    background: var(--surface, var(--background-secondary));
    box-shadow: inset 0 0 0 var(--hairline, 1px) var(--content-border, rgba(255, 255, 255, 0.1));
  }
  .power-card.off .nav-art {
    filter: grayscale(1);
    opacity: 0.55;
  }
  .power-copy h2 {
    margin: 0 0 var(--space-1, 4px);
    font-size: var(--text-lg, 18px);
    font-weight: 650;
    color: var(--text);
  }
  .power-copy p,
  .power-off {
    margin: 0;
    color: var(--text-dim, var(--gray));
    font-size: var(--text-sm, 13px);
    line-height: 1.45;
  }
  .power-off-badge {
    display: inline-block;
    margin-inline-end: 6px;
    padding: 1px 7px;
    border-radius: var(--radius-full, 9999px);
    background: var(--fill-2, rgba(255, 255, 255, 0.08));
    color: var(--text-muted, var(--gray));
    font-size: var(--text-xs, 11px);
    font-weight: 600;
  }
  .power-action {
    margin-top: auto;
  }
</style>
