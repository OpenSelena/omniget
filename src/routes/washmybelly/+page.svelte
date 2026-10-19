<script lang="ts">
  import { onMount } from "svelte";

  interface JunkCategory {
    id: string;
    name: string;
    sizeBytes: number;
    sizeFormatted: string;
    itemsCount: number;
    selected: boolean;
    icon: string;
  }

  // Reactive Svelte 5 state runes
  let healthScore = $state(88);
  let isScanning = $state(false);
  let isCleaning = $state(false);
  let scanProgress = $state(0);
  let scanStep = $state("Idle");
  let bellyRubCount = $state(0);
  let isPurring = $state(false);
  let purrTimeout = $state<number | null>(null);
  let lastRubTime = $state(0);

  // Categories of cleanable items
  let categories = $state<JunkCategory[]>([
    {
      id: "app-cache",
      name: "Application & Browser Cache",
      sizeBytes: 1240000000,
      sizeFormatted: "1.24 GB",
      itemsCount: 4210,
      selected: true,
      icon: "⚡"
    },
    {
      id: "temp-downloads",
      name: "Incomplete Download Chunks & Temp Files",
      sizeBytes: 850000000,
      sizeFormatted: "850 MB",
      itemsCount: 18,
      selected: true,
      icon: "📦"
    },
    {
      id: "orphaned-logs",
      name: "System Crash Logs & Diagnostics",
      sizeBytes: 320000000,
      sizeFormatted: "320 MB",
      itemsCount: 142,
      selected: true,
      icon: "📋"
    },
    {
      id: "empty-folders",
      name: "Empty Folders & Broken Directory Links",
      sizeBytes: 0,
      sizeFormatted: "0 KB",
      itemsCount: 64,
      selected: false,
      icon: "📁"
    }
  ]);

  // Derived calculations
  let totalCleanableBytes = $derived(
    categories.filter(c => c.selected).reduce((acc, c) => acc + c.sizeBytes, 0)
  );

  let totalCleanableFormatted = $derived.by(() => {
    const bytes = totalCleanableBytes;
    if (bytes >= 1024 * 1024 * 1024) {
      return (bytes / (1024 * 1024 * 1024)).toFixed(2) + " GB";
    }
    return (bytes / (1024 * 1024)).toFixed(0) + " MB";
  });

  let mascotMood = $derived.by(() => {
    if (isCleaning) return "Washing bubbles...";
    if (isPurring) return "Purrr! That tickles! (*^ω^*)";
    if (healthScore >= 95) return "Sparkling clean! Maximum speed!";
    if (healthScore >= 80) return "Feeling good, but ready for a scrub!";
    return "MosCat belly needs washing! Too much temporary junk!";
  });

  function rubBelly() {
    bellyRubCount += 1;
    isPurring = true;
    lastRubTime = Date.now();

    if (purrTimeout) clearTimeout(purrTimeout);
    purrTimeout = window.setTimeout(() => {
      isPurring = false;
    }, 2000);
  }

  function startScan() {
    isScanning = true;
    scanProgress = 0;

    const steps = [
      "Auditing WebView2 & browser caches...",
      "Analyzing orphan build artifacts & logs...",
      "Inspecting temporary download fragments...",
      "Validating directory links & duplicates...",
      "Scan complete! Reclaimable space calculated."
    ];

    let currentStepIdx = 0;
    const interval = setInterval(() => {
      if (currentStepIdx < steps.length) {
        scanStep = steps[currentStepIdx];
        scanProgress = Math.min(100, (currentStepIdx + 1) * 20);
        currentStepIdx += 1;
      } else {
        isScanning = false;
        clearInterval(interval);
      }
    }, 450);
  }

  function runClean() {
    isCleaning = true;

    setTimeout(() => {
      // Reclaim space
      categories = categories.map(c => ({
        ...c,
        sizeBytes: c.selected ? 0 : c.sizeBytes,
        sizeFormatted: c.selected ? "0 KB" : c.sizeFormatted,
        itemsCount: c.selected ? 0 : c.itemsCount
      }));
      healthScore = 98;
      isCleaning = false;
      rubBelly();
    }, 1200);
  }
