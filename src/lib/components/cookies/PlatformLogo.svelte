<script lang="ts">
  type Kind =
    | "youtube"
    | "youtube_music"
    | "soundcloud"
    | "spotify"
    | "twitch"
    | "instagram"
    | "x_twitter"
    | "vimeo"
    | "tiktok"
    | "bilibili"
    | "douyin"
    | "reddit"
    | "pinterest"
    | "bluesky"
    | "generic";

  type Props = {
    kind: Kind;
    size?: number;
    domain?: string;
  };

  let { kind, size = 56, domain = "" }: Props = $props();

  function genericGradient(seed: string): string {
    let hash = 0;
    for (let i = 0; i < seed.length; i++) {
      hash = (hash << 5) - hash + seed.charCodeAt(i);
      hash |= 0;
    }
    const h1 = Math.abs(hash) % 360;
    const h2 = (h1 + 60) % 360;
    return `linear-gradient(135deg, hsl(${h1} 60% 45%), hsl(${h2} 60% 35%))`;
  }

  let genericInitial = $derived(
    domain ? domain.trim().charAt(0).toUpperCase() || "?" : "?",
  );
  let genericBg = $derived(genericGradient(domain || "x"));
</script>

<div class="logo" style="width: {size}px; height: {size}px;" data-kind={kind}>
  {#if kind === "youtube"}
    <div class="bg yt">
      <svg viewBox="0 0 24 24" width="60%" height="60%" fill="white" aria-hidden="true">
        <path d="M8 5v14l11-7z"/>
      </svg>
    </div>
  {:else if kind === "youtube_music"}
    <div class="bg ytm">
      <svg viewBox="0 0 24 24" width="60%" height="60%" fill="white" aria-hidden="true">
        <circle cx="12" cy="12" r="10" fill="none" stroke="white" stroke-width="2"/>
        <path d="M10 8v8l6-4z"/>
      </svg>
    </div>
  {:else if kind === "soundcloud"}
    <div class="bg sc">
      <svg viewBox="0 0 24 24" width="70%" height="70%" fill="white" aria-hidden="true">
        <path d="M2 14v4M5 12v6M8 10v8M11 8v10M14 6v12M17 8v8M20 10v6"
              stroke="white" stroke-width="1.6" stroke-linecap="round" fill="none"/>
      </svg>
    </div>
  {:else if kind === "spotify"}
    <div class="bg sp">
      <svg viewBox="0 0 24 24" width="65%" height="65%" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" aria-hidden="true">
        <path d="M5 9a14 14 0 0 1 14 1"/>
        <path d="M6 13a11 11 0 0 1 11 1"/>
        <path d="M7 17a8 8 0 0 1 8 0"/>
      </svg>
    </div>
  {:else if kind === "twitch"}
    <div class="bg tw">
      <svg viewBox="0 0 24 24" width="60%" height="60%" fill="white" aria-hidden="true">
        <path d="M4 4v14h4v3l3-3h4l5-5V4H4zm14 8l-3 3h-3l-3 3v-3H6V6h12v6z"/>
      </svg>
    </div>
  {:else if kind === "instagram"}
    <div class="bg ig">
      <svg viewBox="0 0 24 24" width="60%" height="60%" fill="none" stroke="white" stroke-width="2" aria-hidden="true">
        <rect x="3" y="3" width="18" height="18" rx="5"/>
        <circle cx="12" cy="12" r="4"/>
        <circle cx="17.5" cy="6.5" r="1" fill="white"/>
      </svg>
    </div>
  {:else if kind === "x_twitter"}
    <div class="bg x">
      <svg viewBox="0 0 24 24" width="55%" height="55%" fill="white" aria-hidden="true">
        <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/>
      </svg>
    </div>
  {:else if kind === "vimeo"}
    <div class="bg vm">
      <svg viewBox="0 0 24 24" width="60%" height="60%" fill="white" aria-hidden="true">
        <path d="M22 6.4c-.1 2.2-1.6 5.2-4.7 9-3.1 3.9-5.7 5.9-7.9 5.9-1.3 0-2.4-1.2-3.4-3.7C5.4 15.4 5 13 4.6 11c-.5-2.5-1-3.7-1.6-3.7-.1 0-.6.3-1.5.9l-.9-1.2C2.4 5.7 4.1 4 5.5 3.9c1.6-.2 2.6.9 3 3.4.4 2.7.7 4.4.9 5 .5 1.6 1 2.4 1.6 2.4.5 0 1.2-.7 2.1-2.2.9-1.5 1.4-2.6 1.5-3.4.1-.9-.3-1.4-1.1-1.4-.4 0-.8.1-1.2.3.9-2.8 2.5-4.2 5-4.1 1.7.1 2.6 1.3 2.7 3.5z"/>
      </svg>
    </div>
  {:else if kind === "tiktok"}
    <div class="bg tt">
      <svg viewBox="0 0 24 24" width="60%" height="60%" fill="white" aria-hidden="true">
        <path d="M19.59 6.69a4.83 4.83 0 0 1-3.77-4.25V2h-3.45v13.67a2.89 2.89 0 0 1-5.2 1.74 2.89 2.89 0 0 1 2.31-4.64 2.93 2.93 0 0 1 .88.13V9.4a6.84 6.84 0 0 0-1-.05A6.33 6.33 0 0 0 5.8 20.1a6.34 6.34 0 0 0 10.86-4.43V8.43a8.16 8.16 0 0 0 4.77 1.52v-3.4a4.85 4.85 0 0 1-1.84-.86z"/>
      </svg>
    </div>
  {:else if kind === "bilibili"}
    <div class="bg bl">
      <svg viewBox="0 0 24 24" width="60%" height="60%" fill="white" aria-hidden="true">
        <path d="M17.8 6h-1.5l-1.7-1.7c-.4-.4-1-.4-1.4 0s-.4 1 0 1.4l.3.3h-3l.3-.3c.4-.4.4-1 0-1.4s-1-.4-1.4 0L7.7 6H6.2C4.4 6 3 7.4 3 9.2v6.5C3 17.6 4.4 19 6.2 19h11.6c1.8 0 3.2-1.4 3.2-3.2V9.2C21 7.4 19.6 6 17.8 6zM8 14H6v-3h2v3zm10 0h-2v-3h2v3z"/>
      </svg>
    </div>
  {:else if kind === "douyin"}
    <div class="bg dy">
      <svg viewBox="0 0 24 24" width="62%" height="62%" fill="white" aria-hidden="true">
        <path d="M19.59 6.69a4.83 4.83 0 0 1-3.77-4.25V2h-3.45v13.67a2.89 2.89 0 0 1-5.2 1.74 2.89 2.89 0 0 1 2.31-4.64 2.93 2.93 0 0 1 .88.13V9.4a6.84 6.84 0 0 0-1-.05A6.33 6.33 0 0 0 5.8 20.1a6.34 6.34 0 0 0 10.86-4.43V8.43a8.16 8.16 0 0 0 4.77 1.52v-3.4a4.85 4.85 0 0 1-1.84-.86z"/>
      </svg>
    </div>
  {:else if kind === "reddit"}
    <div class="bg rd">
      <svg viewBox="0 0 24 24" width="60%" height="60%" fill="white" aria-hidden="true">
        <circle cx="12" cy="13" r="7" fill="none" stroke="white" stroke-width="1.6"/>
        <circle cx="9" cy="13" r="1.2"/>
        <circle cx="15" cy="13" r="1.2"/>
        <path d="M9 16c1 1 5 1 6 0" stroke="white" stroke-width="1.4" fill="none" stroke-linecap="round"/>
        <circle cx="18.5" cy="6.5" r="1.5"/>
      </svg>
    </div>
  {:else if kind === "pinterest"}
    <div class="bg pn">
      <svg viewBox="0 0 24 24" width="60%" height="60%" fill="white" aria-hidden="true">
        <path d="M12 2C6.5 2 2 6.5 2 12c0 4.2 2.6 7.8 6.3 9.3-.1-.8-.2-2 0-2.9.2-.8 1.1-5.2 1.1-5.2s-.3-.6-.3-1.4c0-1.3.8-2.4 1.7-2.4.8 0 1.2.6 1.2 1.4 0 .8-.5 2.1-.8 3.3-.2 1 .5 1.8 1.5 1.8 1.8 0 3.2-1.9 3.2-4.7 0-2.4-1.7-4.1-4.2-4.1-2.9 0-4.6 2.2-4.6 4.4 0 .9.3 1.8.8 2.3.1.1.1.2.1.3l-.3 1.2c0 .2-.2.2-.4.1-1.3-.6-2.2-2.5-2.2-4 0-3.3 2.4-6.3 6.9-6.3 3.6 0 6.4 2.6 6.4 6 0 3.6-2.3 6.5-5.4 6.5-1.1 0-2.1-.6-2.4-1.2l-.7 2.5c-.2.9-.9 2-1.3 2.7.9.3 1.9.4 2.9.4 5.5 0 10-4.5 10-10S17.5 2 12 2z"/>
      </svg>
    </div>
  {:else if kind === "bluesky"}
    <div class="bg bs">
      <svg viewBox="0 0 24 24" width="65%" height="65%" fill="white" aria-hidden="true">
        <path d="M5.3 3.7c2.8 2.1 5.9 6.4 7 8.7 1.1-2.3 4.2-6.6 7-8.7 2-1.5 5.2-2.7 5.2 1 0 .7-.4 6.1-.7 7-.9 3-3.8 3.7-6.5 3.3 4.7.8 5.9 3.4 3.3 6-4.9 5-7-1.3-7.6-2.9-.1-.3-.2-.4-.2-.3-.1-.1-.1 0-.2.3-.5 1.6-2.7 7.9-7.6 2.9-2.6-2.6-1.3-5.2 3.3-6-2.7.4-5.6-.3-6.5-3.3-.3-.9-.7-6.3-.7-7 0-3.7 3.2-2.5 5.2-1z"/>
      </svg>
    </div>
  {:else}
    <div class="bg generic" style="background: {genericBg};">
      <span class="initial">{genericInitial}</span>
    </div>
  {/if}
</div>

<style>
  .logo {
    flex: 0 0 auto;
    border-radius: 14px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08), inset 0 0 0 1px rgba(255, 255, 255, 0.05);
  }
  .bg {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .yt { background: #FF0000; }
  .ytm { background: #FF0000; }
  .sc { background: #FF5500; }
  .sp { background: #1DB954; }
  .tw { background: #9146FF; }
  .ig { background: linear-gradient(45deg, #f09433 0%, #e6683c 25%, #dc2743 50%, #cc2366 75%, #bc1888 100%); }
  .x { background: #000000; }
  .vm { background: #1AB7EA; }
  .tt { background: linear-gradient(135deg, #25F4EE 0%, #000000 50%, #FE2C55 100%); }
  .bl { background: #00A1D6; }
  .dy { background: linear-gradient(135deg, #25F4EE 0%, #161823 50%, #FE2C55 100%); }
  .rd { background: #FF4500; }
  .pn { background: #E60023; }
  .bs { background: #0085FF; }
  .generic {
    color: white;
  }
  .initial {
    font-size: 50%;
    font-weight: 800;
    color: white;
    letter-spacing: -0.02em;
  }
</style>
