<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { invokeV100 } from "$lib/engine/ipc-bridge";

  interface MeterData {
    used: number; // 0.0 - 1.0
    expected?: number;
    resets_at?: number;
    runs_out_at?: number;
    level?: string;
  }

  interface OtherQuota {
    id: string;
    label: string;
    used: number;
    expected?: number;
    level?: string;
  }

  interface AccountUsage {
    id: string;
    name: string;
    cli: "claude" | "codex" | string;
    plan?: string;
    status?: "ok" | "needs_auth" | "rate_limited" | "error" | "pending";
    message?: string;
    session?: MeterData;
    weekly?: MeterData;
    others?: OtherQuota[];
    spend_today?: number;
    read_at?: number;
  }

  interface CatalogItem {
    id: string;
    name: string;
    shown: boolean;
  }

  interface UsageView {
    accounts: AccountUsage[];
  }

  let view = $state<UsageView | null>(null);
  let catalog = $state<CatalogItem[]>([]);
  let isChooserOpen = $state(false);
  let now = $state(Date.now());
  let theme = $state<"dark" | "light">("dark");
  let panelEl = $state<HTMLElement | null>(null);

  const RADIUS = 22;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

  const isAccountsEmpty = $derived(!view || view.accounts.length === 0);
  const showChooser = $derived(isChooserOpen || isAccountsEmpty);

  function formatTime(timestamp: number): string {
    const d = new Date(timestamp);
    const isToday = d.toDateString() === new Date(now).toDateString();
    const timeStr = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    return isToday ? timeStr : `${d.toLocaleDateString([], { weekday: "short" })} ${timeStr}`;
  }

  function formatRelativeMinutes(timestamp: number): string {
    const mins = Math.round((timestamp - now) / 60000);
    if (mins <= 0) return $t("usage_tray.panel.now") || "now";
    if (mins < 60) return $t("usage_tray.panel.in_min", { count: mins }) || `in ${mins}m`;
    if (mins < 2880) {
      const h = Math.floor(mins / 60);
      const m = String(mins % 60).padStart(2, "0");
      return $t("usage_tray.panel.in_hours", { hours: h, minutes: m }) || `in ${h}h ${m}m`;
    }
    return $t("usage_tray.panel.in_days", { count: Math.round(mins / 1440) }) || `in ${Math.round(mins / 1440)}d`;
  }

  function formatCurrency(amount: number): string {
    return amount.toLocaleString([], {
      style: "currency",
      currency: "USD",
      maximumFractionDigits: 2
    });
  }

  function formatPercent(meter: MeterData | OtherQuota): string {
    return `${Math.round(meter.used * 100)}%`;
  }

  function getTickCoordinates(fraction: number) {
    const angle = Math.max(0, Math.min(1, fraction)) * 2 * Math.PI - Math.PI / 2;
    const r1 = RADIUS - 4.5;
    const r2 = RADIUS + 4.5;
    return {
      x1: 28 + r1 * Math.cos(angle),
      y1: 28 + r1 * Math.sin(angle),
      x2: 28 + r2 * Math.cos(angle),
      y2: 28 + r2 * Math.sin(angle)
    };
  }

  function getStatusText(account: AccountUsage): string | null {
    if (account.status === "needs_auth") return $t("usage_tray.panel.needs_auth") || "Needs authentication";
    if (account.status === "rate_limited") return $t("usage_tray.panel.rate_limited") || "Rate limited";
    if (account.status === "error") return account.message || $t("usage_tray.panel.error") || "Connection error";
    if (account.status === "pending") return $t("usage_tray.panel.pending") || "Connecting…";
    return null;
  }

  async function loadData() {
    try {
      const res = await invokeV100("usage_tray_view", {});
      if (res && res.view) {
        view = res.view;
        catalog = res.catalog ?? [];
      } else {
        // High fidelity mock fallback for standalone preview
        view = {
          accounts: [
            {
              id: "claude-pro",
              name: "Claude Pro (Personal)",
              cli: "claude",
              plan: "Pro",
              status: "ok",
              session: {
                used: 0.42,
                expected: 0.35,
                resets_at: Date.now() + 5400000,
                runs_out_at: Date.now() + 18000000,
                level: "calm"
              },
              weekly: {
                used: 0.68,
                expected: 0.7,
                resets_at: Date.now() + 86400000 * 4,
                level: "calm"
              },
              others: [
                { id: "tokens", label: "Monthly Output Tokens", used: 0.54, level: "calm" },
                { id: "context", label: "Max Context Usage", used: 0.82, level: "amber" }
              ],
              spend_today: 1.24,
              read_at: Date.now() - 120000
            }
          ]
        };
        catalog = [
          { id: "claude-pro", name: "Claude Pro (Personal)", shown: true }
        ];
      }
    } catch {
      // Keep view intact
    }
  }

  async function toggleAccount(id: string) {
    try {
      await invokeV100("usage_tray_toggle_account", { id });
      await loadData();
    } catch (e) {
      console.warn("toggleAccount failed", e);
    }
  }

  async function refresh() {
    try {
      await invokeV100("usage_tray_refresh", {});
      await loadData();
    } catch (e) {
      console.warn("refresh failed", e);
    }
  }

  async function openApp() {
    try {
      await invokeV100("usage_tray_open_app", {});
    } catch (e) {
      console.warn("openApp failed", e);
    }
  }

  async function openClaude(id: string | null, add: boolean) {
    try {
      await invokeV100("usage_tray_open_claude", { id, add });
      await invokeV100("usage_tray_close_panel", {});
    } catch (e) {
      console.warn("openClaude failed", e);
    }
  }

  onMount(() => {
    const doc = document.documentElement;
    doc.style.background = "transparent";
    document.body.style.background = "transparent";

    const unlistens: (() => void)[] = [];
    let isDisposed = false;

    import("@tauri-apps/api/event").then(({ listen }) => {
      if (isDisposed) return;
      listen<UsageView>("usage-tray://view", (event) => {
        view = event.payload;
      }).then((un) => {
        if (isDisposed) un();
        else unlistens.push(un);
      });
    }).catch(() => {});

    loadData();

    // Theme sync
    const mediaDark = matchMedia("(prefers-color-scheme: dark)");
    const updateTheme = () => {
      const docTheme = doc.getAttribute("data-theme");
      theme = docTheme === "light" || docTheme === "dark" ? docTheme : mediaDark.matches ? "dark" : "light";
    };
    updateTheme();
    mediaDark.addEventListener("change", updateTheme);

    const clock = setInterval(() => {
      now = Date.now();
    }, 30000);

    const handleFocus = () => {
      now = Date.now();
      loadData();
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        invokeV100("usage_tray_close_panel", {}).catch(() => {});
      }
    };

    window.addEventListener("focus", handleFocus);
    window.addEventListener("keydown", handleKeyDown);

    // Height sync
    let ro: ResizeObserver | null = null;
    if (panelEl) {
      ro = new ResizeObserver(() => {
        if (!panelEl) return;
        const height = Math.ceil(panelEl.getBoundingClientRect().height) + 2;
        invokeV100("usage_tray_resize", { height }).catch(() => {});
      });
      ro.observe(panelEl);
    }

    return () => {
      isDisposed = true;
      unlistens.forEach((u) => u());
      clearInterval(clock);
      mediaDark.removeEventListener("change", updateTheme);
      window.removeEventListener("focus", handleFocus);
      window.removeEventListener("keydown", handleKeyDown);
      ro?.disconnect();
    };
  });
