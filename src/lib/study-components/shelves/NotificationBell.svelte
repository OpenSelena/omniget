<script lang="ts">
  import { studyNotificationsCount } from "$lib/study-bridge";
  import NotificationsDrawer from "./NotificationsDrawer.svelte";

  let count = $state({ unread: 0, total: 0 });
  let drawerOpen = $state(false);
  let pulse = $state(false);
  let prevUnread = $state(0);
  const POLL_MS = 60_000;

  async function refresh() {
    try {
      const next = await studyNotificationsCount();
      count = next;
    } catch {
      void 0;
    }
  }

  $effect(() => {
    void refresh();
    const id = window.setInterval(refresh, POLL_MS);
    const onFocus = () => refresh();
    window.addEventListener("focus", onFocus);
    return () => {
      clearInterval(id);
      window.removeEventListener("focus", onFocus);
    };
  });

  $effect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        if (cancelled) return;
        unlisten = await listen("study:library:state-changed", () => {
          void refresh();
        });
        if (cancelled) {
          unlisten?.();
          unlisten = null;
        }
      } catch {
        void 0;
      }
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  $effect(() => {
    const next = count.unread;
    if (next > prevUnread) {
      pulse = true;
      const t = setTimeout(() => (pulse = false), 700);
      prevUnread = next;
      return () => clearTimeout(t);
    }
    prevUnread = next;
  });
</script>

<button
  type="button"
  class="bell"
  class:has-unread={count.unread > 0}
  class:pulse
  aria-label="Notificações ({count.unread} não lidas)"
  onclick={() => (drawerOpen = true)}
>
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
    <path d="M10 21a2 2 0 0 0 4 0" />
  </svg>
  {#if count.unread > 0}
    <span class="badge">{count.unread > 99 ? "99+" : count.unread}</span>
  {/if}
</button>

<NotificationsDrawer
  open={drawerOpen}
  onClose={() => {
    drawerOpen = false;
    void refresh();
  }}
/>

<style>
  .bell {
    position: relative;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: color-mix(in oklab, var(--content-bg) 70%, var(--accent) 4%);
    border: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
    color: color-mix(in oklab, currentColor 70%, transparent);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background var(--tg-duration-fast, 150ms), color var(--tg-duration-fast, 150ms);
  }

  .bell:hover {
    background: color-mix(in oklab, var(--accent) 10%, var(--content-bg) 90%);
    color: inherit;
  }

  .bell.has-unread {
    color: var(--accent);
  }

  .bell.pulse {
    animation: pulse-once 700ms ease-out;
  }

  .badge {
    position: absolute;
    top: -2px;
    right: -2px;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    background: var(--accent);
    color: var(--accent-contrast, white);
    font-size: 10px;
    font-weight: 600;
    line-height: 18px;
    text-align: center;
    border-radius: 9px;
    box-shadow: 0 2px 6px color-mix(in oklab, var(--accent) 40%, transparent);
  }

  @keyframes pulse-once {
    0% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.1);
    }
    100% {
      transform: scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .bell.pulse {
      animation: none;
    }
  }
</style>
