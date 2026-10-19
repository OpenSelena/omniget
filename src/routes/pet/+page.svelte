<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  // Reactive Svelte 5 state runes
  let mood = $state<"happy" | "sleepy" | "curious" | "purring" | "alert">("happy");
  let isCompact = $state(false);
  let showMenu = $state(false);
  let speechBubble = $state<string | null>("Meow! I'm MosCat, your desktop AI companion.");
  let speechTimeout = $state<number | null>(null);

  // Permission request bubble
  let pendingPermission = $state<{ tool: string; prompt: string } | null>(null);

  // Dragging state
  let isDragging = $state(false);
  let petX = $state(40);
  let petY = $state(40);
  let startX = $state(0);
  let startY = $state(0);

  // Animation cycle
  let blinkState = $state(false);
  let earTwitch = $state(false);
  let purrCount = $state(0);

  function say(text: string, duration = 4000) {
    speechBubble = text;
    if (speechTimeout) clearTimeout(speechTimeout);
    speechTimeout = window.setTimeout(() => {
      speechBubble = null;
    }, duration);
  }

  function petCat() {
    purrCount += 1;
    mood = "purring";
    if (purrCount % 5 === 0) {
      say("Purrrrrr... You rubbed my belly! 🐾", 3500);
    } else {
      say("Purrrr! (*^ω^*)", 2500);
    }

    setTimeout(() => {
      if (mood === "purring") mood = "happy";
    }, 4000);
  }

  function feedCat() {
    mood = "happy";
    say("Nom nom nom! Fish treats are the best! 🐟", 3500);
    showMenu = false;
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.button !== 0) return; // Only left click
    isDragging = true;
    startX = e.clientX - petX;
    startY = e.clientY - petY;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    petX = Math.max(0, Math.min(window.innerWidth - 180, e.clientX - startX));
    petY = Math.max(0, Math.min(window.innerHeight - 180, e.clientY - startY));
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    showMenu = !showMenu;
  }

  function allowPermission() {
    if (!pendingPermission) return;
    say(`Authorized ${pendingPermission.tool} ✅`, 3000);
    pendingPermission = null;
  }

  function denyPermission() {
    if (!pendingPermission) return;
    say(`Blocked ${pendingPermission.tool} 🛑`, 3000);
    pendingPermission = null;
  }

  onMount(() => {
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    // Occasional blinking animation loop
    const blinkInterval = setInterval(() => {
      blinkState = true;
      setTimeout(() => (blinkState = false), 180);
    }, 3800);

    // Random thoughts
    const thoughtInterval = setInterval(() => {
      if (!speechBubble && !pendingPermission) {
        const thoughts = [
          "OmniGet v1.0.0 is running with native Rust speed!",
          "All systems clean and purring smoothly.",
          "Need a belly wash? Head over to /washmybelly!",
          "Monitoring downloads and background queues.",
          "Zzz... (MosCat takes a quick catnap)"
        ];
        say(thoughts[Math.floor(Math.random() * thoughts.length)], 3500);
      }
    }, 18000);

    return () => {
      clearInterval(blinkInterval);
      clearInterval(thoughtInterval);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
      if (speechTimeout) clearTimeout(speechTimeout);
    };
  });
</script>

<svelte:head>
  <title>MosCat Desktop Companion</title>
</svelte:head>

<div
  class="pet-overlay"
  class:compact={isCompact}
  style:left="{petX}px"
  style:top="{petY}px"
  onmousedown={handleMouseDown}
  oncontextmenu={handleContextMenu}
  role="region"
  aria-label="MosCat Interactive Mascot"