</script>

<svelte:window oncontextmenu={(e) => e.preventDefault()} />

<main bind:this={panelEl} class="panel theme-{theme}">
  <!-- Header -->
  <header>
    <div class="header-icon">
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" stroke-linecap="round"/>
      </svg>
    </div>
    <h1>{$t("usage_tray.panel.title") || "Usage & Quotas"}</h1>
    <button
      type="button"
      class="icon-btn"
      title={$t("usage_tray.panel.refresh") || "Refresh"}
      aria-label={$t("usage_tray.panel.refresh") || "Refresh"}
      onclick={refresh}
    >
      <svg viewBox="0 0 16 16" width="14" height="14">
        <path
          d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2.5v3h-3"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </header>

  <!-- Accounts List -->
  {#if view && view.accounts.length > 0}
    <section class="accounts">
      {#each view.accounts as account (account.id)}
        <article class="card">
          <!-- Card Header -->
          <div class="who">
            <span class="badge" class:codex={account.cli === "codex"}>
              {account.cli === "codex" ? "Codex" : "Claude"}
            </span>
            <span class="name">{account.name}</span>
            {#if account.plan}
              <span class="plan">{account.plan}</span>
            {/if}
          </div>

          <!-- Status Alert if non-ok -->
          {#if getStatusText(account)}
            <p class="status {account.status}">
              {getStatusText(account)}
            </p>
          {/if}

          <!-- Gauge Meters Grid (Session & Weekly) -->
          <div class="meters">
            <!-- Session Meter -->
            {#if account.session}
              {@const sess = account.session}
              <div class="meter">
                <svg viewBox="0 0 56 56" aria-hidden="true" class="ring {sess.level ?? 'none'}">
                  <circle cx="28" cy="28" r={RADIUS} class="track" />
                  <circle
                    cx="28"
                    cy="28"
                    r={RADIUS}
                    class="fill"
                    transform="rotate(-90 28 28)"
                    stroke-dasharray={CIRCUMFERENCE}
                    stroke-dashoffset={CIRCUMFERENCE * (1 - Math.min(1, Math.max(0, sess.used)))}
                  />
                  {#if sess.expected != null}
                    {@const tick = getTickCoordinates(sess.expected)}
                    <line class="tick-under" x1={tick.x1} y1={tick.y1} x2={tick.x2} y2={tick.y2} />
                    <line class="tick" x1={tick.x1} y1={tick.y1} x2={tick.x2} y2={tick.y2} />
                  {/if}
                  <text x="28" y="32" text-anchor="middle">{formatPercent(sess)}</text>
                </svg>
                <div class="meter-text">
                  <span class="meter-label">{$t("usage_tray.panel.session") || "Session"}</span>
                  {#if sess.resets_at}
                    <span class="dim">
                      {$t("usage_tray.panel.resets_at", { when: `${formatTime(sess.resets_at)} · ${formatRelativeMinutes(sess.resets_at)}` }) || `resets ${formatRelativeMinutes(sess.resets_at)}`}
                    </span>
                  {/if}
                  {#if sess.runs_out_at}
                    <span class="forecast {sess.level}">
                      {$t("usage_tray.panel.runs_out", { when: formatTime(sess.runs_out_at) }) || `runs out at ${formatTime(sess.runs_out_at)}`}
                    </span>
                  {:else if sess.resets_at}
                    <span class="dim">{$t("usage_tray.panel.lasts") || "healthy pace"}</span>
                  {/if}
                </div>
              </div>
            {/if}

            <!-- Weekly Meter -->
            {#if account.weekly}
              {@const wk = account.weekly}
              <div class="meter">
                <svg viewBox="0 0 56 56" aria-hidden="true" class="ring {wk.level ?? 'none'}">
                  <circle cx="28" cy="28" r={RADIUS} class="track" />
                  <circle
                    cx="28"
                    cy="28"
                    r={RADIUS}
                    class="fill"
                    transform="rotate(-90 28 28)"
                    stroke-dasharray={CIRCUMFERENCE}
                    stroke-dashoffset={CIRCUMFERENCE * (1 - Math.min(1, Math.max(0, wk.used)))}
                  />
                  <text x="28" y="32" text-anchor="middle">{formatPercent(wk)}</text>
                </svg>
                <div class="meter-text">
                  <span class="meter-label">{$t("usage_tray.panel.weekly") || "Weekly"}</span>
                  {#if wk.resets_at}
                    <span class="dim">
                      {$t("usage_tray.panel.resets_at", { when: `${formatTime(wk.resets_at)} · ${formatRelativeMinutes(wk.resets_at)}` }) || `resets ${formatRelativeMinutes(wk.resets_at)}`}
                    </span>
                  {/if}
                </div>
              </div>
            {/if}
          </div>

          <!-- Other Quotas Breakdown -->
          {#if account.others && account.others.length > 0}
            <ul class="others">
              {#each account.others as item (item.id)}
                <li>
                  <span>{item.label}</span>
                  <span class="bar">
                    <span
                      class="bar-fill {item.level}"
                      style:width="{Math.min(100, Math.round(item.used * 100))}%"
                    ></span>
                  </span>
                  <span class="num">{formatPercent(item)}</span>
                </li>
              {/each}
            </ul>
          {/if}

          <!-- Card Foot -->
          <div class="foot">
            {#if account.spend_today != null}
              <span>{$t("usage_tray.panel.spent_today", { amount: formatCurrency(account.spend_today) }) || `Today: ${formatCurrency(account.spend_today)}`}</span>
            {/if}
            {#if account.read_at}
              <span class="dim">
                {$t("usage_tray.panel.updated", { when: formatTime(account.read_at) }) || `Updated ${formatTime(account.read_at)}`}
              </span>
            {/if}
            {#if account.cli === "claude"}
              <button
                type="button"
                class="link"
                onclick={() => openClaude(account.id, false)}
              >
                {$t("usage_tray.panel.open_claude") || "Open in Claude"}
              </button>
            {/if}
          </div>
        </article>
      {/each}
    </section>
  {/if}

  <!-- Account Chooser Section -->
  {#if showChooser}
    <section class="chooser">
      {#if isAccountsEmpty}
        <h2>{$t("usage_tray.panel.empty_title") || "No accounts active"}</h2>
        <p class="dim">{$t("usage_tray.panel.empty_body") || "Select accounts to monitor their quotas in real time."}</p>
      {/if}
      {#if catalog.length === 0}
        <p class="dim">{$t("usage_tray.panel.no_logins") || "No model provider accounts found on this system."}</p>
      {:else}
        {#each catalog as item (item.id)}
          <label class="pick">
            <input
              type="checkbox"
              checked={item.shown}
              onchange={() => toggleAccount(item.id)}
            />
            <span>{item.name}</span>
          </label>
        {/each}
      {/if}
    </section>
  {/if}

  <!-- Panel Footer -->
  <footer>
    <button type="button" onclick={openApp}>
      {$t("usage_tray.panel.open_app") || "Open OmniGet"}
    </button>
    <button type="button" onclick={() => openClaude(null, true)}>
      {$t("usage_tray.panel.add_claude_account") || "Add Account"}
    </button>
    {#if !isAccountsEmpty}
      <button
        type="button"
        class:on={isChooserOpen}
        onclick={() => (isChooserOpen = !isChooserOpen)}
      >
        {$t("usage_tray.panel.choose") || "Choose Accounts"}
      </button>
    {/if}
  </footer>
</main>

<style>
  :global(html, body) {
    margin: 0;
    overflow: hidden;
  }

  .panel {
    --calm: #5cc45d;
    --amber: #f59e35;
    --red: #ff5a36;
    --ink: rgba(255, 255, 255, 0.93);
    --ink-dim: rgba(255, 255, 255, 0.58);
    --glass: rgba(28, 28, 30, 0.97);
    --card: rgba(255, 255, 255, 0.05);
    --edge: rgba(255, 255, 255, 0.1);
    --track: rgba(255, 255, 255, 0.13);
    --hover: rgba(255, 255, 255, 0.09);
    box-sizing: border-box;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-radius: 12px;
    background: var(--glass);
    box-shadow: inset 0 0 0 1px var(--edge);
    color: var(--ink);
    font: 12px/1.35 -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    user-select: none;
    -webkit-user-select: none;
    cursor: default;
  }

  .panel.theme-light {
    --calm: #2f8f3a;
    --amber: #d9780f;
    --red: #d23500;
    --ink: rgba(17, 17, 17, 0.92);
    --ink-dim: rgba(17, 17, 17, 0.56);
    --glass: rgba(253, 246, 236, 0.98);
    --card: rgba(29, 90, 60, 0.06);
    --edge: rgba(17, 17, 17, 0.1);
    --track: rgba(17, 17, 17, 0.1);
    --hover: rgba(17, 17, 17, 0.07);
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .header-icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    color: var(--calm);
  }

  h1 {
    flex: 1;
    margin: 0;
    font-size: 14px;
    font-weight: 650;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 13px;
    font-weight: 600;
  }
  p { margin: 0; }
  .dim { color: var(--ink-dim); }

  button {
    font: inherit;
    color: inherit;
    background: var(--card);
    border: 0;
    border-radius: 7px;
    padding: 5px 9px;
    box-shadow: inset 0 0 0 1px var(--edge);
    cursor: pointer;
  }
  button:hover,
  button.on {
    background: var(--hover);
  }

  .icon-btn {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
  }

  .link {
    margin-left: auto;
    padding: 2px 7px;
    color: var(--calm);
    background: transparent;
    box-shadow: none;
  }
  .link:hover {
    text-decoration: underline;
  }

  .accounts {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border-radius: 10px;
    background: var(--card);
  }

  .who {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .plan {
    margin-left: auto;
    color: var(--ink-dim);
    text-transform: capitalize;
  }

  .badge {
    flex: none;
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 99px;
    background: rgba(217, 119, 87, 0.2);
    color: #e08a67;
  }
  .badge.codex {
    background: rgba(120, 140, 255, 0.18);
    color: #8fa0ff;
  }

  .status {
    font-size: 11px;
    color: var(--ink-dim);
  }
  .status.needs_auth,
  .status.error {
    color: var(--red);
  }

  .meters {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .meter {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .ring {
    flex: none;
    width: 48px;
    height: 48px;
  }
  .ring circle {
    fill: none;
    stroke-width: 5;
  }
  .ring .track {
    stroke: var(--track);
  }
  .ring .fill {
    stroke: var(--calm);
    stroke-linecap: round;
    transition: stroke-dashoffset 0.4s ease;
  }
  .ring.warn .fill {
    stroke: var(--amber);
  }
  .ring.critical .fill {
    stroke: var(--red);
  }
  .ring line {
    stroke-linecap: round;
  }
  .ring .tick {
    stroke: var(--ink);
    stroke-width: 1.4;
  }
  .ring .tick-under {
    stroke: var(--glass);
    stroke-width: 3;
  }
  .ring text {
    fill: var(--ink);
    font-size: 12px;
    font-weight: 650;
  }

  .meter-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    font-size: 11px;
  }
  .meter-label {
    font-weight: 600;
    font-size: 12px;
  }

  .forecast {
    color: var(--amber);
  }
  .forecast.critical {
    color: var(--red);
  }

  .others {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
  }
  .others li {
    display: grid;
    grid-template-columns: 1fr 70px 34px;
    align-items: center;
    gap: 6px;
  }

  .bar {
    position: relative;
    height: 5px;
    border-radius: 3px;
    background: var(--track);
    overflow: hidden;
  }
  .bar-fill {
    display: block;
    height: 100%;
    background: var(--calm);
  }
  .bar-fill.warn, .bar-fill.amber {
    background: var(--amber);
  }
  .bar-fill.critical, .bar-fill.red {
    background: var(--red);
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
  }

  .chooser {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
    border-radius: 10px;
    background: var(--card);
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 0;
    cursor: pointer;
  }
  .pick input {
    accent-color: var(--calm);
  }

  footer {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
</style>
