<script lang="ts">
  import { goto } from "$app/navigation";
  import {
    studyNotificationsList,
    studyNotificationsRead,
    studyNotificationsDismiss,
    type NotificationFull,
  } from "$lib/study-bridge";

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();
  let items = $state<NotificationFull[]>([]);
  let loading = $state(false);
  let filter = $state<"unread" | "all">("unread");

  async function load() {
    loading = true;
    try {
      items = await studyNotificationsList({ unreadOnly: filter === "unread" });
    } catch (e) {
      console.error("notifications list failed", e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (open) {
      void filter;
      void load();
    }
  });

  function fmtAgo(secs: number): string {
    const diff = Math.floor(Date.now() / 1000) - secs;
    if (diff < 60) return "agora";
    if (diff < 3600) return `${Math.floor(diff / 60)}min`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
    if (diff < 30 * 86400) return `${Math.floor(diff / 86400)}d`;
    return new Date(secs * 1000).toLocaleDateString();
  }

  async function openItem(n: NotificationFull) {
    try {
      if (n.read_at == null) {
        await studyNotificationsRead([n.id]);
      }
    } catch {
      void 0;
    }
    onClose();
    if (n.lesson_id != null) {
      goto(`/study/course/${n.course_id}/lesson/${n.lesson_id}`);
    } else {
      goto(`/study/course/${n.course_id}`);
    }
  }

  async function dismiss(id: number, e: MouseEvent) {
    e.stopPropagation();
    e.preventDefault();
    const before = items;
    items = items.filter((n) => n.id !== id);
    try {
      await studyNotificationsDismiss(id);
    } catch (err) {
      console.error("dismiss failed", err);
      items = before;
    }
  }

  function onBackdropKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" aria-label="Notificações" tabindex="-1" onkeydown={onBackdropKey}>
    <button type="button" class="bg-btn" aria-label="Fechar" onclick={onClose}></button>
    <aside class="drawer" role="document">
      <header class="head">
        <h2>Notificações</h2>
        <button type="button" class="close" aria-label="Fechar" onclick={onClose}>×</button>
      </header>
      <div class="filter-row" role="tablist" aria-label="Filtro">
        <button
          type="button"
          class="ftab"
          class:active={filter === "unread"}
          role="tab"
          aria-selected={filter === "unread"}
          onclick={() => (filter = "unread")}
        >Não lidas</button>
        <button
          type="button"
          class="ftab"
          class:active={filter === "all"}
          role="tab"
          aria-selected={filter === "all"}
          onclick={() => (filter = "all")}
        >Todas</button>
      </div>
      <div class="body">
        {#if loading}
          <p class="muted">Carregando…</p>
        {:else if items.length === 0}
          <div class="empty">
            <svg viewBox="0 0 24 24" width="40" height="40" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
              <path d="M10 21a2 2 0 0 0 4 0" />
            </svg>
            <p>{filter === "unread" ? "Tudo lido por aqui!" : "Nenhuma notificação"}</p>
          </div>
        {:else}
          <ul class="list">
            {#each items as n (n.id)}
              {@const isUnread = n.read_at == null}
              <li class="row" class:unread={isUnread}>
                <button type="button" class="row-btn" onclick={() => openItem(n)}>
                  {#if isUnread}
                    <span class="dot" aria-label="Não lida"></span>
                  {/if}
                  <span class="content">
                    <span class="course">{n.course_title}</span>
                    <span class="lesson">{n.lesson_title ?? "Nova aula"}</span>
                  </span>
                  <span class="ago">{fmtAgo(n.detected_at)}</span>
                </button>
                <button
                  type="button"
                  class="dismiss"
                  aria-label="Dispensar"
                  onclick={(e) => dismiss(n.id, e)}
                >×</button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </aside>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    grid-template-columns: 1fr auto;
  }

  .bg-btn {
    grid-column: 1;
    background: color-mix(in oklab, black 30%, transparent);
    border: none;
    cursor: default;
    animation: fade-in 200ms ease-out;
  }

  .drawer {
    grid-column: 2;
    width: min(420px, 90vw);
    height: 100%;
    background: var(--content-bg);
    border-left: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
    display: flex;
    flex-direction: column;
    box-shadow: -8px 0 24px color-mix(in oklab, black 25%, transparent);
    animation: slide-in-right 220ms ease-out;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 40%, transparent);
  }

  .head h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  .close {
    background: transparent;
    border: none;
    color: inherit;
    font-size: 24px;
    line-height: 1;
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
  }

  .close:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }

  .filter-row {
    display: flex;
    gap: 4px;
    padding: 12px 20px;
    border-bottom: 1px solid color-mix(in oklab, var(--content-border) 30%, transparent);
  }

  .ftab {
    padding: 6px 14px;
    background: transparent;
    border: 1px solid color-mix(in oklab, var(--content-border) 60%, transparent);
    border-radius: 999px;
    color: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }

  .ftab.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast, white);
  }

  .body {
    flex: 1 1 auto;
    overflow-y: auto;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 4px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row {
    display: flex;
    align-items: stretch;
    border-radius: 8px;
  }

  .row:hover {
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }

  .row-btn {
    flex: 1 1 auto;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background: transparent;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    font: inherit;
    border-radius: 8px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    flex: 0 0 auto;
  }

  .content {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .course {
    font-size: 11px;
    color: color-mix(in oklab, currentColor 60%, transparent);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .lesson {
    font-size: 14px;
    line-height: 1.3;
  }

  .row.unread .lesson {
    font-weight: 600;
  }

  .ago {
    font-size: 11px;
    color: color-mix(in oklab, currentColor 60%, transparent);
    flex: 0 0 auto;
    font-variant-numeric: tabular-nums;
  }

  .dismiss {
    width: 32px;
    background: transparent;
    border: none;
    color: color-mix(in oklab, currentColor 50%, transparent);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    border-radius: 0 8px 8px 0;
  }

  .dismiss:hover {
    color: var(--error, #dc2626);
    background: color-mix(in oklab, var(--error, #dc2626) 8%, transparent);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 64px 20px;
    color: color-mix(in oklab, currentColor 50%, transparent);
    text-align: center;
  }

  .empty p {
    margin: 0;
    font-size: 13px;
  }

  .muted {
    color: color-mix(in oklab, currentColor 60%, transparent);
    padding: 16px 20px;
    margin: 0;
    font-size: 13px;
  }

  @keyframes slide-in-right {
    from {
      transform: translateX(100%);
    }
    to {
      transform: translateX(0);
    }
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .drawer,
    .bg-btn {
      animation: none;
    }
  }
</style>
