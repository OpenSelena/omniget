<script lang="ts">
  type Props = {
    nextLessonId: number | null;
    nextLessonTitle: string | null;
    courseId: number;
    durationMs?: number;
    onCancel: () => void;
    onGo: () => void;
  };

  let { nextLessonId, nextLessonTitle, courseId: _courseId, durationMs = 5000, onCancel, onGo }: Props = $props();

  let remaining = $state(Math.ceil(durationMs / 1000));
  let cancelled = $state(false);
  let timer: number | null = null;

  $effect(() => {
    if (nextLessonId == null || cancelled) return;
    remaining = Math.ceil(durationMs / 1000);
    timer = window.setInterval(() => {
      remaining -= 1;
      if (remaining <= 0) {
        if (timer) {
          clearInterval(timer);
          timer = null;
        }
        onGo();
      }
    }, 1000);
    return () => {
      if (timer) {
        clearInterval(timer);
        timer = null;
      }
    };
  });

  function cancel() {
    cancelled = true;
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
    onCancel();
  }

  function goNow() {
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
    onGo();
  }
</script>

{#if nextLessonId != null && !cancelled}
  <aside class="toast" role="status" aria-live="polite">
    <div class="info">
      <span class="eyebrow">Próxima aula em {remaining}s</span>
      <span class="title">{nextLessonTitle ?? "Continuar"}</span>
    </div>
    <div class="actions">
      <button type="button" class="btn ghost" onclick={cancel}>Cancelar</button>
      <button type="button" class="btn primary" onclick={goNow}>Continuar agora</button>
    </div>
  </aside>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 80;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 18px;
    min-width: 320px;
    max-width: 480px;
    background: color-mix(in oklab, black 80%, transparent);
    color: white;
    border: 1px solid color-mix(in oklab, var(--accent) 50%, transparent);
    border-radius: 12px;
    box-shadow: 0 12px 32px color-mix(in oklab, black 40%, transparent);
    backdrop-filter: blur(8px);
    animation: slide-in 220ms ease-out;
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1 1 auto;
    min-width: 0;
  }

  .eyebrow {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: color-mix(in oklab, white 70%, transparent);
    font-weight: 600;
  }

  .title {
    font-size: 14px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    gap: 6px;
    flex: 0 0 auto;
  }

  .btn {
    padding: 7px 14px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid color-mix(in oklab, white 16%, transparent);
    background: transparent;
    color: white;
    transition: background var(--tg-duration-fast, 150ms);
  }

  .btn.ghost:hover {
    background: color-mix(in oklab, white 10%, transparent);
  }

  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
  }

  .btn.primary:hover {
    filter: brightness(1.08);
  }

  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateY(16px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
    }
  }
</style>
