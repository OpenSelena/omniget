<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { invokeV100 } from "$lib/v100/ipc-bridge";

  interface WindowLimit {
    id: string;
    label: string;
    group?: string;
    used: number | null;
    limit_abs?: number;
    used_abs?: number;
    unit?: string;
    resets_at?: number;
    delta?: number;
    runsOutAt?: number;
  }

  interface LocalModel {
    name: string;
    params?: string;
    quant?: string;
    vram_bytes?: number;
    size_bytes?: number;
    expires_at?: number;
  }

  interface RingItem {
    id: string;
    label: string;
    status: "ok" | "needs_auth" | "error" | "rate_limited" | "absent";
    activity: "idle" | "working" | "waiting" | "done";
    message?: string;
    beta?: boolean;
    local?: boolean;
    account?: {
      label?: string;
      email?: string;
    };
    read_at?: number;
    reading?: {
      plan?: string;
      account?: string;
      note?: string;
      windows?: WindowLimit[];
      local_models?: LocalModel[];
    };
  }

  interface StripLook {
    size?: string;
    scale?: number;
    show_percent?: boolean;
    show_pace?: boolean;
    contrast?: boolean;
  }

  interface StripState {
    open: boolean;
    edge: "top" | "bottom" | "left" | "right";
    look: StripLook;
    rings: RingItem[];
  }

  const defaultLook: StripLook = {
    size: "m",
    scale: 1,
    show_percent: true,
    show_pace: true,
    contrast: false
  };

  let dockState = $state<StripState>({
    open: true,
    edge: "top",
    look: defaultLook,
    rings: []
  });

  let selectedId = $state<string | null>(null);
  let isLoading = $state(true);
  let hasError = $state(false);
  let isOpeningApp = $state(false);
  let animTick = $state(0);
  let now = $state(Date.now());
  let chimeProvider = $state<string | null>(null);
  let isSolid = $state(false);
  let theme = $state<"dark" | "light">("dark");
  let isAsleep = $state(false);
  let showEmptyTip = $state(false);

  const isVertical = $derived(dockState.edge === "left" || dockState.edge === "right");
  const look = $derived(dockState.look ?? defaultLook);
  const zoomScale = $derived(Number.isFinite(look.scale) && (look.scale ?? 1) > 0 ? (look.scale ?? 1) : 1);
  const activeRing = $derived(dockState.rings.find((r: RingItem) => r.id === selectedId) ?? null);

  const groupedToolRings = $derived.by(() => {
    const groups: { tool: string; rings: RingItem[] }[] = [];
    for (const r of dockState.rings) {
      const tool = r.id.split(":")[0];
      const last = groups[groups.length - 1];
      if (last && last.tool === tool) {
        last.rings.push(r);
      } else {
        groups.push({ tool, rings: [r] });
      }
    }
    return groups;
  });

  const criticalRingId = $derived.by(() => {
    let top: { id: string; used: number } | null = null;
    for (const r of dockState.rings) {
      const maxUsed = getRingMaxUsed(r);
      if (maxUsed != null && maxUsed >= 0.8 && (!top || maxUsed > top.used)) {
        top = { id: r.id, used: maxUsed };
      }
    }
    return top?.id ?? null;
  });

  const accountSiblings = $derived.by(() => {
    if (!activeRing) return [];
    const toolPrefix = activeRing.id.split(":")[0];
    return dockState.rings.filter((r: RingItem) => r.id.split(":")[0] === toolPrefix);
  });

  function getTone(val: number | null | undefined): "calm" | "amber" | "red" {
    if (val == null || val < 0.8) return "calm";
    if (val < 0.95) return "amber";
    return "red";
  }

  function getRingMaxUsed(ring: RingItem): number | null {
    const usedVals = (ring.reading?.windows ?? [])
      .map((w) => w.used)
      .filter((u): u is number => u != null);
    return usedVals.length ? Math.max(...usedVals) : null;
  }

  function getRingWindows(ring: RingItem): { outer: number | null; inner: number | null } {
    const valid = (ring.reading?.windows ?? []).filter((w) => w.used != null);
    if (!valid.length) return { outer: null, inner: null };
    if (valid.length === 1) return { outer: valid[0].used, inner: null };
    const rest = valid.slice(1).map((w) => w.used!);
    return { outer: valid[0].used, inner: Math.max(...rest) };
  }

  function formatDuration(ms: number): string {
    const totalMin = Math.max(1, Math.round(ms / 60000));
    const days = Math.floor(totalMin / 1440);
    const hours = Math.floor((totalMin % 1440) / 60);
    const mins = totalMin % 60;
    const parts: string[] = [];
    if (days) parts.push(`${days}d`);
    if (hours) parts.push(`${hours}h`);
    if (mins && !days) parts.push(`${mins}m`);
    return parts.join(" ") || "0m";
  }

  function formatReset(resetsAt: number | undefined): string | null {
    if (resetsAt == null) return null;
    if (resetsAt <= now) return "resets now";
    return `resets in ${formatDuration(resetsAt - now)}`;
  }

  function formatUsage(val: number, unit?: string): string {
    const fraction = unit === "usd" ? 2 : 0;
    return val.toLocaleString(undefined, {
      minimumFractionDigits: fraction,
      maximumFractionDigits: fraction
    });
  }

  function formatWindowLimit(w: WindowLimit): string | null {
    if (w.used_abs == null) return null;
    const unit = w.unit ? ` ${w.unit}` : "";
    if (w.limit_abs != null) {
      return `${formatUsage(w.used_abs, w.unit)} / ${formatUsage(w.limit_abs, w.unit)}${unit}`;
    }
    return `${formatUsage(w.used_abs, w.unit)}${unit}`;
  }

  function formatBytes(bytes?: number): string | null {
    if (!bytes) return null;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }

  function formatAccountLabel(ring: RingItem): string | null {
    const parts = [ring.account?.label, ring.account?.email].filter(Boolean);
    return parts.length ? parts.join(" · ") : null;
  }

  function formatGlyph(ring: RingItem): string {
    const sameLabelCount = dockState.rings.filter((r: RingItem) => r.label === ring.label).length;
    const acct = sameLabelCount > 1 ? (ring.account?.label ?? ring.account?.email ?? "") : "";
    const clean = acct.replace(/\(.*?\)/g, "").split(/[^\p{L}\p{N}]+/u).filter(Boolean);
    if (clean.length > 1) return (clean[0][0] + clean[1][0]).toUpperCase();
    if (clean.length === 1) return clean[0].slice(0, 2).toUpperCase();
    return ring.label.slice(0, 2).toUpperCase();
  }

  async function toggleExpand(ringId: string) {
    const next = selectedId === ringId ? null : ringId;
    selectedId = next;
    try {
      const res = await invokeV100("limits_strip_set_expanded", { expanded: next !== null });
      if (res && typeof res.offset === "number") {
        animTick = res.offset;
      }
    } catch {
      animTick = 0;
    }
    now = Date.now();
  }

  function selectSibling(id: string) {
    now = Date.now();
    selectedId = id;
  }

  async function refreshProvider(id: string) {
    try {
      await invokeV100("limits_strip_refresh", { providerId: id });
    } catch (e) {
      console.warn("limits_strip_refresh failed", e);
    }
  }

  async function closeStrip() {
    try {
      await invokeV100("limits_strip_close", {});
    } catch (e) {
      console.warn("limits_strip_close failed", e);
    }
  }

  async function openApp() {
    if (isOpeningApp) return;
    isOpeningApp = true;
    try {
      await invokeV100("usage_tray_open_app", {});
    } catch {
      hasError = true;
    } finally {
      isOpeningApp = false;
    }
  }

  async function handleGripMouseDown(e: MouseEvent) {
    if (e.button !== 0 || selectedId) return;
    try {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      getCurrentWebviewWindow().startDragging();
    } catch {
      // not in tauri window
    }
  }

  onMount(() => {
    isSolid = new URLSearchParams(location.search).has("solid");
    const doc = document.documentElement;
    const origBackground = [doc.style.background, document.body.style.background];
    if (!isSolid) {
      doc.style.background = "transparent";
      document.body.style.background = "transparent";
    }

    const unlistens: (() => void)[] = [];
    let isDisposed = false;

    import("@tauri-apps/api/event").then(({ listen }) => {
      if (isDisposed) return;
      listen<StripState>("limits://state", (event) => {
        now = Date.now();
        dockState = event.payload;
        isLoading = false;
        hasError = false;
      }).then((un) => {
        if (isDisposed) un();
        else unlistens.push(un);
      });

      listen<{ provider: string }>("limits://chime", (event) => {
        chimeProvider = event.payload.provider;
        setTimeout(() => {
          chimeProvider = null;
        }, 2400);
      }).then((un) => {
        if (isDisposed) un();
        else unlistens.push(un);
      });
    }).catch(() => {});

    // Initial state fetch
    invokeV100("limits_strip_state", {})
      .then((res: any) => {
        now = Date.now();
        if (res && res.rings) {
          dockState = res;
        } else {
          // Mock data for pristine preview when no active providers registered
          dockState = {
            open: true,
            edge: "top",
            look: defaultLook,
            rings: [
              {
                id: "claude:pro",
                label: "Claude Pro",
                status: "ok",
                activity: "idle",
                reading: {
                  plan: "Pro Plan",
                  windows: [
                    { id: "session", label: "Session Quota", used: 0.35, resets_at: Date.now() + 7200000 },
                    { id: "weekly", label: "Weekly Cap", used: 0.58, resets_at: Date.now() + 86400000 * 3 }
                  ]
                }
              },
              {
                id: "codex:team",
                label: "Codex",
                status: "ok",
                activity: "working",
                reading: {
                  plan: "Team Plan",
                  windows: [
                    { id: "tokens", label: "Daily Tokens", used: 0.88, resets_at: Date.now() + 18000000 }
                  ]
                }
              }
            ]
          };
        }
      })
      .catch(() => {
        hasError = true;
      })
      .finally(() => {
        isLoading = false;
      });

    // Theme sync
    const mediaDark = matchMedia("(prefers-color-scheme: dark)");
    const updateTheme = () => {
      const docTheme = doc.getAttribute("data-theme");
      theme = docTheme === "light" || docTheme === "dark" ? docTheme : mediaDark.matches ? "dark" : "light";
    };
    updateTheme();
    mediaDark.addEventListener("change", updateTheme);

    const observer = new MutationObserver(updateTheme);
    observer.observe(doc, { attributes: true, attributeFilter: ["data-theme"] });

    const handleVisibility = () => {
      isAsleep = document.visibilityState === "hidden";
    };
    document.addEventListener("visibilitychange", handleVisibility);

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && selectedId) {
        toggleExpand(selectedId);
      }
    };
    const handleBlur = () => {
      if (selectedId) toggleExpand(selectedId);
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("blur", handleBlur);

    const clockInterval = setInterval(() => {
      if (selectedId) now = Date.now();
    }, 30000);

    return () => {
      isDisposed = true;
      unlistens.forEach((u) => u());
      clearInterval(clockInterval);
      mediaDark.removeEventListener("change", updateTheme);
      observer.disconnect();
      document.removeEventListener("visibilitychange", handleVisibility);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("blur", handleBlur);
      doc.style.background = origBackground[0];
      document.body.style.background = origBackground[1];
    };
  });
