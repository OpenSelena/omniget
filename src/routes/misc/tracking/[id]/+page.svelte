<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { t } from "$lib/i18n";
  import {
    onTrackerEvent,
    trackerDismissArchivePrompt,
    trackerGet,
    trackerRefresh,
    trackerRemove,
    trackerSetAlias,
    trackerSetArchived,
    trackerSetCpf,
    type TrackedPackage,
    type TrackingEvent,
  } from "$lib/tracker-bridge";
  import { getCarrierIcon, getCarrierRoute } from "$lib/tracker-carrier-icons";
  import { classifyEvent, getStatusIcon } from "$lib/tracker-status-icons";
  import { maskCode, privacyEnabled, togglePrivacy } from "$lib/tracker-privacy.svelte";

  type Toast = { id: number; kind: "info" | "success" | "error"; text: string };

  const id = $derived(parseInt(page.params.id ?? "", 10));

  let pkg = $state<TrackedPackage | null>(null);
  let events = $state<TrackingEvent[]>([]);
  let loading = $state(true);
  let notFound = $state(false);
  let lastError = $state<string | null>(null);
  let refreshing = $state(false);

  let showCpfDialog = $state(false);
  let cpfValue = $state("");
  let cpfSubmitting = $state(false);
  let cpfError = $state<string | null>(null);

  let showRemoveConfirm = $state(false);
  let removing = $state(false);

  let toasts = $state<Toast[]>([]);
  let toastSeq = 0;

  let unlistenStatus: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  $effect(() => {
    if (Number.isFinite(id)) {
      void load(id);
    } else {
      notFound = true;
      loading = false;
    }
  });

  onMount(async () => {
    document.addEventListener("keydown", onGlobalKey);
    unlistenStatus = await onTrackerEvent("status-changed", async (p) => {
      if (p.id !== id) return;
      pushToast("success", p.new_status);
      await load(id);
    });
    unlistenDone = await onTrackerEvent("refresh-done", async (p) => {
      if (p.id !== id) return;
      refreshing = false;
      if (p.error) {
        lastError = p.error;
      } else {
        lastError = null;
        await load(id);
      }
    });
  });

  onDestroy(() => {
    document.removeEventListener("keydown", onGlobalKey);
    unlistenStatus?.();
    unlistenDone?.();
  });

  function onGlobalKey(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    if (showRemoveConfirm && !removing) showRemoveConfirm = false;
    else if (showCpfDialog && !cpfSubmitting) showCpfDialog = false;
  }

  async function load(packageId: number) {
    if (!Number.isFinite(packageId)) {
      notFound = true;
      loading = false;
      return;
    }
    try {
      const res = await trackerGet(packageId);
      if (packageId !== id) return;
      if (!res) {
        notFound = true;
        pkg = null;
        events = [];
      } else {
        notFound = false;
        pkg = res.package;
        events = res.events;
      }
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function onRefresh() {
    if (!pkg || refreshing) return;
    refreshing = true;
    lastError = null;
    try {
      await trackerRefresh(pkg.id);
      await load(pkg.id);
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
      pushToast("error", lastError);
    } finally {
      refreshing = false;
    }
  }

  async function onRename() {
    if (!pkg) return;
    const next = prompt($t("tracking.rename_prompt"), pkg.alias ?? "");
    if (next == null) return;
    const trimmed = next.trim();
    try {
      await trackerSetAlias(pkg.id, trimmed.length ? trimmed : null);
      await load(pkg.id);
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  function onRemove() {
    if (!pkg) return;
    showRemoveConfirm = true;
  }

  async function confirmRemove() {
    if (!pkg || removing) return;
    removing = true;
    try {
      await trackerRemove(pkg.id);
      showRemoveConfirm = false;
      pushToast("info", $t("tracking.toast.package_removed"));
      await goto("/misc/tracking");
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      removing = false;
    }
  }

  async function onArchive() {
    if (!pkg) return;
    try {
      await trackerSetArchived(pkg.id, true);
      pushToast("info", $t("tracking.toast.archived"));
      await goto("/misc/tracking");
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function onUnarchive() {
    if (!pkg) return;
    try {
      await trackerSetArchived(pkg.id, false);
      await load(pkg.id);
      pushToast("info", $t("tracking.toast.unarchived"));
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function onDismissArchivePrompt() {
    if (!pkg) return;
    try {
      await trackerDismissArchivePrompt(pkg.id);
      // Atualiza só a flag local — economiza um roundtrip e mantém o feedback
      // visual imediato (o card amarelo some).
      pkg = { ...pkg, archive_prompt_dismissed: true };
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  }

  function openCpfDialog() {
    if (!pkg) return;
    cpfValue = pkg.cpf ?? "";
    cpfError = null;
    showCpfDialog = true;
  }

  async function submitCpf(e: Event) {
    e.preventDefault();
    if (!pkg || cpfSubmitting) return;
    cpfSubmitting = true;
    cpfError = null;
    try {
      const trimmed = cpfValue.trim();
      await trackerSetCpf(pkg.id, trimmed.length ? trimmed : null);
      showCpfDialog = false;
      await load(pkg.id);
      pushToast("info", $t("tracking.toast.cpf_updated"));
    } catch (err) {
      cpfError = err instanceof Error ? err.message : String(err);
    } finally {
      cpfSubmitting = false;
    }
  }

  async function copyCode() {
    if (!pkg) return;
    try {
      await navigator.clipboard.writeText(pkg.code);
      pushToast("info", $t("tracking.toast.copied"));
    } catch {
      pushToast("error", $t("tracking.toast.copy_failed"));
    }
  }

  function pushToast(kind: Toast["kind"], text: string) {
    const tid = ++toastSeq;
    toasts = [...toasts, { id: tid, kind, text }];
    setTimeout(() => {
      toasts = toasts.filter((t) => t.id !== tid);
    }, 4500);
  }

  function dismissToast(tid: number) {
    toasts = toasts.filter((t) => t.id !== tid);
  }

  function maskCpf(cpf: string | null): string {
    if (!cpf) return "—";
    const digits = cpf.replace(/\D/g, "");
    if (digits.length !== 11) return cpf;
    return `***.***.${digits.slice(6, 9)}-${digits.slice(9)}`;
  }

  function fmtDate(d: string | null | undefined): string {
    if (!d) return "—";
    const iso = /^(\d{4})-(\d{2})-(\d{2})/.exec(d);
    if (iso) return `${iso[3]}/${iso[2]}/${iso[1]}`;
    return d;
  }

  function fmtRel(ms: number | null): string {
    if (!ms) return "—";
    const diff = (Date.now() - ms) / 1000;
    if (diff < 60) return $t("tracking.rel.now");
    if (diff < 3600) return $t("tracking.rel.minutes", { count: Math.floor(diff / 60) });
    if (diff < 86400) return $t("tracking.rel.hours", { count: Math.floor(diff / 3600) });
    return new Intl.DateTimeFormat(undefined, {
      day: "2-digit",
      month: "short",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(ms));
  }

  function fmtDaysPill(s: string | null): string {
    if (!s) return "";
    const n = parseInt(s, 10);
    if (!Number.isFinite(n)) return s;
    return n === 1
      ? $t("tracking.detail.days_one_pill")
      : $t("tracking.detail.days_other_pill", { count: n });
  }

  function truncate(s: string, max: number): string {
    return s.length <= max ? s : s.slice(0, max - 1) + "…";
  }
</script>

<section class="detail">
  <a class="back" href="/misc/tracking">{$t("tracking.detail.back")}</a>

  {#if loading}
    <div class="skeleton" aria-hidden="true">
      <div class="sk-row sk-lg"></div>
      <div class="sk-row sk-md"></div>
      <div class="sk-card"></div>
      <div class="sk-card"></div>
      <div class="sk-card"></div>
    </div>
  {:else if notFound || !pkg}
    <div class="empty-card">
      <h1>{$t("tracking.detail.not_found_title")}</h1>
      <p>{$t("tracking.detail.not_found_body")}</p>
      <a class="btn-primary" href="/misc/tracking">{$t("tracking.detail.not_found_back")}</a>
    </div>
  {:else}
    {@const icon = getCarrierIcon(pkg.carrier)}
    {@const carrierLabel = icon.labelKey ? $t(icon.labelKey) : icon.label}
    {@const status = getStatusIcon(pkg.canonical_status)}
    {@const statusLabel = $t(status.labelKey)}
    {@const route = getCarrierRoute(pkg.carrier)}
    {@const title = pkg.alias?.trim() || pkg.code}
    {@const postedDate = events.length > 0 ? events[events.length - 1].date : null}
    {@const hasAlias = !!pkg.alias?.trim()}

    <header class="hero">
      <div
        class="carrier-icon-lg"
        style="--carrier-color: {icon.color}"
        title={carrierLabel}
        aria-hidden="true"
      >
        <svg
          viewBox="0 0 24 24"
          width="30"
          height="30"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d={icon.svgPath} />
        </svg>
      </div>
      {#if pkg.canonical_status}
        <div
          class="status-icon-lg"
          style="--status-color: {status.color}"
          title={statusLabel}
          aria-label={statusLabel}
        >
          <svg
            viewBox="0 0 24 24"
            width="24"
            height="24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <path d={status.svgPath} />
          </svg>
        </div>
      {/if}
      <div class="hero-text">
        <h1>{hasAlias ? title : maskCode(pkg.code)}</h1>
        <p class="hero-meta">
          <span class="carrier-name" style="color: {icon.color}">{carrierLabel}</span>
          {#if hasAlias}
            <span class="dot-sep">·</span>
            <span class="mono small">{maskCode(pkg.code)}</span>
          {/if}
        </p>
      </div>
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
    </header>

    <div class="status-row">
      {#if pkg.canonical_status}
        <span
          class="status-badge-big"
          class:delivered={pkg.is_delivered}
          style="--status-color: {status.color}"
        >
          <span class="status-badge-label">{statusLabel}</span>
          {#if pkg.last_status && pkg.last_status.toLowerCase() !== statusLabel.toLowerCase()}
            <span class="status-badge-raw">· {pkg.last_status}</span>
          {/if}
        </span>
      {:else}
        <span class="status-badge-big" class:delivered={pkg.is_delivered}>
          {pkg.last_status ?? $t("tracking.detail.awaiting_first")}
        </span>
      {/if}
      {#if !pkg.is_delivered && pkg.days_in_transit}
        <span class="days-pill">{fmtDaysPill(pkg.days_in_transit)}</span>
      {/if}
      {#if pkg.is_delivered}
        <span class="days-pill delivered-pill">{$t("tracking.detail.delivered_pill")}</span>
      {/if}
      {#if pkg.is_archived}
        <span class="days-pill archived-pill">{$t("tracking.detail.archived_pill")}</span>
      {/if}
    </div>

    {#if pkg.canonical_status === "delivered" && !pkg.is_archived && !pkg.archive_prompt_dismissed}
      <aside class="archive-prompt" role="note" aria-label={$t("tracking.archive_prompt.title")}>
        <div class="archive-prompt-text">
          <h3>{$t("tracking.archive_prompt.title")}</h3>
          <p>{$t("tracking.archive_prompt.body")}</p>
        </div>
        <div class="archive-prompt-actions">
          <button type="button" class="btn-secondary" onclick={onDismissArchivePrompt}>
            {$t("tracking.archive_prompt.keep")}
          </button>
          <button type="button" class="btn-primary" onclick={onArchive}>
            {$t("tracking.archive_prompt.archive")}
          </button>
        </div>
      </aside>
    {/if}

    {#if lastError}
      <div class="warn" role="alert">
        <div>
          <strong>{$t("tracking.detail.refresh_failed_title")}</strong>
          <span class="warn-msg">{truncate(lastError, 140)}</span>
        </div>
        <button type="button" class="warn-btn" onclick={onRefresh} disabled={refreshing}>
          {$t("tracking.detail.retry")}
        </button>
      </div>
    {/if}

    <dl class="meta">
      <div class="meta-row">
        <dt>{$t("tracking.detail.meta.code")}</dt>
        <dd>
          <code class="mono">{maskCode(pkg.code)}</code>
          <button type="button" class="icon-btn" onclick={copyCode} title={$t("tracking.detail.meta.copy_code_title")} aria-label={$t("tracking.detail.meta.copy_code_title")}>
            <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
            </svg>
          </button>
        </dd>
      </div>
      {#if pkg.cpf}
        <div class="meta-row">
          <dt>{$t("tracking.detail.meta.cpf")}</dt>
          <dd>
            <span class="mono">{maskCpf(pkg.cpf)}</span>
            <button type="button" class="text-btn" onclick={openCpfDialog}>{$t("tracking.detail.meta.edit")}</button>
          </dd>
        </div>
      {/if}
      {#if route}
        <div class="meta-row">
          <dt>{$t("tracking.detail.meta.route")}</dt>
          <dd>
            <span class="mono">{route.from}</span>
            <span class="route-arrow" aria-hidden="true">→</span>
            <span class="mono">{route.to}</span>
          </dd>
        </div>
      {/if}
      <div class="meta-row">
        <dt>{$t("tracking.detail.meta.posted_at")}</dt>
        <dd>{fmtDate(postedDate)}</dd>
      </div>
      <div class="meta-row">
        <dt>{$t("tracking.detail.meta.updated")}</dt>
        <dd>{fmtRel(pkg.last_event_at_ms ?? pkg.updated_at_ms)}</dd>
      </div>
    </dl>

    <div class="actions">
      <button type="button" class="btn-secondary" onclick={onRefresh} disabled={refreshing || pkg.is_archived}>
        {refreshing ? $t("tracking.detail.action.refreshing") : $t("tracking.detail.action.refresh")}
      </button>
      <button type="button" class="btn-secondary" onclick={onRename}>{$t("tracking.detail.action.rename")}</button>
      {#if pkg.is_archived}
        <button type="button" class="btn-secondary" onclick={onUnarchive}>{$t("tracking.detail.action.unarchive")}</button>
      {:else if pkg.canonical_status === "delivered"}
        <button type="button" class="btn-secondary" onclick={onArchive}>{$t("tracking.detail.action.archive")}</button>
      {:else if !pkg.cpf}
        <button type="button" class="btn-secondary" onclick={openCpfDialog}>{$t("tracking.detail.action.add_cpf")}</button>
      {/if}
      <button type="button" class="btn-danger" onclick={onRemove}>{$t("tracking.detail.action.remove")}</button>
    </div>

    <section class="history">
      <h2>{$t("tracking.detail.history.title")}</h2>
      {#if events.length === 0}
        <p class="muted">{$t("tracking.detail.history.empty")}</p>
      {:else}
        <ol class="timeline">
          {#each events as ev, i (ev.id)}
            {@const evCanonical = classifyEvent(ev.status, ev.location)}
            {@const evIcon = getStatusIcon(evCanonical)}
            {@const evLabel = $t(evIcon.labelKey)}
            {@const samePrev = i > 0 ? classifyEvent(events[i - 1].status, events[i - 1].location) === evCanonical : false}
            <li class="event" class:first={i === 0} class:repeat={samePrev}>
              <div
                class="event-icon"
                style="--status-color: {evIcon.color}"
                title={evLabel}
                aria-label={evLabel}
              >
                <svg
                  viewBox="0 0 24 24"
                  width={samePrev ? "12" : "16"}
                  height={samePrev ? "12" : "16"}
                  fill="none"
                  stroke="currentColor"
                  stroke-width={samePrev ? "1.5" : "1.7"}
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  aria-hidden="true"
                >
                  <path d={evIcon.svgPath} />
                </svg>
              </div>
              <div class="event-body">
                {#if ev.location}
                  <p class="event-desc">{ev.location}</p>
                {/if}
                <p class="event-meta">
                  {#if ev.status}
                    <span class="event-status-chip" style="--status-color: {evIcon.color}">{ev.status}</span>
                  {/if}
                  {#if ev.date}<span>{fmtDate(ev.date)}</span>{/if}
                  {#if ev.time}<span class="dot-sep">·</span><span>{ev.time}</span>{/if}
                </p>
                {#if ev.sub_status && ev.sub_status.length > 0}
                  <ul class="sub-list">
                    {#each ev.sub_status as s}
                      <li>{s}</li>
                    {/each}
                  </ul>
                {/if}
              </div>
            </li>
          {/each}
        </ol>
      {/if}
    </section>
  {/if}
</section>

{#if showCpfDialog && pkg}
  <div
    class="dialog-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="cpf-dialog-title"
    onclick={(e) => { if (e.target === e.currentTarget) showCpfDialog = false; }}
  >
    <form class="dialog" onsubmit={submitCpf}>
      <header class="dialog-head">
        <h2 id="cpf-dialog-title">{pkg.cpf ? $t("tracking.detail.cpf_dialog.title_edit") : $t("tracking.detail.cpf_dialog.title_add")}</h2>
        <button type="button" class="dialog-close" onclick={() => (showCpfDialog = false)} aria-label={$t("tracking.detail.cpf_dialog.close")}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M18 6 6 18 M6 6l12 12"/>
          </svg>
        </button>
      </header>
      <div class="dialog-body">
        <label class="field">
          <span class="field-label">{$t("tracking.detail.cpf_dialog.label")}</span>
          <input
            type="text"
            class="field-input mono"
            placeholder={$t("tracking.detail.cpf_dialog.placeholder")}
            bind:value={cpfValue}
            autocomplete="off"
          />
          <span class="field-hint">{$t("tracking.detail.cpf_dialog.hint")}</span>
        </label>
        {#if cpfError}
          <div class="error" role="alert">{cpfError}</div>
        {/if}
      </div>
      <footer class="dialog-foot">
        <button type="button" class="btn-secondary" onclick={() => (showCpfDialog = false)} disabled={cpfSubmitting}>
          {$t("tracking.detail.cpf_dialog.cancel")}
        </button>
        <button type="submit" class="btn-primary" disabled={cpfSubmitting}>
          {cpfSubmitting ? $t("tracking.detail.cpf_dialog.submitting") : $t("tracking.detail.cpf_dialog.submit")}
        </button>
      </footer>
    </form>
  </div>
{/if}

{#if showRemoveConfirm && pkg}
  {@const label = pkg.alias?.trim() || maskCode(pkg.code)}
  <div
    class="dialog-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="remove-confirm-title"
    onclick={(e) => { if (e.target === e.currentTarget && !removing) showRemoveConfirm = false; }}
  >
    <div class="dialog confirm-dialog">
      <header class="dialog-head">
        <h2 id="remove-confirm-title">{$t("tracking.remove_dialog.title")}</h2>
        <button
          type="button"
          class="dialog-close"
          onclick={() => (showRemoveConfirm = false)}
          disabled={removing}
          aria-label={$t("tracking.remove_dialog.close")}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M18 6 6 18 M6 6l12 12"/>
          </svg>
        </button>
      </header>
      <div class="dialog-body">
        <p class="confirm-msg">
          {$t("tracking.remove_dialog.question", { name: label })}
        </p>
        <p class="confirm-sub">{$t("tracking.remove_dialog.sub")}</p>
      </div>
      <footer class="dialog-foot">
        <button type="button" class="btn-secondary" onclick={() => (showRemoveConfirm = false)} disabled={removing}>
          {$t("tracking.remove_dialog.cancel")}
        </button>
        <button type="button" class="btn-danger" onclick={confirmRemove} disabled={removing}>
          {removing ? $t("tracking.remove_dialog.confirming") : $t("tracking.remove_dialog.confirm")}
        </button>
      </footer>
    </div>
  </div>
{/if}

{#if toasts.length > 0}
  <div class="toast-stack" aria-live="polite" aria-atomic="false">
    {#each toasts as t (t.id)}
      <button type="button" class="toast toast-{t.kind}" onclick={() => dismissToast(t.id)}>
        {t.text}
      </button>
    {/each}
  </div>
{/if}

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    max-width: 720px;
    margin-inline: auto;
    padding: 16px 20px 80px;
  }

  .back {
    color: var(--tertiary);
    text-decoration: none;
    font-size: 13px;
    width: fit-content;
  }
  .back:hover {
    color: var(--accent);
  }

  /* Skeleton */
  .skeleton {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .sk-row,
  .sk-card {
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--input-border) 50%, transparent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .sk-row {
    height: 14px;
  }
  .sk-lg { width: 70%; height: 22px; }
  .sk-md { width: 50%; }
  .sk-card { height: 56px; }
  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

  /* Empty / not found */
  .empty-card {
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
  .empty-card h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 500;
    color: var(--secondary);
  }
  .empty-card p {
    margin: 0 0 12px;
    font-size: 13px;
    color: var(--tertiary);
  }

  /* Hero */
  .hero {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .carrier-icon-lg {
    flex-shrink: 0;
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: color-mix(in oklab, var(--carrier-color) 16%, transparent);
    color: var(--carrier-color);
  }
  .status-icon-lg {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: color-mix(in oklab, var(--status-color) 18%, transparent);
    color: var(--status-color);
  }
  .hero-text {
    flex: 1;
    min-width: 0;
  }
  .hero h1 {
    margin: 0 0 2px;
    font-size: 20px;
    font-weight: 500;
    letter-spacing: -0.3px;
    color: var(--secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hero-meta {
    margin: 0;
    font-size: 12px;
    color: var(--tertiary);
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .carrier-name {
    font-weight: 500;
  }
  .dot-sep {
    opacity: 0.6;
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
    flex-shrink: 0;
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

  /* Status row */
  .status-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .status-badge-big {
    --status-color: var(--accent);
    padding: 6px 14px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--status-color) 14%, transparent);
    color: var(--status-color);
    font-size: 13px;
    font-weight: 500;
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    max-width: 100%;
  }
  .status-badge-big.delivered {
    /* Quando `canonical_status === "delivered"`, --status-color já vem como
       var(--success) via tracker-status-icons. Esse override só importa
       quando o pacote tem `is_delivered` mas sem canonical (legado). */
    background: color-mix(in oklab, var(--success) 18%, transparent);
    color: var(--success);
  }
  .status-badge-label {
    font-weight: 500;
  }
  .status-badge-raw {
    font-weight: 400;
    opacity: 0.75;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .days-pill {
    padding: 4px 10px;
    border-radius: 999px;
    background: transparent;
    border: 1px solid var(--input-border);
    color: var(--tertiary);
    font-size: 11px;
  }
  .delivered-pill {
    border-color: color-mix(in oklab, var(--success) 50%, var(--input-border));
    color: var(--success);
  }
  .archived-pill {
    border-color: color-mix(in oklab, var(--tertiary) 50%, var(--input-border));
    color: var(--tertiary);
    background: color-mix(in oklab, var(--tertiary) 10%, transparent);
  }

  /* Archive prompt — aparece para pacotes Delivered ainda não arquivados.
     Tom de "atenção positiva" (parabens, terminou + opção de arquivar) via
     `--warning`. Texto principal usa `--secondary` (legibilidade) e o
     título destaca em `--warning`. Em temas eink/light, `--warning` é
     ambar escuro; em temas dark/dracula é amarelo saturado. Ambos
     funcionam bem como tom positivo de notificação. */
  .archive-prompt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 16px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--warning) 16%, transparent);
    border: 1px solid color-mix(in oklab, var(--warning) 35%, var(--input-border));
    color: var(--secondary);
  }
  .archive-prompt-text {
    min-width: 0;
    flex: 1;
  }
  .archive-prompt-text h3 {
    margin: 0 0 2px;
    font-size: 13px;
    font-weight: 500;
    color: var(--warning);
  }
  .archive-prompt-text p {
    margin: 0;
    font-size: 12px;
    color: var(--secondary);
    opacity: 0.85;
    line-height: 1.4;
  }
  .archive-prompt-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  /* Warn banner — refresh failure, tom de atenção (não erro fatal).
     Usa `--warning` por consistência com archive-prompt; o problema é
     transient e o usuário pode tentar de novo. */
  .warn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--warning) 16%, transparent);
    border: 1px solid color-mix(in oklab, var(--warning) 30%, var(--input-border));
    color: var(--secondary);
    font-size: 12px;
  }
  .warn strong {
    color: var(--warning);
  }
  .warn-msg {
    margin-left: 4px;
    opacity: 0.85;
  }
  .warn-btn {
    padding: 4px 10px;
    border-radius: var(--border-radius);
    border: 1px solid currentColor;
    background: transparent;
    color: inherit;
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
    flex-shrink: 0;
  }
  .warn-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Meta */
  .meta {
    margin: 0;
    padding: 12px 16px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .meta-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .meta-row dt {
    flex: 0 0 110px;
    font-size: 11px;
    color: var(--tertiary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .meta-row dd {
    margin: 0;
    flex: 1;
    min-width: 0;
    font-size: 13px;
    color: var(--secondary);
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .icon-btn,
  .text-btn {
    border: none;
    background: transparent;
    cursor: pointer;
    font-family: inherit;
    color: var(--tertiary);
    padding: 4px;
    border-radius: 4px;
  }
  .icon-btn:hover,
  .text-btn:hover {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 10%, transparent);
  }
  .text-btn {
    font-size: 11px;
    padding: 2px 6px;
  }
  .route-arrow {
    color: var(--tertiary);
    opacity: 0.7;
  }

  /* Actions */
  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
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

  /* Timeline */
  .history {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .history h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--secondary);
  }
  .muted {
    margin: 0;
    color: var(--tertiary);
    font-size: 13px;
  }
  .timeline {
    margin: 0;
    padding: 0;
    list-style: none;
    position: relative;
  }
  .timeline::before {
    content: "";
    position: absolute;
    left: 13px;
    top: 12px;
    bottom: 12px;
    width: 1px;
    background: var(--input-border);
  }
  .event {
    position: relative;
    display: flex;
    gap: 12px;
    padding: 6px 0 14px 0;
  }
  .event:last-child {
    padding-bottom: 0;
  }
  .event-icon {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    margin-top: 2px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: color-mix(in oklab, var(--status-color) 16%, var(--surface));
    color: var(--status-color);
    border: 1px solid color-mix(in oklab, var(--status-color) 30%, var(--input-border));
    z-index: 1;
  }
  .event.first .event-icon {
    background: color-mix(in oklab, var(--status-color) 24%, var(--surface));
    border-color: var(--status-color);
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--status-color) 18%, transparent);
  }
  .event.repeat .event-icon {
    width: 22px;
    height: 22px;
    margin-top: 5px;
    margin-left: 3px;
    opacity: 0.7;
  }
  .event-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .event-desc {
    margin: 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--secondary);
  }
  .event-meta {
    margin: 0;
    font-size: 11px;
    color: var(--tertiary);
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .event-status-chip {
    --status-color: var(--accent);
    padding: 1px 7px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--status-color) 12%, transparent);
    color: var(--status-color);
    font-size: 10px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .sub-list {
    margin: 4px 0 0;
    padding: 0 0 0 14px;
    list-style: disc;
    font-size: 11px;
    color: var(--tertiary);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  /* Dialog */
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
  .error {
    padding: 10px 14px;
    border-radius: var(--border-radius);
    background: color-mix(in oklab, var(--danger) 14%, transparent);
    color: var(--danger);
    font-size: 12px;
  }

  /* Toast */
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
  }
  .toast-success {
    border-color: color-mix(in oklab, var(--success) 50%, var(--input-border));
  }
  .toast-error {
    border-color: color-mix(in oklab, var(--danger) 50%, var(--input-border));
    color: var(--danger);
  }

  .mono {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-variant-numeric: tabular-nums;
  }
  .small {
    font-size: 11px;
  }

  @media (max-width: 480px) {
    .hero h1 {
      font-size: 17px;
    }
    .meta-row {
      flex-direction: column;
      align-items: flex-start;
      gap: 2px;
    }
    .meta-row dt {
      flex: 0 0 auto;
    }
    .actions .btn-secondary,
    .actions .btn-danger {
      flex: 1;
      min-width: 120px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sk-row,
    .sk-card {
      animation: none;
    }
  }
</style>
