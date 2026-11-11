<script lang="ts">
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import {
    getDownloads,
    getSpeedHistory,
    type DownloadItem,
  } from "$lib/stores/download-store.svelte";
  import { formatBytes, formatSpeed } from "$lib/download-format";

  const downloadsMap = $derived(getDownloads());

  const activeItems = $derived.by(() => {
    const list: DownloadItem[] = [];
    for (const item of downloadsMap.values()) {
      if (item.status === "downloading" || item.status === "seeding") {
        list.push(item);
      }
    }
    return list;
  });

  const visible = $derived(
    activeItems.length > 0 && page.url.pathname !== "/downloads",
  );

  const avgPercent = $derived.by(() => {
    if (activeItems.length === 0) return 0;
    let sum = 0;
    for (const item of activeItems) {
      sum += Math.max(0, Math.min(100, item.percent || 0));
    }
    return sum / activeItems.length;
  });

  const totalSpeed = $derived.by(() => {
    let bps = 0;
    for (const item of activeItems) {
      bps += item.speed || 0;
    }
    return bps;
  });

  const bytesSummary = $derived.by(() => {
    let downloaded = 0;
    let total = 0;
    let allKnown = activeItems.length > 0;
    for (const item of activeItems) {
      if (item.kind === "generic") {
        downloaded += item.downloadedBytes || 0;
        if (item.totalBytes && item.totalBytes > 0) {
          total += item.totalBytes;
        } else {
          allKnown = false;
        }
      } else {
        downloaded += item.bytesDownloaded || 0;
        allKnown = false;
      }
    }
    if (allKnown && total > 0) {
      return `${formatBytes(downloaded)} / ${formatBytes(total)}`;
    }
    return `${formatBytes(downloaded)}`;
  });

  const sparklinePaths = $derived.by(() => {
    const primary = activeItems[0];
    if (!primary) return { line: "", area: "" };
    const pts = getSpeedHistory(primary.id);
    const w = 64;
    const h = 18;
    if (pts.length < 2) {
      const y = h - 2;
      return {
        line: `M0,${y} L${w},${y}`,
        area: `M0,${h} L0,${y} L${w},${y} L${w},${h} Z`,
      };
    }
    let maxBps = 1024;
    for (const p of pts) {
      if (p.bps > maxBps) maxBps = p.bps;
    }
    const coords = pts.map((p, idx) => {
      const x = (idx / (pts.length - 1)) * w;
      const ratio = Math.min(1, Math.max(0, p.bps / maxBps));
      const y = h - 2 - ratio * (h - 4);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    });
    const line = `M${coords.join(" L")}`;
    const area = `M0,${h} L${coords.join(" L")} L${w},${h} Z`;
    return { line, area };
  });
</script>

{#if visible}
  <div class="global-download-dock" role="status" aria-live="polite">
    <div class="dock-progress-track" aria-hidden="true">
      <div class="dock-progress-fill" style="width: {avgPercent.toFixed(1)}%"></div>
    </div>

    <div class="dock-left">
      <span class="dock-badge" aria-hidden="true">
        <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M8 2.5v7.5M4.75 7.25 8 10.5l3.25-3.25M3 13h10" />
        </svg>
      </span>
      <span class="dock-status-label">
        {activeItems.length} {$t('downloads.phase_downloading').toLowerCase()}
      </span>
    </div>

    <div class="dock-right">
      <span class="dock-metrics">
        <strong>{Math.round(avgPercent)}%</strong>
        <span class="dock-sep">&middot;</span>
        <span>{bytesSummary}</span>
        <span class="dock-speed">{formatSpeed(totalSpeed)}</span>
      </span>

      <svg
        class="dock-sparkline"
        viewBox="0 0 64 18"
        width="64"
        height="18"
        aria-hidden="true"
      >
        <path class="dock-sparkline-area" d={sparklinePaths.area} />
        <path class="dock-sparkline-line" d={sparklinePaths.line} />
      </svg>

      <button
        type="button"
        class="dock-view-btn"
        onclick={() => goto("/downloads")}
      >
        {$t('downloads.view_queue')}
      </button>
    </div>
  </div>
{/if}

<style>
  .global-download-dock {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    height: 44px;
    padding: 0 var(--space-4);
    background: var(--surface);
    border-top: var(--hairline) solid var(--separator);
    flex-shrink: 0;
    z-index: 25;
  }

  .dock-progress-track {
    position: absolute;
    top: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: transparent;
    overflow: hidden;
    pointer-events: none;
  }

  .dock-progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width var(--duration-base) var(--ease-out);
  }

  .dock-left {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .dock-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-hi);
    flex-shrink: 0;
  }

  .dock-status-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
  }

  .dock-right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }

  .dock-metrics {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .dock-metrics strong {
    color: var(--text);
    font-weight: 600;
  }

  .dock-sep {
    color: var(--text-dim);
  }

  .dock-speed {
    margin-left: var(--space-1);
    color: var(--text-dim);
  }

  .dock-sparkline {
    flex-shrink: 0;
    overflow: visible;
  }

  .dock-sparkline-area {
    fill: var(--accent-soft);
  }

  .dock-sparkline-line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .dock-view-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: var(--control-h);
    padding: 0 var(--space-3);
    border: none;
    border-radius: var(--radius-sm);
    background: var(--fill-1);
    box-shadow: inset 0 0 0 var(--hairline) var(--content-border);
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    transition: background var(--duration-fast) var(--ease-out);
  }

  @media (hover: hover) {
    .dock-view-btn:hover {
      background: var(--fill-2);
    }
  }

  .dock-view-btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }
</style>
