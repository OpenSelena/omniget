<script lang="ts">
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import { openCommandPalette } from "$lib/stores/command-palette-store.svelte";
  import { shortcut } from "$lib/platform";

  let pageTitle = $derived.by(() => {
    const path = page.url.pathname;
    if (path === "/") return $t("nav.home");
    if (path.startsWith("/downloads")) return $t("nav.downloads");
    if (path.startsWith("/marketplace")) return $t("nav.marketplace");
    if (path.startsWith("/settings")) return $t("nav.settings");
    if (path.startsWith("/about")) return $t("nav.about");
    if (path.startsWith("/omnidisc")) return $t("nav.omnidisc");
    if (path.startsWith("/league")) return $t("league.nav");
    if (path.startsWith("/courses")) return $t("courses.title");
    if (path.startsWith("/convert")) return $t("convert.title");
    if (path.startsWith("/telegram")) return $t("telegram.title");
    if (path.startsWith("/study/music")) return $t("study.hub.music");
    if (path.startsWith("/study")) return $t("study.hub.title");
    if (path.includes("/library")) return $t("study.hub.library");
    return "OmniGet";
  });
</script>

<header class="mac-titlebar" data-tauri-drag-region aria-label={pageTitle}>
  {#if page.url.pathname !== "/"}
    <span class="mac-titlebar-title" data-tauri-drag-region>{pageTitle}</span>
  {:else}
    <span class="mac-titlebar-title mac-titlebar-title--quiet" data-tauri-drag-region></span>
  {/if}
  <div class="mac-titlebar-actions">
    <button
      type="button"
      class="mac-search-field"
      onclick={() => openCommandPalette()}
      aria-label={$t("command_palette.open")}
    >
      <svg class="mac-search-glyph" viewBox="0 0 20 20" width="14" height="14" fill="currentColor" aria-hidden="true">
        <path d="M8.5 2.5a6 6 0 1 0 3.67 10.74l3.8 3.79a1 1 0 0 0 1.41-1.41l-3.79-3.8A6 6 0 0 0 8.5 2.5zM4.5 8.5a4 4 0 1 1 8 0 4 4 0 0 1-8 0z" />
      </svg>
      <span class="mac-search-label">{$t("command_palette.open")}</span>
      <span class="kbd">{shortcut("K")}</span>
    </button>
  </div>
</header>
