<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { t } from "$lib/i18n";
  import {
    onTrackerEvent,
    trackerAdd,
    trackerDetectCarrier,
    trackerList,
    trackerRefresh,
    trackerRefreshAll,
    trackerRemove,
    trackerSetAlias,
    trackerSetArchived,
    type CarrierDetection,
    type TrackedPackage,
  } from "$lib/tracker-bridge";
  import { getCarrierIcon } from "$lib/tracker-carrier-icons";
  import { getStatusIcon } from "$lib/tracker-status-icons";
  import { maskCode, privacyEnabled, togglePrivacy } from "$lib/tracker-privacy.svelte";

  type Toast = { id: number; kind: "info" | "success" | "error"; text: string };
  type Tab = "all" | "in_transit" | "awaiting" | "delivered" | "delayed" | "archived";

  const TAB_STORAGE_KEY = "omniget.tracking.tab";

  // Abas "ativas" (não-arquivados) compartilham a mesma listagem do backend
  // (ListFilter::Active); só "archived" muda o filter. Filtragem entre as
  // ativas é client-side via TAB_BUCKETS.
  function backendFilter(t: Tab): "active" | "archived" {
    return t === "archived" ? "archived" : "active";
  }

  // Buckets de cada aba "ativa", mapeados pra `canonical_status`. Pacotes
  // sem canonical (pré-v2 ainda não backfillado, ou sem nenhum evento) usam
  // o fallback `is_delivered` legado. `archived` não tem bucket — é filtrado
  // direto pelo backend via ListFilter.
  type BucketTab = Exclude<Tab, "all" | "archived">;
  const TAB_BUCKETS: Record<BucketTab, readonly string[]> = {
    in_transit: [
      "preadvice",
      "picked_up",
      "in_transit",
      "in_warehouse",
      "customs",
      "customs_cleared",
      "out_for_delivery",
    ],
    awaiting: ["awaiting_pickup", "customs_held"],
    delivered: ["delivered"],
    delayed: ["delayed", "delivery_failure", "damaged"],
  };

  function inBucket(pkg: TrackedPackage, tab: BucketTab): boolean {
    if (pkg.canonical_status) return TAB_BUCKETS[tab].includes(pkg.canonical_status);
    // Fallback pra pacotes sem canonical: só "in_transit" vs "delivered"
    // funcionam de forma sensata via `is_delivered`.
    if (tab === "delivered") return pkg.is_delivered;
    if (tab === "in_transit") return !pkg.is_delivered;
    return false;
  }

  let items = $state<TrackedPackage[]>([]);
  // Mantém a contagem mais recente de arquivados pra exibir no chip mesmo
  // quando a aba ativa está em "Ativos". Atualizada toda vez que `refresh()`
  // roda na aba archived ou via heartbeat opcional (Sessão 9).
  let archivedCount = $state<number | null>(null);
  let loading = $state(true);
  let query = $state("");
  let tab = $state<Tab>("all");
  let lastError = $state<string | null>(null);
  let refreshingAll = $state(false);
  let busyIds = $state<Record<number, boolean>>({});
  let menuOpenId = $state<number | null>(null);

  let confirmTarget = $state<TrackedPackage | null>(null);
  let confirmRemoving = $state(false);

  let showAddDialog = $state(false);
  let addCode = $state("");
  let addAlias = $state("");
  let addCpf = $state("");
  let addDetect = $state<CarrierDetection | null>(null);
  let addDetecting = $state(false);
  let addSubmitting = $state(false);
  let addError = $state<string | null>(null);
  let codeInputEl: HTMLInputElement | undefined = $state();

  let toasts = $state<Toast[]>([]);
  let toastSeq = 0;
  let detectDebounce: number | null = null;

  let unlistenStarted: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenStatus: UnlistenFn | null = null;

  const inTransitCount = $derived(items.filter((p) => inBucket(p, "in_transit")).length);
  const awaitingCount = $derived(items.filter((p) => inBucket(p, "awaiting")).length);
  const deliveredCount = $derived(items.filter((p) => inBucket(p, "delivered")).length);
  const delayedCount = $derived(items.filter((p) => inBucket(p, "delayed")).length);

  const filtered = $derived.by(() => {
    let list = items;
    // Em "archived", a listagem do backend já vem filtrada por
    // `is_archived = 1`; client-side só aplica a busca.
    const t = tab;
    if (t !== "all" && t !== "archived") list = list.filter((p) => inBucket(p, t));
    const q = query.trim().toLowerCase();
    if (!q) return list;
    return list.filter((p) => {
      const hay = [p.code, p.alias ?? "", p.last_status ?? ""].join(" ").toLowerCase();
      return hay.includes(q);
    });
  });

  onMount(() => {
    try {
      const saved = localStorage.getItem(TAB_STORAGE_KEY);
      if (
        saved === "all" ||
        saved === "in_transit" ||
        saved === "awaiting" ||
        saved === "delivered" ||
        saved === "delayed" ||
        saved === "archived"
      ) {
        tab = saved;
      }
    } catch {
      // localStorage may be unavailable (private mode, etc.) — fall back to default
    }
    void refresh();
    void initListeners();
    document.addEventListener("keydown", onGlobalKey);
    document.addEventListener("click", onGlobalClick);
  });

  function setTab(next: Tab) {
    const prev = tab;
    tab = next;
    try {
      localStorage.setItem(TAB_STORAGE_KEY, next);
    } catch {
      // ignore — non-fatal
    }
    // Trocar entre Ativos↔Arquivados muda o filter do backend; refetch.
    if (backendFilter(prev) !== backendFilter(next)) void refresh();
  }

  onDestroy(() => {
    unlistenStarted?.();
    unlistenDone?.();
    unlistenStatus?.();
    document.removeEventListener("keydown", onGlobalKey);
    document.removeEventListener("click", onGlobalClick);
  });

  function onGlobalKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (confirmTarget) confirmTarget = null;
      else if (showAddDialog) closeAddDialog();
      else if (menuOpenId != null) menuOpenId = null;
    }
  }

  function onGlobalClick(e: MouseEvent) {
    if (menuOpenId == null) return;
    const t = e.target as HTMLElement | null;
    if (t && !t.closest(".kebab-wrap")) menuOpenId = null;
  }

  async function initListeners() {
    unlistenStarted = await onTrackerEvent("refresh-started", () => {
      // no-op: per-card spinner already managed by busyIds when user-initiated
    });
    unlistenDone = await onTrackerEvent("refresh-done", async (p) => {
      busyIds = { ...busyIds, [p.id]: false };
      if (p.error) {
        pushToast(
          "error",
          $t("tracking.toast.refresh_failed", { error: truncate(p.error, 80) }),
        );
      }
      await refresh();
    });
    unlistenStatus = await onTrackerEvent("status-changed", async (p) => {
      pushToast(
        "success",
        $t("tracking.toast.status_changed", { code: p.code, status: p.new_status }),
      );
      await refresh();
    });
  }

  async function refresh() {
    loading = true;
    lastError = null;
    const want = backendFilter(tab);
    try {
      // Quando a aba ativa é "archived", a chamada principal já devolve a
      // contagem. Caso contrário, faço uma 2ª chamada barata só pra contar
      // arquivados — SQLite local, lista pequena, sem custo perceptível.
      if (want === "archived") {
        const list = await trackerList("archived");
        items = list;
        archivedCount = list.length;
      } else {
        const [active, archived] = await Promise.all([
          trackerList("active"),
          trackerList("archived"),
        ]);
        items = active;
        archivedCount = archived.length;
      }
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function onRefreshAll() {
    if (refreshingAll) return;
    refreshingAll = true;
    try {
      await trackerRefreshAll();
      await refresh();
      pushToast("info", $t("tracking.toast.refresh_done"));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (looksLikeOffline(msg)) {
        pushToast("error", $t("tracking.toast.offline"));
      } else {
        pushToast("error", msg);
      }
    } finally {
      refreshingAll = false;
    }
  }

  function looksLikeOffline(msg: string): boolean {
    const m = msg.toLowerCase();
    return (
      m.includes("no network") ||
      m.includes("offline") ||
      m.includes("network is unreachable") ||
      m.includes("dns") ||
      m.includes("name resolution") ||
      m.includes("failed to lookup") ||
      m.includes("connection refused") ||
      m.includes("os error 11001")
    );
  }

  async function onRefreshOne(id: number) {
    if (busyIds[id]) return;
    busyIds = { ...busyIds, [id]: true };
    menuOpenId = null;
    try {
      await trackerRefresh(id);
      await refresh();
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      busyIds = { ...busyIds, [id]: false };
    }
  }

  function onRemove(pkg: TrackedPackage) {
    menuOpenId = null;
    confirmTarget = pkg;
  }

  async function onUnarchive(pkg: TrackedPackage) {
    menuOpenId = null;
    try {
      await trackerSetArchived(pkg.id, false);
      await refresh();
      pushToast("info", $t("tracking.toast.unarchived"));
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function confirmRemove() {
    const pkg = confirmTarget;
    if (!pkg || confirmRemoving) return;
    confirmRemoving = true;
    try {
      await trackerRemove(pkg.id);
      confirmTarget = null;
      await refresh();
      pushToast("info", $t("tracking.toast.package_removed"));
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      confirmRemoving = false;
    }
  }

  async function copyCode(code: string) {
    try {
      await navigator.clipboard.writeText(code);
      pushToast("info", $t("tracking.toast.copied"));
    } catch {
      pushToast("error", $t("tracking.toast.copy_failed"));
    }
  }

  async function onRename(pkg: TrackedPackage) {
    menuOpenId = null;
    const next = prompt($t("tracking.rename_prompt"), pkg.alias ?? "");
    if (next == null) return;
    const trimmed = next.trim();
    try {
      await trackerSetAlias(pkg.id, trimmed.length ? trimmed : null);
      await refresh();
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  function openAddDialog() {
    addCode = "";
    addAlias = "";
    addCpf = "";
    addDetect = null;
    addError = null;
    showAddDialog = true;
    void tick().then(() => codeInputEl?.focus());
  }

  function closeAddDialog() {
    showAddDialog = false;
    if (detectDebounce != null) {
      clearTimeout(detectDebounce);
      detectDebounce = null;
    }
  }

  function onCodeInput() {
    addDetect = null;
    if (detectDebounce != null) clearTimeout(detectDebounce);
    const code = addCode.trim();
    if (code.length < 4) return;
    addDetecting = true;
    detectDebounce = window.setTimeout(async () => {
      try {
        addDetect = await trackerDetectCarrier(code);
      } catch {
        addDetect = null;
      } finally {
        addDetecting = false;
      }
    }, 300);
  }

  async function onSubmitAdd(e: Event) {
    e.preventDefault();
    if (addSubmitting) return;
    const code = addCode.trim();
    if (!code) {
      addError = $t("tracking.add_dialog.error_code_required");
      return;
    }
    if (addDetect?.requires_cpf && !addCpf.trim()) {
      addError = $t("tracking.add_dialog.error_cpf_required");
      return;
    }
    addSubmitting = true;
    addError = null;
    try {
      const alias = addAlias.trim();
      const cpf = addCpf.trim();
      await trackerAdd({
        code,
        alias: alias.length ? alias : undefined,
        cpf: cpf.length ? cpf : undefined,
      });
      closeAddDialog();
      pushToast("info", $t("tracking.toast.added"));
      await refresh();
    } catch (e) {
      addError = e instanceof Error ? e.message : String(e);
    } finally {
      addSubmitting = false;
    }
  }

  function pushToast(kind: Toast["kind"], text: string) {
    const id = ++toastSeq;
    toasts = [...toasts, { id, kind, text }];
    setTimeout(() => {
      toasts = toasts.filter((t) => t.id !== id);
    }, 4500);
  }

  function dismissToast(id: number) {
    toasts = toasts.filter((t) => t.id !== id);
  }

  function fmtDaysInTransit(s: string | null): string {
    if (!s) return "";
    const n = parseInt(s, 10);
    if (!Number.isFinite(n)) return s;
    return n === 1 ? $t("tracking.card.days_one") : $t("tracking.card.days_other", { count: n });
  }

  function fmtRel(ms: number | null): string {
    if (!ms) return "—";
    const diff = (Date.now() - ms) / 1000;
    if (diff < 60) return $t("tracking.rel.now");
    if (diff < 3600) return $t("tracking.rel.minutes", { count: Math.floor(diff / 60) });
    if (diff < 86400) return $t("tracking.rel.hours", { count: Math.floor(diff / 3600) });
    return new Intl.DateTimeFormat(undefined, { day: "2-digit", month: "short" }).format(new Date(ms));
  }

  function truncate(s: string, max: number): string {
    return s.length <= max ? s : s.slice(0, max - 1) + "…";
  }

  function truncateCode(code: string): string {
    return code.length <= 18 ? code : code.slice(0, 10) + "…" + code.slice(-4);
  }
</script>

<section class="tracking-page">
  <header class="head">
    <div class="head-text">
      <h1>{$t("tracking.title")}</h1>
      <p class="subtitle">
        {#if items.length > 0}
          {$t("tracking.subtitle_summary", {
            count: items.length,
            in_transit: items.filter((p) => !p.is_delivered).length,
          })}
        {:else}
          {$t("tracking.subtitle_empty")}
        {/if}
      </p>
    </div>
    <div class="head-actions">
      <button
        type="button"
        class="privacy-btn"
        onclick={togglePrivacy}
        aria-pressed={privacyEnabled()}
        title={privacyEnabled() ? $t("tracking.privacy_show_title") : $t("tracking.privacy_hide_title")}
      >
        {#if privacyEnabled()}
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
            <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
            <path d="M14.12 14.12A3 3 0 1 1 9.88 9.88"/>
            <line x1="1" y1="1" x2="23" y2="23"/>
          </svg>
        {:else}
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
            <circle cx="12" cy="12" r="3"/>
          </svg>
        {/if}
        <span>{$t("tracking.privacy_button")}</span>
      </button>
      <button
        type="button"
        class="btn-secondary"
        onclick={onRefreshAll}
        disabled={refreshingAll || items.length === 0}
        title={$t("tracking.refresh_all_title")}
      >
        {refreshingAll ? $t("tracking.refresh_all_busy") : $t("tracking.refresh_all")}
      </button>
      <a
        class="settings-link"
        href="/misc/tracking/settings"
        title={$t("tracking.settings.title")}
        aria-label={$t("tracking.settings.title")}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h0a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h0a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v0a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </a>
      <button type="button" class="btn-primary" onclick={openAddDialog}>{$t("tracking.add_button")}</button>
    </div>
  </header>

  {#if items.length > 0}
    <input
      type="search"
      class="search-input"
      placeholder={$t("tracking.search_placeholder")}
      bind:value={query}
    />
    <div class="kind-chips" role="tablist" aria-label={$t("tracking.title")}>
      <button
        type="button"
        class="chip"
        class:active={tab === "all"}
        role="tab"
        aria-selected={tab === "all"}
        onclick={() => setTab("all")}
      >
        {$t("tracking.tab.all")} ({items.length})
      </button>
      <button
        type="button"
        class="chip"
        class:active={tab === "in_transit"}
        role="tab"
        aria-selected={tab === "in_transit"}
        onclick={() => setTab("in_transit")}
      >
        {$t("tracking.tab.in_transit")} ({inTransitCount})
      </button>
      {#if awaitingCount > 0}
        <button
          type="button"
          class="chip"
          class:active={tab === "awaiting"}
          role="tab"
          aria-selected={tab === "awaiting"}
          onclick={() => setTab("awaiting")}
        >
          {$t("tracking.tab.awaiting")} ({awaitingCount})
        </button>
      {/if}
      <button
        type="button"
        class="chip"
        class:active={tab === "delivered"}
        role="tab"
        aria-selected={tab === "delivered"}
        onclick={() => setTab("delivered")}
      >
        {$t("tracking.tab.delivered")} ({deliveredCount})
      </button>
      {#if delayedCount > 0}
        <button
          type="button"
          class="chip"
          class:active={tab === "delayed"}
          role="tab"
          aria-selected={tab === "delayed"}
          onclick={() => setTab("delayed")}
        >
          {$t("tracking.tab.delayed")} ({delayedCount})
        </button>
      {/if}
      {#if archivedCount != null && archivedCount > 0}
        <button
          type="button"
          class="chip"
          class:active={tab === "archived"}
          role="tab"
          aria-selected={tab === "archived"}
          onclick={() => setTab("archived")}
        >
          {$t("tracking.tab.archived")} ({archivedCount})
        </button>
      {/if}
    </div>
  {/if}

  {#if lastError}
    <div class="error" role="alert"><strong>{$t("tracking.error_prefix")}:</strong> {lastError}</div>
  {/if}

  {#if loading}
    <p class="empty">{$t("tracking.loading")}</p>
  {:else if items.length === 0}
    <div class="empty-state">
      <div class="empty-icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
          <path d="M3.27 6.96 12 12.01l8.73-5.05" />
          <path d="M12 22.08V12" />
        </svg>
      </div>
      <h2 class="empty-title">{$t("tracking.empty.none_title")}</h2>
      <p class="empty-desc">{$t("tracking.empty.none_desc")}</p>
      <button type="button" class="btn-primary" onclick={openAddDialog}>{$t("tracking.empty.none_cta")}</button>
    </div>
  {:else if filtered.length === 0}
    {#if query.trim()}
      <p class="empty">{$t("tracking.empty.search", { query })}</p>
    {:else if tab === "in_transit"}
      <p class="empty">{$t("tracking.empty.in_transit")}</p>
    {:else if tab === "awaiting"}
      <p class="empty">{$t("tracking.empty.awaiting")}</p>
    {:else if tab === "delivered"}
      <p class="empty">{$t("tracking.empty.delivered")}</p>
    {:else if tab === "delayed"}
      <p class="empty">{$t("tracking.empty.delayed")}</p>
    {:else if tab === "archived"}
      <p class="empty">{$t("tracking.empty.archived")}</p>
    {:else}
      <p class="empty">{$t("tracking.empty.nothing")}</p>
    {/if}
  {:else}
    <div class="grid">
      {#each filtered as pkg (pkg.id)}
        {@const icon = getCarrierIcon(pkg.carrier)}
        {@const carrierLabel = icon.labelKey ? $t(icon.labelKey) : icon.label}
        {@const status = getStatusIcon(pkg.canonical_status)}
        {@const statusLabel = $t(status.labelKey)}
        {@const title = pkg.alias?.trim() || maskCode(pkg.code)}
        <article class="card" class:delivered={pkg.is_delivered} class:archived={pkg.is_archived}>
          {#if pkg.is_archived}
            <span class="archived-badge" aria-label={$t("tracking.card.archived_label")}>{$t("tracking.card.archived_label")}</span>
          {/if}
          <a class="card-link" href="/misc/tracking/{pkg.id}" aria-label={title}>
            <div class="card-head">
              <div class="carrier-icon" style="--carrier-color: {icon.color}" title={carrierLabel} aria-hidden="true">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                  <path d={icon.svgPath} />
                </svg>
              </div>
              <div class="card-titles">
                <span class="card-title" title={title}>{title}</span>
                {#if pkg.alias?.trim()}
                  <span class="card-code mono" title={maskCode(pkg.code)}>{truncateCode(maskCode(pkg.code))}</span>
                {:else}
                  <span class="card-code mono carrier-label">{carrierLabel}</span>
                {/if}
              </div>
              {#if pkg.canonical_status}
                <span
                  class="status-icon"
                  style="--status-color: {status.color}"
                  title={statusLabel}
                  aria-label={statusLabel}
                >
                  <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <path d={status.svgPath} />
                  </svg>
                </span>
              {/if}
            </div>

            <div class="card-body">
              {#if pkg.canonical_status}
                <span class="status-badge" style="--status-color: {status.color}">
                  {statusLabel}
                </span>
              {:else if pkg.last_status}
                <span class="status-badge">{pkg.last_status}</span>
              {:else}
                <span class="status-badge pending">{$t("tracking.card.awaiting_first")}</span>
              {/if}
            </div>

            <div class="card-foot">
              <span class="meta-bit">
                {#if pkg.is_delivered}
                  {$t("tracking.card.delivered")}
                {:else if pkg.days_in_transit}
                  {fmtDaysInTransit(pkg.days_in_transit)}
                {:else}
                  {$t("tracking.card.no_events")}
                {/if}
              </span>
              <span class="meta-bit">{fmtRel(pkg.last_event_at_ms ?? pkg.updated_at_ms)}</span>
            </div>
          </a>

          <div class="kebab-wrap">
            <button
              type="button"
              class="kebab"
              aria-label={$t("tracking.menu.refresh")}
              aria-haspopup="menu"
              aria-expanded={menuOpenId === pkg.id}
              onclick={(e) => {
                e.stopPropagation();
                menuOpenId = menuOpenId === pkg.id ? null : pkg.id;
              }}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><circle cx="5" cy="12" r="1.6"/><circle cx="12" cy="12" r="1.6"/><circle cx="19" cy="12" r="1.6"/></svg>
            </button>
            {#if menuOpenId === pkg.id}
              <div class="menu" role="menu">
                {#if !pkg.is_archived}
                  <button type="button" class="menu-item" role="menuitem" onclick={() => onRefreshOne(pkg.id)} disabled={busyIds[pkg.id]}>
                    {busyIds[pkg.id] ? $t("tracking.menu.refreshing") : $t("tracking.menu.refresh")}
                  </button>
                {/if}
                <button type="button" class="menu-item" role="menuitem" onclick={() => { menuOpenId = null; void copyCode(pkg.code); }}>
                  {$t("tracking.menu.copy_code")}
                </button>
                <button type="button" class="menu-item" role="menuitem" onclick={() => onRename(pkg)}>
                  {$t("tracking.menu.rename")}
                </button>
                {#if pkg.is_archived}
                  <button type="button" class="menu-item" role="menuitem" onclick={() => onUnarchive(pkg)}>
                    {$t("tracking.menu.unarchive")}
                  </button>
                {/if}
                <button type="button" class="menu-item danger" role="menuitem" onclick={() => onRemove(pkg)}>
                  {$t("tracking.menu.remove")}
                </button>
              </div>
            {/if}
          </div>

          {#if busyIds[pkg.id]}
            <div class="card-busy" aria-hidden="true"></div>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>

{#if showAddDialog}
  <div
    class="dialog-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="add-dialog-title"
    onclick={(e) => { if (e.target === e.currentTarget) closeAddDialog(); }}
  >
    <form class="dialog" onsubmit={onSubmitAdd}>
      <header class="dialog-head">
        <h2 id="add-dialog-title">{$t("tracking.add_dialog.title")}</h2>
        <button type="button" class="dialog-close" onclick={closeAddDialog} aria-label={$t("tracking.add_dialog.close")}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6 6 18 M6 6l12 12"/></svg>
        </button>
      </header>

      <div class="dialog-body">
        <label class="field">
          <span class="field-label">{$t("tracking.add_dialog.code_label")}</span>
          <input
            bind:this={codeInputEl}
            type="text"
            class="field-input mono"
            placeholder={$t("tracking.add_dialog.code_placeholder")}
            bind:value={addCode}
            oninput={onCodeInput}
            autocomplete="off"
            spellcheck="false"
            required
          />
          {#if addDetecting}
            <span class="field-hint">{$t("tracking.add_dialog.detecting")}</span>
          {:else if addDetect}
            <span class="field-hint">
              {$t("tracking.add_dialog.detected", { carrier: addDetect.display_name })}
              {#if addDetect.requires_cpf}
                {$t("tracking.add_dialog.requires_cpf_suffix")}
              {/if}
            </span>
          {/if}
        </label>

        <label class="field">
          <span class="field-label">{$t("tracking.add_dialog.alias_label")}</span>
          <input
            type="text"
            class="field-input"
            placeholder={$t("tracking.add_dialog.alias_placeholder")}
            bind:value={addAlias}
            autocomplete="off"
          />
        </label>

        {#if addDetect?.requires_cpf}
          <label class="field">
            <span class="field-label">{$t("tracking.add_dialog.cpf_label")}</span>
            <input
              type="text"
              class="field-input mono"
              placeholder={$t("tracking.add_dialog.cpf_placeholder")}
              bind:value={addCpf}
              autocomplete="off"
              required
            />
            <span class="field-hint">{$t("tracking.add_dialog.cpf_hint")}</span>
          </label>
        {/if}

        {#if addError}
          <div class="error" role="alert">{addError}</div>
        {/if}
      </div>

      <footer class="dialog-foot">
        <button type="button" class="btn-secondary" onclick={closeAddDialog} disabled={addSubmitting}>
          {$t("tracking.add_dialog.cancel")}
        </button>
        <button type="submit" class="btn-primary" disabled={addSubmitting || !addCode.trim()}>
          {addSubmitting ? $t("tracking.add_dialog.submitting") : $t("tracking.add_dialog.submit")}
        </button>
      </footer>
    </form>
  </div>
{/if}

{#if confirmTarget}
  {@const target = confirmTarget}
  <div
    class="dialog-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-remove-title"
    onclick={(e) => { if (e.target === e.currentTarget && !confirmRemoving) confirmTarget = null; }}
  >
    <div class="dialog confirm-dialog">
      <header class="dialog-head">
        <h2 id="confirm-remove-title">{$t("tracking.remove_dialog.title")}</h2>
        <button
          type="button"
          class="dialog-close"
          onclick={() => (confirmTarget = null)}
          disabled={confirmRemoving}
          aria-label={$t("tracking.remove_dialog.close")}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6 6 18 M6 6l12 12"/></svg>
        </button>
      </header>
      <div class="dialog-body">
        <p class="confirm-msg">
          {$t("tracking.remove_dialog.question", { name: target.alias?.trim() || maskCode(target.code) })}
        </p>
        <p class="confirm-sub">{$t("tracking.remove_dialog.sub")}</p>
      </div>
      <footer class="dialog-foot">
        <button type="button" class="btn-secondary" onclick={() => (confirmTarget = null)} disabled={confirmRemoving}>
          {$t("tracking.remove_dialog.cancel")}
        </button>
        <button type="button" class="btn-danger" onclick={confirmRemove} disabled={confirmRemoving}>
          {confirmRemoving ? $t("tracking.remove_dialog.confirming") : $t("tracking.remove_dialog.confirm")}
        </button>
      </footer>
    </div>
  </div>
{/if}

{#if toasts.length > 0}
  <div class="toast-stack" aria-live="polite" aria-atomic="false">
    {#each toasts as t (t.id)}
      <button
        type="button"
        class="toast toast-{t.kind}"
        onclick={() => dismissToast(t.id)}
      >
        {t.text}
      </button>
    {/each}
  </div>
{/if}

<style>
  .tracking-page {
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 2);
    width: 100%;
    max-width: 1100px;
    margin-inline: auto;
    padding: 16px 20px 80px;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  .head-text {
    min-width: 0;
  }
  .head h1 {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 500;
    letter-spacing: -0.5px;
    color: var(--secondary);
  }
  .subtitle {
    margin: 0;
    font-size: 13px;
    color: var(--tertiary);
  }
  .head-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .search-input {
    width: 100%;
    padding: 9px 12px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
    color: var(--secondary);
    font-size: 13px;
    font-family: inherit;
  }
  .search-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .kind-chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .chip {
    padding: 5px 12px;
    border: 1px solid var(--input-border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--tertiary);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    transition: border-color 120ms ease, color 120ms ease, background 120ms ease;
  }
  .chip:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .chip.active {
    border-color: var(--accent);
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 12%, var(--surface));
  }

  .error {
    padding: 10px 14px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--danger) 14%, transparent);
    color: var(--danger);
    font-size: 13px;
  }

  .empty {
    color: var(--tertiary);
    font-size: 13px;
    margin: 0;
    padding: 20px 0;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 48px 20px;
    text-align: center;
    border: 1px dashed var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
  }
  .empty-icon {
    color: var(--accent);
    opacity: 0.7;
    margin-bottom: 4px;
  }
  .empty-title {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
    color: var(--secondary);
  }
  .empty-desc {
    margin: 0 0 12px;
    font-size: 13px;
    color: var(--tertiary);
    max-width: 320px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }

  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
    transition: border-color 150ms ease, transform 100ms ease;
  }
  .card:hover {
    border-color: var(--accent);
    transform: translateY(-1px);
  }
  .card.delivered {
    opacity: 0.78;
  }
  .card.archived {
    opacity: 0.6;
    background: color-mix(in oklab, var(--surface) 92%, var(--tertiary));
  }
  .card.archived:hover {
    opacity: 0.85;
  }
  .archived-badge {
    position: absolute;
    bottom: 10px;
    right: 10px;
    z-index: 2;
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--tertiary) 24%, transparent);
    color: var(--tertiary);
    font-size: 10px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    pointer-events: none;
  }

  .card-link {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    text-decoration: none;
    color: inherit;
    min-width: 0;
  }

  .card-head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .carrier-icon {
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: color-mix(in oklab, var(--carrier-color) 16%, transparent);
    color: var(--carrier-color);
  }
  .card-titles {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .card-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-code {
    font-size: 11px;
    color: var(--tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .carrier-label {
    font-family: inherit;
  }
  .status-icon {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: color-mix(in oklab, var(--status-color) 18%, transparent);
    color: var(--status-color);
  }

  .card-body {
    display: flex;
    align-items: center;
  }
  .status-badge {
    --status-color: var(--accent);
    display: inline-block;
    padding: 4px 9px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--status-color) 14%, transparent);
    color: var(--status-color);
    font-size: 11px;
    font-weight: 500;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status-badge.pending {
    background: color-mix(in oklab, var(--input-border) 50%, transparent);
    color: var(--tertiary);
  }

  .card-foot {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 11px;
    color: var(--tertiary);
  }
  .meta-bit {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .kebab-wrap {
    position: absolute;
    top: 10px;
    right: 10px;
  }
  .kebab {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    color: var(--tertiary);
    cursor: pointer;
    border-radius: 6px;
  }
  .kebab:hover {
    background: color-mix(in oklab, var(--input-border) 50%, transparent);
    color: var(--secondary);
  }
  .menu {
    position: absolute;
    top: 30px;
    right: 0;
    min-width: 140px;
    background: var(--surface);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    box-shadow: 0 4px 18px color-mix(in oklab, black 18%, transparent);
    display: flex;
    flex-direction: column;
    padding: 4px;
    z-index: 10;
  }
  .menu-item {
    padding: 7px 10px;
    background: transparent;
    border: none;
    text-align: left;
    color: var(--secondary);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    border-radius: 4px;
  }
  .menu-item:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 12%, transparent);
  }
  .menu-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .menu-item.danger {
    color: var(--danger);
  }
  .menu-item.danger:hover:not(:disabled) {
    background: color-mix(in oklab, var(--danger) 12%, transparent);
  }

  .card-busy {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: color-mix(in oklab, var(--accent) 5%, transparent);
    border-radius: var(--border-radius);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.3; }
    50% { opacity: 0.7; }
  }

  .settings-link {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 999px;
    border: 1px solid var(--input-border);
    background: var(--surface);
    color: var(--tertiary);
    text-decoration: none;
    cursor: pointer;
  }
  .settings-link:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .privacy-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 6px 10px;
    border-radius: 999px;
    border: 1px solid var(--input-border);
    background: var(--surface);
    color: var(--tertiary);
    font-size: 11px;
    font-family: inherit;
    font-weight: 500;
    cursor: pointer;
  }
  .privacy-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .privacy-btn[aria-pressed="true"] {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    border-color: var(--accent);
    color: var(--accent);
  }

  .btn-primary,
  .btn-secondary,
  .btn-danger {
    padding: 7px 14px;
    border-radius: var(--border-radius);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    font-weight: 500;
  }
  .btn-primary {
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }
  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.05);
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .btn-secondary {
    border: 1px solid var(--input-border);
    background: transparent;
    color: var(--secondary);
  }
  .btn-secondary:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .btn-danger {
    border: 1px solid color-mix(in oklab, var(--danger) 60%, var(--input-border));
    background: transparent;
    color: var(--danger);
  }
  .btn-danger:hover:not(:disabled) {
    background: color-mix(in oklab, var(--danger) 14%, transparent);
  }
  .btn-danger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .confirm-dialog {
    max-width: 380px;
  }
  .confirm-msg {
    margin: 0 0 6px;
    font-size: 14px;
    color: var(--secondary);
    line-height: 1.4;
  }
  .confirm-sub {
    margin: 0;
    font-size: 12px;
    color: var(--tertiary);
  }

  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: color-mix(in oklab, black 50%, transparent);
    display: grid;
    place-items: center;
    padding: 16px;
    z-index: 100;
  }
  .dialog {
    width: 100%;
    max-width: 440px;
    background: var(--surface);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 32px);
  }
  .dialog-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--input-border);
  }
  .dialog-head h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 500;
    color: var(--secondary);
  }
  .dialog-close {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    color: var(--tertiary);
    cursor: pointer;
    border-radius: 6px;
  }
  .dialog-close:hover {
    background: color-mix(in oklab, var(--input-border) 50%, transparent);
    color: var(--secondary);
  }
  .dialog-body {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow-y: auto;
  }
  .dialog-foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--input-border);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .field-label {
    font-size: 12px;
    color: var(--tertiary);
  }
  .field-input {
    width: 100%;
    padding: 8px 11px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
    color: var(--secondary);
    font-size: 13px;
    font-family: inherit;
  }
  .field-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .field-hint {
    font-size: 11px;
    color: var(--tertiary);
  }

  .toast-stack {
    position: fixed;
    top: 16px;
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 200;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    padding: 10px 14px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
    color: var(--secondary);
    font-size: 12px;
    font-family: inherit;
    text-align: left;
    box-shadow: 0 4px 14px color-mix(in oklab, black 15%, transparent);
    max-width: 320px;
    cursor: pointer;
    animation: toast-in 180ms ease-out;
  }
  .toast-success {
    border-color: color-mix(in oklab, var(--success) 50%, var(--input-border));
  }
  .toast-error {
    border-color: color-mix(in oklab, var(--danger) 50%, var(--input-border));
    color: var(--danger);
  }
  @keyframes toast-in {
    from { transform: translateY(-6px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  .mono {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-variant-numeric: tabular-nums;
  }

  @media (max-width: 480px) {
    .head {
      flex-direction: column;
      align-items: stretch;
    }
    .head-actions {
      justify-content: stretch;
    }
    .head-actions .btn-primary,
    .head-actions .btn-secondary {
      flex: 1;
    }
    .grid {
      grid-template-columns: 1fr;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .card {
      transition: none;
    }
    .card:hover {
      transform: none;
    }
    .card-busy {
      animation: none;
    }
    .toast {
      animation: none;
    }
  }
</style>