</script>

<svelte:window oncontextmenu={(e) => e.preventDefault()} />

<main
  class="stage edge-{dockState.edge} theme-{theme}"
  class:vertical={isVertical}
  class:solid={isSolid}
  class:asleep={isAsleep}
  class:labels={look.show_percent}
  class:contrast={look.contrast}
  style:zoom={zoomScale === 1 ? null : zoomScale}
>
  <!-- Main Dock Bar -->
  <div
    role="toolbar"
    class="pill"
    class:vertical={isVertical}
    aria-label={$t("llm.limits.strip.aria") || "LLM Quota Dock"}
    style={isVertical ? `margin-top: ${animTick / zoomScale}px` : `margin-left: ${animTick / zoomScale}px`}
  >
    <!-- Drag Handle Grip -->
    <span class="grip" aria-hidden="true" onmousedown={handleGripMouseDown}>
      <i></i><i></i><i></i>
    </span>

    <!-- Stack of Provider Rings -->
    {#if dockState.rings.length > 0}
      {#each groupedToolRings as group (group.rings[0].id)}
        <div class="stack" class:many={group.rings.length > 1}>
          {#each group.rings as ring (ring.id)}
            {@const maxUsed = getRingMaxUsed(ring)}
            {@const wins = getRingWindows(ring)}
            {@const tone = getTone(maxUsed)}
            <button
              type="button"
              class="ring tone-{tone} act-{ring.activity}"
              class:open={selectedId === ring.id}
              class:pulse={chimeProvider === ring.id}
              class:critical={criticalRingId === ring.id}
              class:dim={ring.status !== "ok" && !ring.reading}
              title={ring.label}
              aria-label={ring.label}
              aria-expanded={selectedId === ring.id}
              onclick={() => toggleExpand(ring.id)}
            >
              <!-- Concentric Rings Canvas / SVG -->
              <span class="lr" class:two={wins.inner != null}>
                <svg viewBox="0 0 32 32" aria-hidden="true">
                  {#if wins.inner != null}
                    <!-- Outer Ring Track & Arc -->
                    <circle class="track" cx="16" cy="16" r="14.4" />
                    <circle
                      class="arc t-{getTone(wins.outer)}"
                      class:zero={wins.outer == null || wins.outer < 0.005}
                      cx="16"
                      cy="16"
                      r="14.4"
                      pathLength="100"
                      style:stroke-dashoffset={wins.outer != null ? 100 - Math.min(1, wins.outer) * 100 : 100}
                    />
                    <!-- Inner Ring Track & Arc -->
                    <circle class="track in" cx="16" cy="16" r="10.9" />
                    <circle
                      class="arc in t-{getTone(wins.inner)}"
                      class:zero={wins.inner == null || wins.inner < 0.005}
                      cx="16"
                      cy="16"
                      r="10.9"
                      pathLength="100"
                      style:stroke-dashoffset={wins.inner != null ? 100 - Math.min(1, wins.inner) * 100 : 100}
                    />
                  {:else}
                    <!-- Solo Ring Track & Arc -->
                    <circle class="track solo" cx="16" cy="16" r="13.6" />
                    {#if wins.outer != null}
                      <circle
                        class="arc solo t-{getTone(wins.outer)}"
                        class:zero={wins.outer < 0.005}
                        cx="16"
                        cy="16"
                        r="13.6"
                        pathLength="100"
                        style:stroke-dashoffset={100 - Math.min(1, wins.outer) * 100}
                      />
                    {/if}
                  {/if}
                </svg>

                <span class="glyph">
                  {ring.local ? String(ring.reading?.local_models?.length ?? 0) : formatGlyph(ring)}
                </span>
              </span>

              <!-- Percentage Label -->
              {#if look.show_percent}
                <span class="pct" class:none={maxUsed == null}>
                  {maxUsed != null ? `${Math.round(maxUsed * 100)}%` : "–"}
                </span>
              {/if}

              <!-- Status Dots -->
              {#if ring.activity === "waiting" || ring.activity === "done"}
                <span class="dot dot-{ring.activity}"></span>
              {:else if ring.status === "needs_auth" || ring.status === "error" || ring.status === "rate_limited"}
                <span class="dot dot-problem"></span>
              {/if}
            </button>
          {/each}
        </div>
      {/each}
    {:else}
      <!-- Empty / Loading State -->
      {#if isLoading}
        <span class="empty" role="status">…</span>
      {:else}
        <span class="empty-tip-anchor">
          <button
            class="empty empty-action"
            type="button"
            disabled={isOpeningApp}
            aria-label={$t(hasError ? "llm.limits.strip.open_after_error" : "llm.limits.strip.connect_hint") || "Connect"}
            title={$t(hasError ? "llm.limits.strip.open_after_error" : "llm.limits.strip.connect_hint") || "Connect"}
            onclick={openApp}
            onfocus={() => (showEmptyTip = true)}
            onblur={() => (showEmptyTip = false)}
          >
            +
          </button>
          {#if showEmptyTip}
            <span class="empty-tip" role="tooltip">
              {$t(hasError ? "llm.limits.strip.open_after_error" : "llm.limits.strip.connect_hint") || "Open app to connect models"}
            </span>
          {/if}
        </span>
      {/if}
    {/if}
  </div>

  <!-- Expanded Details Overlay Card -->
  {#if activeRing}
    {@const ring = activeRing}
    {@const maxUsed = getRingMaxUsed(ring)}
    {@const tone = getTone(maxUsed)}
    <section class="card" aria-label={ring.label}>
      <header class="card-head">
        <div class="avatar" class:alert={tone === "red"}>
          <span class="glyph-big">{formatGlyph(ring)}</span>
        </div>
        <div class="head-main">
          <div class="head-row">
            <span class="card-title">{ring.label}</span>
            {#if ring.beta}
              <span class="tag" title={$t("llm.limits.beta_hint") || "Beta model"}>BETA</span>
            {/if}
            {#if ring.local}
              <span class="tag">LOCAL</span>
            {/if}
          </div>
          {#if formatAccountLabel(ring)}
            <p class="card-who" title={formatAccountLabel(ring)}>
              {formatAccountLabel(ring)}
            </p>
          {/if}
          <div class="head-row chips">
            {#if ring.activity !== "idle"}
              <span class="chip chip-{ring.activity}">
                {$t(`llm.limits.strip.state.${ring.activity}`) || ring.activity}
              </span>
            {/if}
            {#if maxUsed != null}
              <span class="chip tone-{tone} chip-used">
                {tone === "red" ? ($t("llm.limits.strip.near_limit") || "Near limit") : `${Math.round(maxUsed * 100)}% used`}
              </span>
            {/if}
          </div>
        </div>
      </header>

      <!-- Account Switcher Nav -->
      {#if accountSiblings.length > 1}
        <nav class="accounts" aria-label={$t("llm.limits.strip.accounts") || "Accounts"}>
          {#each accountSiblings as sibling (sibling.id)}
            {@const sibWins = getRingWindows(sibling)}
            <button
              type="button"
              class="acct"
              class:on={sibling.id === ring.id}
              title={sibling.label}
              aria-pressed={sibling.id === ring.id}
              onclick={() => selectSibling(sibling.id)}
            >
              <span class="acct-dot tone-{getTone(sibWins.outer)}"></span>
              <span class="acct-name">{sibling.account?.label || sibling.account?.email || sibling.label}</span>
            </button>
          {/each}
        </nav>
      {/if}

      <!-- Windows & Quotas Breakdown -->
      <div class="card-body">
        {#if ring.status !== "ok"}
          <p class="status">
            {$t(`llm.limits.strip.status.${ring.status}`) || ring.status}
            {#if ring.message}
              <span class="status-detail">{ring.message}</span>
            {/if}
          </p>
        {/if}

        {#if ring.reading}
          {#each ring.reading.windows ?? [] as win (win.id)}
            {@const winTone = getTone(win.used)}
            <div class="win tone-{winTone}">
              <div class="win-top">
                <span class="win-label">{win.group ? `${win.group} · ${win.label}` : win.label}</span>
                {#if win.used != null}
                  <span class="win-pct">{Math.round(win.used * 100)}%</span>
                {/if}
              </div>
              <div class="bar">
                <span style:width="{Math.min(100, Math.round((win.used ?? 0) * 100))}%"></span>
              </div>
              {#if formatWindowLimit(win) || formatReset(win.resets_at)}
                <div class="win-sub">
                  {[formatWindowLimit(win), formatReset(win.resets_at)].filter(Boolean).join(" · ")}
                </div>
              {/if}
            </div>
          {/each}

          <!-- Local Models Info -->
          {#if ring.reading.local_models && ring.reading.local_models.length > 0}
            <h3 class="sub">{$t("llm.limits.strip.models") || "Local Models"}</h3>
            {#each ring.reading.local_models as model (model.name)}
              <div class="model">
                <span class="model-name">{model.name}</span>
                <span class="win-sub">
                  {[model.params, model.quant, formatBytes(model.vram_bytes ?? model.size_bytes)].filter(Boolean).join(" · ")}
                </span>
              </div>
            {/each}
          {/if}

          <!-- Metadata -->
          {#if ring.reading.plan}
            <p class="meta"><strong>{$t("llm.limits.strip.plan") || "Plan"}:</strong> {ring.reading.plan}</p>
          {/if}
          {#if ring.reading.account}
            <p class="meta">{ring.reading.account}</p>
          {/if}
          {#if ring.reading.note}
            <p class="meta">{ring.reading.note}</p>
          {/if}
        {/if}
      </div>

      <!-- Footer Actions -->
      <footer class="card-foot">
        <span class="meta">
          {ring.read_at ? `Updated ${formatDuration(now - ring.read_at)} ago` : ""}
        </span>
        <span class="foot-actions">
          {#if ring.id !== "omniget"}
            <button type="button" class="link" onclick={() => refreshProvider(ring.id)}>
              {$t("llm.limits.strip.refresh") || "Refresh"}
            </button>
          {/if}
          <button type="button" class="link" onclick={closeStrip}>
            {$t("llm.limits.strip.hide") || "Hide"}
          </button>
        </span>
      </footer>
    </section>
  {/if}
</main>

<style>
  :global(html, body) {
    margin: 0;
    overflow: hidden;
  }

  .stage {
    --calm: #4aa84b;
    --amber: #f59e35;
    --red: #e03a00;
    --work: #4aa84b;
    --ink: rgba(255, 255, 255, 0.93);
    --ink-dim: rgba(255, 255, 255, 0.6);
    --glass: rgba(24, 24, 26, 0.94);
    --edge: rgba(255, 255, 255, 0.09);
    --track: rgba(255, 255, 255, 0.14);
    --hover: rgba(255, 255, 255, 0.1);
    --stack: rgba(255, 255, 255, 0.055);
    --chip: rgba(255, 255, 255, 0.1);
    --shadow: 0 8px 24px rgba(0, 0, 0, 0.28);
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: flex-start;
    color: var(--ink);
    font: 12px/1.35 -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    user-select: none;
    -webkit-user-select: none;
    cursor: default;
  }

  .stage.theme-dark {
    --calm: #5cc45d;
  }

  .stage.theme-light {
    --calm: #2f8f3a;
    --amber: #d9780f;
    --red: #d23500;
    --ink: rgba(17, 17, 17, 0.92);
    --ink-dim: rgba(17, 17, 17, 0.58);
    --glass: rgba(253, 246, 236, 0.95);
    --edge: rgba(17, 17, 17, 0.1);
    --track: rgba(17, 17, 17, 0.1);
    --hover: rgba(17, 17, 17, 0.07);
    --stack: rgba(29, 90, 60, 0.07);
    --chip: rgba(17, 17, 17, 0.07);
    --shadow: 0 8px 24px rgba(60, 40, 10, 0.14);
  }

  .stage.contrast {
    --calm: #6fdc70;
    --amber: #ffb35c;
    --red: #ff6a3d;
    --work: #6fdc70;
    --ink: #fff;
    --ink-dim: rgba(255, 255, 255, 0.84);
    --glass: #111113;
    --edge: rgba(255, 255, 255, 0.34);
    --track: rgba(255, 255, 255, 0.3);
    --stack: rgba(255, 255, 255, 0.1);
    --chip: rgba(255, 255, 255, 0.16);
  }

  .stage.solid {
    background: #1c1c1e;
  }
  .stage.solid.theme-light {
    background: #fdf6ec;
  }

  .stage.edge-bottom {
    flex-direction: column-reverse;
  }
  .stage.edge-left {
    flex-direction: row;
  }
  .stage.edge-right {
    flex-direction: row-reverse;
  }

  .stage.asleep * {
    animation-play-state: paused !important;
  }

  /* Pill bar */
  .pill {
    flex: none;
    box-sizing: border-box;
    height: 44px;
    padding: 0 8px;
    display: flex;
    align-items: center;
    border-radius: 14px;
    background: var(--glass);
    box-shadow: inset 0 0 0 1px var(--edge), var(--shadow);
  }
  .pill.vertical {
    height: auto;
    width: 44px;
    padding: 8px 0;
    flex-direction: column;
  }
  .stage.labels .pill:not(.vertical) {
    height: 56px;
  }

  /* Grip */
  .grip {
    flex: none;
    width: 14px;
    height: 28px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    cursor: grab;
  }
  .pill.vertical .grip {
    width: 28px;
    height: 14px;
    flex-direction: row;
  }
  .grip i {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--ink-dim);
    opacity: 0.6;
  }

  /* Stack */
  .stack {
    position: relative;
    display: flex;
    flex-direction: inherit;
  }
  .stack.many::before {
    content: "";
    position: absolute;
    top: 2px;
    right: 1px;
    bottom: 2px;
    left: 1px;
    border-radius: 12px;
    background: var(--stack);
    box-shadow: inset 0 0 0 1px var(--edge);
  }
  .pill.vertical .stack {
    flex-direction: column;
  }
  .pill.vertical .stack.many::before {
    top: 1px;
    right: 2px;
    bottom: 1px;
    left: 2px;
  }

  /* Ring Button */
  .ring {
    flex: none;
    position: relative;
    width: 36px;
    height: 36px;
    padding: 2px;
    border: none;
    border-radius: 10px;
    background: transparent;
    color: inherit;
    cursor: pointer;
    transition: background 0.16s ease, transform 0.16s ease;
  }
  .ring:hover,
  .ring.open {
    background: var(--hover);
  }
  .ring:active {
    transform: scale(0.94);
  }
  .ring:focus-visible {
    outline: 2px solid var(--calm);
    outline-offset: -2px;
  }
  .ring.dim {
    opacity: 0.5;
  }
  .stage.labels .ring {
    height: 48px;
  }

  .pct {
    display: block;
    height: 12px;
    font-size: 9.5px;
    font-weight: 700;
    line-height: 12px;
    text-align: center;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
    color: var(--tone);
    white-space: nowrap;
  }
  .pct.none {
    color: var(--ink-dim);
  }

  .tone-calm { --tone: var(--calm); }
  .tone-amber { --tone: var(--amber); }
  .tone-red { --tone: var(--red); }

  .ring::before {
    content: "";
    position: absolute;
    top: 3px;
    right: 3px;
    bottom: 3px;
    left: 3px;
    border-radius: 50%;
    box-shadow: 0 0 7px 1px color-mix(in srgb, var(--halo, var(--tone)) 75%, transparent),
      inset 0 0 5px color-mix(in srgb, var(--halo, var(--tone)) 45%, transparent);
    opacity: 0;
    transform: scale(0.9);
    transition: opacity 0.26s ease, transform 0.26s ease;
    pointer-events: none;
  }
  .stage.labels .ring::before {
    bottom: 15px;
  }
  .ring.critical::before {
    opacity: 0.8;
    transform: scale(1);
    animation: alarm 1.1s ease-in-out 3;
  }
  .ring.act-working::before {
    --halo: var(--work);
    animation: breathe 2.4s ease-in-out infinite;
  }
  .ring.pulse::before {
    animation: alarm 0.8s ease-in-out 3;
  }

  @keyframes breathe {
    0%, 100% { opacity: 0.25; transform: scale(0.86); }
    50% { opacity: 0.9; transform: scale(1.08); }
  }
  @keyframes alarm {
    0%, 100% { opacity: 0.85; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.2); }
  }

  /* SVG / Concentric Rings */
  .lr {
    position: relative;
    display: block;
    width: 32px;
    height: 32px;
  }
  svg {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
    overflow: visible;
  }
  .track {
    fill: none;
    stroke: var(--track);
    stroke-width: 3;
  }
  .track.in { stroke-width: 2.4; }
  .track.solo { stroke-width: 3.6; }
  .arc {
    fill: none;
    stroke: var(--tone);
    stroke-width: 3;
    stroke-linecap: round;
    stroke-dasharray: 100 100;
    transition: stroke-dashoffset 0.9s cubic-bezier(0.22, 1.2, 0.36, 1), stroke 0.26s ease, opacity 0.2s ease;
  }
  .arc.in { stroke-width: 2.4; }
  .arc.solo { stroke-width: 3.6; }
  .arc.zero { opacity: 0; }
  .t-calm { --tone: var(--calm); }
  .t-amber { --tone: var(--amber); }
  .t-red { --tone: var(--red); }

  .glyph {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    display: grid;
    place-items: center;
    font-size: 8.5px;
    font-weight: 750;
    letter-spacing: 0.01em;
    color: var(--ink);
  }

  .dot {
    position: absolute;
    right: 2px;
    bottom: 2px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px var(--glass);
  }
  .stage.labels .dot { bottom: 14px; }
  .dot-waiting { background: var(--amber); }
  .dot-done { background: var(--calm); }
  .dot-problem { background: var(--ink-dim); }

  .empty {
    width: 36px;
    text-align: center;
    color: var(--ink-dim);
  }
  .empty-action {
    min-height: 32px;
    border: none;
    border-radius: 8px;
    background: transparent;
    font-size: 21px;
    cursor: pointer;
    color: var(--ink);
  }
  .empty-action:hover {
    background: var(--hover);
  }
  .empty-tip-anchor {
    position: relative;
    display: inline-flex;
  }
  .empty-tip {
    position: absolute;
    top: 50%;
    left: calc(100% + 6px);
    transform: translateY(-50%);
    z-index: 2;
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--glass);
    box-shadow: inset 0 0 0 1px var(--edge), var(--shadow);
    color: var(--ink);
    font-size: 12px;
    white-space: nowrap;
  }

  /* Card Details */
  .card {
    flex: 1;
    align-self: stretch;
    min-width: 280px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    border-radius: 16px;
    background: var(--glass);
    box-shadow: inset 0 0 0 1px var(--edge), var(--shadow);
    user-select: text;
    -webkit-user-select: text;
    animation: card-in 0.22s cubic-bezier(0.2, 0.9, 0.3, 1.1) 1;
  }
  @keyframes card-in {
    0% { opacity: 0; transform: scale(0.96) translateY(-4px); }
  }

  .card-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 12px 8px;
  }
  .avatar {
    flex: none;
    position: relative;
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--stack);
    box-shadow: inset 0 0 0 1px var(--edge);
  }
  .glyph-big {
    font-size: 14px;
    font-weight: 750;
    color: var(--ink);
  }
  .head-main {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .head-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .card-title {
    font-size: 14px;
    font-weight: 700;
  }
  .tag {
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 1px 5px;
    border-radius: 999px;
    background: var(--chip);
    color: var(--ink-dim);
  }
  .card-who {
    margin: 0;
    font-size: 11px;
    color: var(--ink-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chips {
    margin-top: 2px;
  }
  .chip {
    font-size: 10.5px;
    font-weight: 650;
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--chip);
    color: var(--ink-dim);
    white-space: nowrap;
  }
  .chip-working { color: var(--work); background: color-mix(in srgb, var(--work) 16%, transparent); }
  .chip-waiting { color: var(--amber); background: color-mix(in srgb, var(--amber) 16%, transparent); }
  .chip-done { color: var(--calm); background: color-mix(in srgb, var(--calm) 16%, transparent); }
  .chip-used { color: var(--tone); background: color-mix(in srgb, var(--tone) 14%, transparent); }

  /* Accounts Bar */
  .accounts {
    display: flex;
    gap: 4px;
    padding: 0 10px 8px;
    overflow-x: auto;
  }
  .acct {
    flex: none;
    display: flex;
    align-items: center;
    gap: 5px;
    max-width: 120px;
    padding: 3px 8px 3px 6px;
    border: none;
    border-radius: 999px;
    background: var(--chip);
    color: var(--ink-dim);
    font-size: 11px;
    cursor: pointer;
  }
  .acct:hover { color: var(--ink); }
  .acct.on {
    color: var(--ink);
    background: color-mix(in srgb, var(--calm) 20%, transparent);
  }
  .acct-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--tone);
  }
  .acct-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Body */
  .card-body {
    flex: 1;
    overflow-y: auto;
    padding: 0 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .status {
    margin: 0;
    color: var(--amber);
  }
  .status-detail {
    display: block;
    color: var(--ink-dim);
  }
  .win-top {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .win-label { font-weight: 600; }
  .win-pct {
    font-variant-numeric: tabular-nums;
    color: var(--tone);
    font-weight: 700;
  }
  .bar {
    position: relative;
    height: 5px;
    margin: 3px 0;
    border-radius: 3px;
    background: var(--track);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: var(--tone);
    transition: width 0.7s cubic-bezier(0.22, 1.2, 0.36, 1);
  }
  .win-sub, .meta {
    margin: 0;
    font-size: 11px;
    color: var(--ink-dim);
  }
  .sub {
    margin: 4px 0 0;
    font-size: 11px;
    font-weight: 700;
    color: var(--ink-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .model-name { font-weight: 600; }

  /* Footer */
  .card-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 12px 8px;
  }
  .foot-actions {
    display: flex;
    gap: 10px;
  }
  .link {
    border: none;
    padding: 0;
    background: none;
    font: inherit;
    font-size: 11px;
    color: var(--calm);
    cursor: pointer;
  }
  .link:hover { text-decoration: underline; }
</style>