</script>

<svelte:head>
  <title>Wash My Belly - MosCat System Cleaner</title>
</svelte:head>

<div class="wmb-container">
  <!-- Top Navigation -->
  <header class="wmb-header">
    <div class="header-left">
      <a href="/" class="btn-back" title="Back to Dashboard">
        <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
          <path d="M19 12H5M12 19l-7-7 7-7" />
        </svg>
      </a>
      <div>
        <h1 class="header-title">Wash My Belly</h1>
        <span class="header-sub">MosCat System Hygiene & Storage Optimizer</span>
      </div>
    </div>

    <div class="header-actions">
      <button
        type="button"
        class="btn-scan"
        onclick={startScan}
        disabled={isScanning || isCleaning}
      >
        {#if isScanning}
          Scanning...
        {:else}
          Run Full Scan
        {/if}
      </button>

      <button
        type="button"
        class="btn-clean"
        onclick={runClean}
        disabled={isCleaning || isScanning || totalCleanableBytes === 0}
      >
        {#if isCleaning}
          Cleaning...
        {:else}
          Clean ({totalCleanableFormatted})
        {/if}
      </button>
    </div>
  </header>

  <!-- Main Hero Section with Interactive MosCat Mascot -->
  <main class="wmb-content">
    <section class="mascot-hero-card">
      <div class="mascot-stage">
        <!-- Interactive Cat SVG -->
        <div
          class="cat-interactive-wrap"
          class:purring={isPurring}
          onclick={rubBelly}
          role="button"
          tabindex="0"
          onkeydown={(e) => e.key === "Enter" && rubBelly()}
          title="Click to rub MosCat's belly!"
        >
          <svg viewBox="0 0 200 200" width="180" height="180" class="cat-svg">
            <ellipse cx="100" cy="180" rx="68" ry="12" fill="rgba(0,0,0,0.3)" />

            <!-- Body -->
            <ellipse cx="100" cy="116" rx="58" ry="52" fill="#3b82f6" />
            <ellipse cx="100" cy="122" rx="46" ry="40" fill="#60a5fa" />

            <!-- Interactive White Belly Spot -->
            <ellipse cx="100" cy="130" rx="34" ry="28" fill="#eff6ff" class="belly-glow" />

            <!-- Ears -->
            <polygon points="56,72 36,22 82,48" fill="#2563eb" />
            <polygon points="56,68 42,28 78,50" fill="#f472b6" />
            <polygon points="144,72 164,22 118,48" fill="#2563eb" />
            <polygon points="144,68 158,28 122,50" fill="#f472b6" />

            <!-- Head -->
            <circle cx="100" cy="80" r="46" fill="#3b82f6" />

            <!-- Cheeks -->
            <circle cx="70" cy="88" r="7" fill="#f472b6" opacity="0.6" />
            <circle cx="130" cy="88" r="7" fill="#f472b6" opacity="0.6" />

            <!-- Eyes -->
            {#if isPurring}
              <path d="M64,78 Q74,86 84,78" stroke="#1e293b" stroke-width="4" fill="none" stroke-linecap="round" />
              <path d="M116,78 Q126,86 136,78" stroke="#1e293b" stroke-width="4" fill="none" stroke-linecap="round" />
            {:else}
              <ellipse cx="74" cy="76" rx="7" ry="9" fill="#0f172a" />
              <circle cx="72" cy="74" r="3" fill="#fff" />
              <ellipse cx="126" cy="76" rx="7" ry="9" fill="#0f172a" />
              <circle cx="124" cy="74" r="3" fill="#fff" />
            {/if}

            <!-- Nose & Mouth -->
            <polygon points="100,85 95,81 105,81" fill="#f472b6" />
            <path d="M95,88 Q100,93 105,88" stroke="#1e293b" stroke-width="2.5" fill="none" stroke-linecap="round" />

            <!-- Paws -->
            <ellipse cx="72" cy="162" rx="14" ry="10" fill="#eff6ff" />
            <ellipse cx="128" cy="162" rx="14" ry="10" fill="#eff6ff" />

            <!-- Bubbles when cleaning -->
            {#if isCleaning}
              <circle cx="45" cy="50" r="10" fill="rgba(255,255,255,0.4)" stroke="#fff" stroke-width="1.5" class="bubble-float b1" />
              <circle cx="155" cy="60" r="8" fill="rgba(255,255,255,0.4)" stroke="#fff" stroke-width="1.5" class="bubble-float b2" />
              <circle cx="100" cy="30" r="12" fill="rgba(255,255,255,0.4)" stroke="#fff" stroke-width="1.5" class="bubble-float b3" />
            {/if}
          </svg>
        </div>

        <div class="mascot-details">
          <div class="mood-badge" class:happy={healthScore >= 90} class:purring={isPurring}>
            <span>{isPurring ? "💖 Purring" : "🐾 MosCat AI"}</span>
          </div>
          <h2 class="mood-heading">{mascotMood}</h2>
          <p class="rub-hint">
            {#if bellyRubCount === 0}
              Click MosCat's belly to tickle and rub!
            {:else}
              Belly rubbed <strong>{bellyRubCount}</strong> times! Happiness maximum!
            {/if}
          </p>

          <!-- System Health Ring Card -->
          <div class="health-kpi-row">
            <div class="kpi-box">
              <span class="kpi-label">System Health Score</span>
              <strong class="kpi-value" class:high={healthScore >= 90}>{healthScore}%</strong>
            </div>

            <div class="kpi-box">
              <span class="kpi-label">Reclaimable Space</span>
              <strong class="kpi-value cleanable">{totalCleanableFormatted}</strong>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Scan Progress Banner -->
    {#if isScanning}
      <section class="scan-banner">
        <div class="scan-header">
          <span class="scan-step-title">{scanStep}</span>
          <span class="scan-pct">{scanProgress}%</span>
        </div>
        <div class="scan-progress-bar">
          <div class="scan-fill" style:width="{scanProgress}%"></div>
        </div>
      </section>
    {/if}

    <!-- Cleanable Categories Table -->
    <section class="categories-section">
      <h3 class="section-heading">System Clean Targets</h3>

      <div class="categories-grid">
        {#each categories as cat}
          <div
            class="category-card"
            class:selected={cat.selected}
            onclick={() => (cat.selected = !cat.selected)}
          >
            <div class="cat-icon">{cat.icon}</div>
            <div class="cat-content">
              <div class="cat-top">
                <span class="cat-name">{cat.name}</span>
                <input
                  type="checkbox"
                  checked={cat.selected}
                  onclick={(e) => e.stopPropagation()}
                  onchange={() => (cat.selected = !cat.selected)}
                  class="cat-checkbox"
                />
              </div>
              <div class="cat-meta">
                <span class="cat-size">{cat.sizeFormatted}</span>
                <span class="cat-items">{cat.itemsCount} items detected</span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </section>
  </main>
</div>

<style>
  .wmb-container {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: #090d16;
    color: #f3f4f6;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  }

  .wmb-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 28px;
    background: #0f172a;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .btn-back {
    all: unset;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
    cursor: pointer;
  }

  .btn-back:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .header-title {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
  }

  .header-sub {
    font-size: 12px;
    color: #94a3b8;
  }

  .header-actions {
    display: flex;
    gap: 10px;
  }

  .btn-scan {
    all: unset;
    padding: 8px 18px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.08);
    color: #e2e8f0;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-scan:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
  }

  .btn-clean {
    all: unset;
    padding: 8px 20px;
    border-radius: 8px;
    background: #0284c7;
    color: #fff;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }

  .btn-clean:hover:not(:disabled) {
    background: #0369a1;
  }

  .btn-clean:disabled, .btn-scan:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* Content */
  .wmb-content {
    flex: 1;
    padding: 24px 28px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  /* Mascot Hero Card */
  .mascot-hero-card {
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    padding: 24px 32px;
  }

  .mascot-stage {
    display: flex;
    align-items: center;
    gap: 36px;
  }

  .cat-interactive-wrap {
    cursor: pointer;
    transition: transform 0.15s ease;
  }

  .cat-interactive-wrap:hover {
    transform: scale(1.05);
  }

  .cat-interactive-wrap.purring {
    animation: purr 0.3s ease-in-out infinite alternate;
  }

  @keyframes purr {
    from { transform: translateY(0) scale(1.02); }
    to { transform: translateY(-4px) scale(1.05); }
  }

  .belly-glow {
    cursor: pointer;
    transition: fill 0.2s ease;
  }

  .belly-glow:hover {
    fill: #dbeafe;
  }

  .bubble-float {
    animation: floatUp 1.5s ease-out infinite;
  }

  .b1 { animation-delay: 0.1s; }
  .b2 { animation-delay: 0.4s; }
  .b3 { animation-delay: 0.7s; }

  @keyframes floatUp {
    0% { transform: translateY(0); opacity: 0.8; }
    100% { transform: translateY(-20px); opacity: 0; }
  }

  .mascot-details {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .mood-badge {
    align-self: flex-start;
    padding: 4px 10px;
    border-radius: 9999px;
    background: rgba(14, 165, 233, 0.15);
    color: #38bdf8;
    font-size: 11px;
    font-weight: 700;
  }

  .mood-badge.purring {
    background: rgba(236, 72, 153, 0.2);
    color: #f472b6;
  }

  .mood-heading {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
  }

  .rub-hint {
    margin: 0 0 12px;
    font-size: 13px;
    color: #94a3b8;
  }

  .health-kpi-row {
    display: flex;
    gap: 16px;
  }

  .kpi-box {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    padding: 12px 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 140px;
  }

  .kpi-label {
    font-size: 11px;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .kpi-value {
    font-size: 20px;
    font-weight: 700;
  }

  .kpi-value.high {
    color: #10b981;
  }

  .kpi-value.cleanable {
    color: #38bdf8;
  }

  /* Scan Banner */
  .scan-banner {
    background: rgba(14, 165, 233, 0.1);
    border: 1px solid rgba(14, 165, 233, 0.3);
    border-radius: 12px;
    padding: 14px 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .scan-header {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    font-weight: 600;
  }

  .scan-step-title {
    color: #38bdf8;
  }

  .scan-progress-bar {
    width: 100%;
    height: 6px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 9999px;
    overflow: hidden;
  }

  .scan-fill {
    height: 100%;
    background: #0284c7;
    transition: width 0.3s ease;
  }

  /* Categories Section */
  .section-heading {
    margin: 0 0 14px;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
  }

  .categories-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
  }

  .category-card {
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 16px;
    display: flex;
    align-items: flex-start;
    gap: 14px;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .category-card:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .category-card.selected {
    border-color: #0284c7;
    background: rgba(2, 132, 199, 0.08);
  }

  .cat-icon {
    font-size: 24px;
  }

  .cat-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .cat-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .cat-name {
    font-size: 13px;
    font-weight: 600;
  }

  .cat-checkbox {
    accent-color: #0284c7;
    width: 16px;
    height: 16px;
  }

  .cat-meta {
    display: flex;
    gap: 10px;
    font-size: 11px;
    color: #94a3b8;
  }

  .cat-size {
    font-weight: 700;
    color: #38bdf8;
  }
</style>
