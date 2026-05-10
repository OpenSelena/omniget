<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { playlistsStore } from "$lib/study-music/playlists-store.svelte";
  import { musicUI } from "$lib/study-music/ui-store.svelte";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import AboutLink from "./AboutLink.svelte";

  type NavItem = {
    href: string;
    labelKey: string;
    icon: string;
    match: "exact" | "prefix";
  };

  const ITEMS: NavItem[] = [
    {
      href: "/study/music",
      labelKey: "study.music.nav_home",
      icon: "M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z M9 22V12h6v10",
      match: "exact",
    },
    {
      href: "/study/music/search",
      labelKey: "study.music.nav_search",
      icon: "M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z M21 21l-4.35-4.35",
      match: "prefix",
    },
    {
      href: "/study/music/library",
      labelKey: "study.music.nav_library",
      icon: "M4 19.5A2.5 2.5 0 0 1 6.5 17H20 M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z",
      match: "prefix",
    },
    {
      href: "/study/music/albums",
      labelKey: "study.music.nav_albums",
      icon: "M9 18V5l12-2v13 M6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M18 19a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
      match: "prefix",
    },
    {
      href: "/study/music/artists",
      labelKey: "study.music.nav_artists",
      icon: "M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2 M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0z",
      match: "prefix",
    },
    {
      href: "/study/music/playlists",
      labelKey: "study.music.nav_playlists",
      icon: "M3 6h18 M3 12h18 M3 18h12",
      match: "prefix",
    },
    {
      href: "/study/music/genres",
      labelKey: "study.music.nav_genres",
      icon: "M4 4h16v16H4z M4 9h16 M4 15h16 M9 4v16 M15 4v16",
      match: "prefix",
    },
    {
      href: "/study/music/favorites",
      labelKey: "study.music.nav_favorites",
      icon: "M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z",
      match: "prefix",
    },
    {
      href: "/study/music/activity",
      labelKey: "study.music.nav_activity",
      icon: "M3 12h4l3-9 4 18 3-9h4",
      match: "prefix",
    },
    {
      href: "/study/music/download",
      labelKey: "study.music.nav_download",
      icon: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4 M7 10l5 5 5-5 M12 15V3",
      match: "prefix",
    },
    {
      href: "/study/music/youtube",
      labelKey: "study.music.nav_youtube",
      icon: "M22.54 6.42a2.78 2.78 0 0 0-1.94-2C18.88 4 12 4 12 4s-6.88 0-8.6.46a2.78 2.78 0 0 0-1.94 2A29 29 0 0 0 1 11.75a29 29 0 0 0 .46 5.33A2.78 2.78 0 0 0 3.4 19c1.72.46 8.6.46 8.6.46s6.88 0 8.6-.46a2.78 2.78 0 0 0 1.94-2 29 29 0 0 0 .46-5.25 29 29 0 0 0-.46-5.33z M9.75 15.02l5.75-3.27-5.75-3.27v6.54z",
      match: "prefix",
    },
    {
      href: "/study/music/spotify",
      labelKey: "study.music.nav_spotify",
      icon: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z M7.5 14.5c2.5-1.5 6.5-1.8 9.5-.5 M7 11c3-1.8 8-2 11.5-.5 M6.5 7.5c4-2 9.5-2 13 0",
      match: "prefix",
    },
    {
      href: "/study/music/soundcloud",
      labelKey: "study.music.nav_soundcloud",
      icon: "M2 16h2 M5 14h2 M8 13h2 M11 13h2 M14 13c0-2 2-4 5-4s4 2 4 4-1 4-4 4h-9",
      match: "prefix",
    },
  ];

  function isActive(href: string, match: "exact" | "prefix", url: URL): boolean {
    if (match === "exact") return url.pathname === href;
    return url.pathname === href || url.pathname.startsWith(href + "/");
  }

  let creatingPlaylist = $state(false);
  let newPlaylistName = $state("");
  let newPlaylistRef = $state<HTMLInputElement | null>(null);
  let busy = $state(false);

  function startCreate() {
    creatingPlaylist = true;
    newPlaylistName = "";
    queueMicrotask(() => newPlaylistRef?.focus());
  }

  async function confirmCreate() {
    const trimmed = newPlaylistName.trim();
    if (!trimmed || busy) return;
    busy = true;
    try {
      const id = await playlistsStore.create(trimmed);
      creatingPlaylist = false;
      newPlaylistName = "";
      if (id) goto(`/study/music/playlists/${id}`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      showToast("error", msg);
    } finally {
      busy = false;
    }
  }

  function cancelCreate() {
    creatingPlaylist = false;
    newPlaylistName = "";
  }
</script>

<aside class="music-sidebar">
  <header class="brand">
    <span class="brand-text">{$t("study.music.title")}</span>
  </header>

  <nav class="nav" aria-label={$t("study.music.nav_aria") as string}>
    {#each ITEMS as item (item.href)}
      {@const active = isActive(item.href, item.match, $page.url)}
      <a
        href={item.href}
        class="nav-item"
        class:active
        aria-current={active ? "page" : undefined}
      >
        <span class="indicator" aria-hidden="true"></span>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d={item.icon}></path>
        </svg>
        <span class="label">{$t(item.labelKey)}</span>
      </a>
    {/each}
  </nav>

  <div class="divider" aria-hidden="true"></div>

  <div class="playlists-section">
    <header class="ps-head">
      <span class="ps-title">{$t("study.music.playlists_section")}</span>
      <button
        type="button"
        class="ps-add"
        onclick={startCreate}
        disabled={creatingPlaylist}
        aria-label={$t("study.music.new_playlist") as string}
        title={$t("study.music.new_playlist") as string}
      >+</button>
    </header>
    {#if creatingPlaylist}
      <div class="new-row">
        <input
          type="text"
          bind:this={newPlaylistRef}
          bind:value={newPlaylistName}
          placeholder={$t("study.music.playlist_name_placeholder")}
          onkeydown={(e) => {
            if (e.key === "Enter") confirmCreate();
            else if (e.key === "Escape") {
              e.stopPropagation();
              cancelCreate();
            }
          }}
          onblur={() => {
            if (newPlaylistName.trim()) confirmCreate();
            else cancelCreate();
          }}
        />
      </div>
    {/if}
    {#if playlistsStore.list.length === 0 && !creatingPlaylist}
      <p class="ps-empty">{$t("study.music.no_playlists_yet")}</p>
    {:else}
      <ul class="playlist-list">
        {#each playlistsStore.list as p (p.id)}
          {@const href = `/study/music/playlists/${p.id}`}
          {@const active = $page.url.pathname === href}
          <li>
            <a class="playlist-item" class:active {href}>
              <span class="playlist-name">{p.name}</span>
              <span class="playlist-count">{p.track_count}</span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="footer-nav">
    <button
      type="button"
      class="footer-item"
      onclick={() => musicUI.openRoots()}
      title={$t("study.music.manage_folders") as string}
    >
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M3 7v13h18V9h-9l-2-2H3z"/>
        <line x1="12" y1="13" x2="12" y2="17"/>
        <line x1="10" y1="15" x2="14" y2="15"/>
      </svg>
      <span>{$t("study.music.manage_folders")}</span>
    </button>
    <AboutLink variant="footer" />
  </div>
</aside>

<style>
  .music-sidebar {
    display: grid;
    grid-template-rows: auto auto auto 1fr auto;
    gap: 12px;
    padding: 16px 12px;
    height: 100%;
    overflow: hidden;
    background: color-mix(in oklab, var(--primary) 96%, black);
    border-right: 1px solid color-mix(in oklab, var(--content-border) 40%, transparent);
    scrollbar-width: thin;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px 8px;
  }
  .brand-text {
    font-size: 13px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--tertiary);
  }
  .nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 14px 9px 16px;
    color: var(--tertiary);
    font-size: 13px;
    font-weight: 500;
    text-decoration: none;
    border-radius: 6px;
    transition: color 120ms ease, background 120ms ease;
  }
  .nav-item:hover {
    color: var(--secondary);
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .nav-item.active {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 12%, transparent);
  }
  .indicator {
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: transparent;
    transition: background 120ms ease;
  }
  .nav-item.active .indicator {
    background: var(--accent);
  }
  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .divider {
    height: 1px;
    background: color-mix(in oklab, var(--content-border) 50%, transparent);
    margin: 4px 8px;
  }
  .playlists-section {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .ps-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 12px;
  }
  .ps-title {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--tertiary);
  }
  .ps-add {
    width: 22px;
    height: 22px;
    padding: 0;
    background: transparent;
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 50%;
    color: var(--tertiary);
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    transition: color 120ms ease, border-color 120ms ease;
  }
  .ps-add:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
  }
  .ps-add:disabled { opacity: 0.5; cursor: default; }
  .ps-empty {
    margin: 4px 12px 8px;
    color: var(--tertiary);
    font-size: 11px;
    line-height: 1.4;
  }
  .new-row {
    padding: 4px 12px;
  }
  .new-row input {
    width: 100%;
    padding: 6px 10px;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: var(--input-bg);
    color: var(--secondary);
    font-family: inherit;
    font-size: 12px;
    outline: none;
  }
  .playlist-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .playlist-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 14px;
    color: var(--tertiary);
    font-size: 12px;
    text-decoration: none;
    border-radius: 4px;
    transition: color 120ms ease, background 120ms ease;
  }
  .playlist-item:hover {
    color: var(--secondary);
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .playlist-item.active {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 10%, transparent);
  }
  .playlist-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .playlist-count {
    font-size: 10px;
    color: var(--tertiary);
    font-variant-numeric: tabular-nums;
  }
  .footer-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-top: 12px;
    padding-bottom: 4px;
    border-top: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
    background: color-mix(in oklab, var(--primary) 96%, black);
  }
  .footer-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 8px 14px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: var(--tertiary);
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: color 120ms ease, background 120ms ease;
  }
  .footer-item:hover {
    color: var(--secondary);
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
</style>