>
  <!-- Thought / Message Bubble -->
  {#if speechBubble && !pendingPermission}
    <div class="speech-bubble">
      <span>{speechBubble}</span>
    </div>
  {/if}

  <!-- Tool Permission Request Bubble -->
  {#if pendingPermission}
    <div class="permission-bubble">
      <span class="bubble-tag">Tool Authorization</span>
      <strong class="bubble-tool">{pendingPermission.tool}</strong>
      <p class="bubble-prompt">{pendingPermission.prompt}</p>
      <div class="bubble-actions">
        <button type="button" class="btn-allow" onclick={allowPermission}>Allow</button>
        <button type="button" class="btn-deny" onclick={denyPermission}>Deny</button>
      </div>
    </div>
  {/if}

  <!-- MosCat Character SVG Illustration -->
  <div class="cat-sprite" class:purring={mood === "purring"} onclick={petCat} role="button" tabindex="0" onkeydown={(e) => e.key === "Enter" && petCat()}>
    <svg viewBox="0 0 160 160" width="140" height="140" class="cat-svg">
      <!-- Mascot Shadow -->
      <ellipse cx="80" cy="148" rx="55" ry="10" fill="rgba(0,0,0,0.3)" />

      <!-- Body -->
      <ellipse cx="80" cy="100" rx="46" ry="42" fill="#3b82f6" />
      <ellipse cx="80" cy="105" rx="36" ry="32" fill="#60a5fa" />

      <!-- Belly (Rub spot) -->
      <ellipse cx="80" cy="112" rx="26" ry="22" fill="#eff6ff" class="belly-spot" />

      <!-- Left Ear -->
      <polygon points="46,62 30,22 66,42" fill="#2563eb" />
      <polygon points="46,58 35,28 62,44" fill="#f472b6" />

      <!-- Right Ear -->
      <polygon points="114,62 130,22 94,42" fill="#2563eb" />
      <polygon points="114,58 125,28 98,44" fill="#f472b6" />

      <!-- Head -->
      <circle cx="80" cy="68" r="38" fill="#3b82f6" />

      <!-- Cheeks -->
      <circle cx="56" cy="74" r="6" fill="#f472b6" opacity="0.6" />
      <circle cx="104" cy="74" r="6" fill="#f472b6" opacity="0.6" />

      <!-- Eyes -->
      {#if blinkState || mood === "purring" || mood === "sleepy"}
        <path d="M52,66 Q60,72 68,66" stroke="#1e293b" stroke-width="3.5" fill="none" stroke-linecap="round" />
        <path d="M92,66 Q100,72 108,66" stroke="#1e293b" stroke-width="3.5" fill="none" stroke-linecap="round" />
      {:else}
        <ellipse cx="60" cy="64" rx="6" ry="7" fill="#0f172a" />
        <circle cx="58" cy="62" r="2.5" fill="#fff" />
        <ellipse cx="100" cy="64" rx="6" ry="7" fill="#0f172a" />
        <circle cx="98" cy="62" r="2.5" fill="#fff" />
      {/if}

      <!-- Nose & Mouth -->
      <polygon points="80,72 76,69 84,69" fill="#f472b6" />
      <path d="M76,74 Q80,78 84,74" stroke="#1e293b" stroke-width="2" fill="none" stroke-linecap="round" />

      <!-- Whiskers -->
      <line x1="42" y1="70" x2="22" y2="67" stroke="#93c5fd" stroke-width="1.8" stroke-linecap="round" />
      <line x1="42" y1="75" x2="20" y2="76" stroke="#93c5fd" stroke-width="1.8" stroke-linecap="round" />
      <line x1="118" y1="70" x2="138" y2="67" stroke="#93c5fd" stroke-width="1.8" stroke-linecap="round" />
      <line x1="118" y1="75" x2="140" y2="76" stroke="#93c5fd" stroke-width="1.8" stroke-linecap="round" />

      <!-- Paws -->
      <ellipse cx="58" cy="136" rx="12" ry="8" fill="#eff6ff" />
      <ellipse cx="102" cy="136" rx="12" ry="8" fill="#eff6ff" />

      <!-- Tail -->
      <path d="M120,118 Q145,110 142,88 Q140,80 134,84 Q130,88 132,102 Q130,118 116,124" fill="#2563eb" class="tail-anim" />
    </svg>
  </div>

  <!-- Right-Click Context Menu -->
  {#if showMenu}
    <div class="cat-menu" onclick={(e) => e.stopPropagation()}>
      <div class="menu-head">
        <strong>MosCat v1.0.0</strong>
        <span>Status: {mood}</span>
      </div>
      <button type="button" class="menu-item" onclick={feedCat}>
        🐟 Feed Fish Treat
      </button>
      <button type="button" class="menu-item" onclick={petCat}>
        🐾 Rub Belly
      </button>
      <a href="/washmybelly" class="menu-item link">
        🛁 Open Wash My Belly
      </a>
      <button type="button" class="menu-item" onclick={() => { isCompact = !isCompact; showMenu = false; }}>
        {isCompact ? "Standard Size" : "Compact Mode"}
      </button>
      <button type="button" class="menu-item danger" onclick={() => (showMenu = false)}>
        Dismiss
      </button>
    </div>
  {/if}
</div>

<style>
  :global(body) {
    background: transparent !important;
    margin: 0;
    overflow: hidden;
  }

  .pet-overlay {
    position: fixed;
    width: 180px;
    height: 180px;
    z-index: 99999;
    user-select: none;
    -webkit-user-select: none;
    cursor: grab;
    display: flex;
    flex-direction: column;
    align-items: center;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  .pet-overlay:active {
    cursor: grabbing;
  }

  .pet-overlay.compact {
    transform: scale(0.7);
  }

  .speech-bubble {
    position: absolute;
    top: -46px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(15, 23, 42, 0.92);
    backdrop-filter: blur(8px);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    color: #fff;
    padding: 6px 14px;
    border-radius: 9999px;
    font-size: 11px;
    font-weight: 500;
    white-space: nowrap;
    pointer-events: none;
    animation: popIn 0.25s ease-out;
  }

  .speech-bubble::after {
    content: "";
    position: absolute;
    bottom: -5px;
    left: 50%;
    transform: translateX(-50%) rotate(45deg);
    width: 8px;
    height: 8px;
    background: rgba(15, 23, 42, 0.92);
    border-right: 1px solid rgba(255, 255, 255, 0.15);
    border-bottom: 1px solid rgba(255, 255, 255, 0.15);
  }

  .permission-bubble {
    position: absolute;
    top: -95px;
    width: 200px;
    background: #0f172a;
    border: 1px solid rgba(14, 165, 233, 0.4);
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.6);
    border-radius: 10px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    z-index: 100;
  }

  .bubble-tag {
    font-size: 9px;
    text-transform: uppercase;
    color: #38bdf8;
    font-weight: 700;
  }

  .bubble-tool {
    font-size: 11px;
    color: #fff;
    font-family: monospace;
  }

  .bubble-prompt {
    margin: 0;
    font-size: 10px;
    color: #94a3b8;
    line-height: 1.3;
  }

  .bubble-actions {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }

  .btn-allow {
    all: unset;
    flex: 1;
    text-align: center;
    background: #10b981;
    color: #fff;
    padding: 3px 0;
    border-radius: 4px;
    font-weight: 700;
    font-size: 10px;
    cursor: pointer;
  }

  .btn-deny {
    all: unset;
    flex: 1;
    text-align: center;
    background: rgba(255, 255, 255, 0.1);
    color: #e2e8f0;
    padding: 3px 0;
    border-radius: 4px;
    font-weight: 600;
    font-size: 10px;
    cursor: pointer;
  }

  .cat-sprite {
    cursor: pointer;
    transition: transform 0.1s ease;
  }

  .cat-sprite:hover {
    transform: scale(1.04);
  }

  .cat-sprite:active {
    transform: scale(0.96);
  }

  .purring {
    animation: purrBounce 0.4s ease-in-out infinite alternate;
  }

  @keyframes purrBounce {
    from { transform: translateY(0); }
    to { transform: translateY(-4px); }
  }

  @keyframes popIn {
    from { opacity: 0; transform: translateX(-50%) scale(0.85); }
    to { opacity: 1; transform: translateX(-50%) scale(1); }
  }

  .tail-anim {
    transform-origin: 120px 120px;
    animation: tailWag 3.2s ease-in-out infinite alternate;
  }

  @keyframes tailWag {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(15deg); }
  }

  .belly-spot {
    cursor: pointer;
    transition: fill 0.2s ease;
  }

  .belly-spot:hover {
    fill: #dbeafe;
  }

  /* Menu */
  .cat-menu {
    position: absolute;
    bottom: -10px;
    right: -130px;
    width: 150px;
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 10px;
    padding: 6px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    gap: 3px;
    z-index: 1000;
  }

  .menu-head {
    display: flex;
    flex-direction: column;
    padding: 4px 6px 6px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 11px;
  }

  .menu-head strong {
    color: #38bdf8;
  }

  .menu-head span {
    color: #94a3b8;
    font-size: 10px;
  }

  .menu-item {
    all: unset;
    padding: 6px 8px;
    border-radius: 6px;
    font-size: 11px;
    color: #e2e8f0;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .menu-item:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .menu-item.link {
    display: block;
    box-sizing: border-box;
  }

  .menu-item.danger {
    color: #f87171;
  }
</style>
