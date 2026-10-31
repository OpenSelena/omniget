<script lang="ts">
  /**
   * Sidebar icon tile supporting both custom MosCat character illustrations
   * and classic macOS System Settings-style gradient tiles for dynamic plugins.
   */
  let {
    icon,
    iconSvg,
    size = 22,
    active = false,
  }: { icon: string; iconSvg?: string; size?: number; active?: boolean } = $props();

  const MOSCAT_ICONS: Record<string, string> = {
    home: "/icons/nav/home.svg",
    downloads: "/icons/nav/downloads.svg",
    tools: "/icons/nav/tools.svg",
    "open-nami": "/icons/nav/open-nami.svg",
    marketplace: "/icons/nav/marketplace.svg",
    convert: "/icons/nav/convert.svg",
    settings: "/emoji/gear.png",
    about: "/icons/about-64.png",
    courses: "/emoji/books.png",
    telegram: "/icons/telegram-64.png",
  };

  const TILES: Record<string, { glyph: string; from: string; to: string }> = {
    home: { glyph: "house", from: "#5AA9FF", to: "#1E6FE8" },
    downloads: { glyph: "tray-arrow-down", from: "#FFB340", to: "#F28500" },
    chat: { glyph: "chats-circle", from: "#4CD964", to: "#2AA845" },
    marketplace: { glyph: "storefront", from: "#6E8CFF", to: "#3D5BF0" },
    settings: { glyph: "gear-six", from: "#A3A3A8", to: "#6F6F75" },
    about: { glyph: "info", from: "#5AA9FF", to: "#1E6FE8" },
    league: { glyph: "sword", from: "#E8B84A", to: "#B8860B" },
    courses: { glyph: "graduation-cap", from: "#C77DFF", to: "#8E3FD8" },
    telegram: { glyph: "paper-plane-tilt", from: "#55C2FF", to: "#1F8FE0" },
    convert: { glyph: "arrows-clockwise", from: "#FF7A7A", to: "#E33A3A" },
    misc: { glyph: "wrench", from: "#9B9BA3", to: "#63636B" },
    tools: { glyph: "toolbox", from: "#FF9F5A", to: "#E8641A" },
    "open-nami": { glyph: "cloud-arrow-down", from: "#FF6584", to: "#D23B68" },
    plugin: { glyph: "puzzle-piece", from: "#8E8E93", to: "#5C5C60" },
  };

  let moscatIcon = $derived(MOSCAT_ICONS[icon]);
  let tile = $derived(TILES[icon] ?? TILES.plugin);
  let glyphSize = $derived(Math.round(size * 0.64));
</script>

{#if iconSvg}
  <span
    class="nav-icon nav-tile"
    class:nav-icon-active={active}
    style:--tile-from={tile.from}
    style:--tile-to={tile.to}
    style:--tile-size="{size}px"
    aria-hidden="true"
  >
    <svg viewBox="0 0 24 24" width={glyphSize} height={glyphSize} fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
      {#each iconSvg.split(" M").map((d, i) => (i === 0 ? d : "M" + d)) as pathD}
        <path d={pathD} />
      {/each}
    </svg>
  </span>
{:else if moscatIcon}
  <span
    class="nav-icon nav-moscat"
    class:nav-icon-active={active}
    style:--icon-size="{size}px"
    aria-hidden="true"
  >
    <img
      src={moscatIcon}
      alt=""
      width={size}
      height={size}
      class="nav-moscat-img"
      draggable="false"
    />
  </span>
{:else}
  <span
    class="nav-icon nav-tile"
    class:nav-icon-active={active}
    style:--tile-from={tile.from}
    style:--tile-to={tile.to}
    style:--tile-size="{size}px"
    aria-hidden="true"
  >
    <span class="nav-glyph" style:--glyph="url(/icons/{tile.glyph}.svg)" style:width="{glyphSize}px" style:height="{glyphSize}px"></span>
  </span>
{/if}

<style>
  .nav-tile {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--tile-size);
    height: var(--tile-size);
    flex-shrink: 0;
    border-radius: calc(var(--tile-size) * 0.28);
    background: linear-gradient(180deg, var(--tile-from), var(--tile-to));
    box-shadow:
      inset 0 0 0 0.5px rgba(255, 255, 255, 0.25),
      inset 0 -1px 1px rgba(0, 0, 0, 0.12),
      0 0.5px 1px rgba(0, 0, 0, 0.25);
    color: #fff;
  }

  .nav-glyph {
    display: block;
    background: #fff;
    -webkit-mask: var(--glyph) center / contain no-repeat;
    mask: var(--glyph) center / contain no-repeat;
    filter: drop-shadow(0 0.5px 0 rgba(0, 0, 0, 0.18));
  }

  .nav-moscat {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--icon-size);
    height: var(--icon-size);
    flex-shrink: 0;
  }

  .nav-moscat-img {
    width: var(--icon-size);
    height: var(--icon-size);
    object-fit: contain;
    display: block;
    border-radius: 4px;
    filter: drop-shadow(0 0.5px 1px rgba(0, 0, 0, 0.18));
    transition: transform 0.15s ease-out;
  }

  .nav-icon-active .nav-moscat-img {
    transform: scale(1.08);
  }
</style>
