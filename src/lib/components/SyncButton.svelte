<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    petsDiff,
    petsInstallMissing,
    attachSyncListeners,
    type DiffReport,
    type SyncProgressEvent,
    type InstallReport,
  } from "$lib/pets/sync";

  type State = "idle" | "checking" | "uptodate" | "confirming" | "downloading" | "done" | "error";

  let {
    onChange,
  }: {
    onChange?: () => void;
  } = $props();

  let phase = $state<State>("idle");
  let diff = $state<DiffReport | null>(null);
  let progress = $state<SyncProgressEvent | null>(null);
  let lastReport = $state<InstallReport | null>(null);
  let errorMsg = $state("");
  let detach: (() => void) | null = null;
  let revertTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    detach = await attachSyncListeners({
      onProgress: (e) => {
        progress = e;
      },
      onFinished: (e) => {
        lastReport = e;
        progress = null;
        if (e.failed.length === 0 && e.added.length === 0) {
          phase = "uptodate";
        } else {
          phase = "done";
        }
        if (onChange) onChange();
        if (revertTimer) clearTimeout(revertTimer);
        revertTimer = setTimeout(() => {
          phase = "idle";
          lastReport = null;
        }, 5000);
      },
      onError: (e) => {
        errorMsg = e.message;
      },
    });
  });

  onDestroy(() => {
    if (detach) detach();
    if (revertTimer) clearTimeout(revertTimer);
  });

  async function check() {
    if (phase !== "idle" && phase !== "uptodate" && phase !== "error" && phase !== "done") return;
    phase = "checking";
    errorMsg = "";
    try {
      const d = await petsDiff();
      diff = d;
      if (d.new_remote.length === 0) {
        phase = "uptodate";
        if (revertTimer) clearTimeout(revertTimer);
        revertTimer = setTimeout(() => (phase = "idle"), 3000);
      } else {
        phase = "confirming";
      }
    } catch (e: any) {
      phase = "error";
      errorMsg = typeof e === "string" ? e : (e?.message ?? "Falha ao verificar");
    }
  }

  async function confirmInstall() {
    if (!diff) return;
    phase = "downloading";
    progress = null;
    try {
      await petsInstallMissing(diff.new_remote);
    } catch (e: any) {
      phase = "error";
      errorMsg = typeof e === "string" ? e : (e?.message ?? "Falha ao baixar");
    }
  }

  function cancelConfirm() {
    phase = "idle";
    diff = null;
  }

  const label = $derived.by(() => {
    if (phase === "checking") return "Verificando…";
    if (phase === "confirming") return "Confirmar atualização";
    if (phase === "downloading") {
      if (progress) return `Baixando ${progress.current} de ${progress.total}…`;
      return "Baixando…";
    }
    if (phase === "uptodate") return "Tudo em dia";
    if (phase === "done" && lastReport) {
      return `${lastReport.added.length} novos adicionados`;
    }
    if (phase === "error") return "Tentar novamente";
    return "Atualizar pets";
  });

  const progressPct = $derived(
    progress && progress.total > 0
      ? Math.min(100, (progress.current / progress.total) * 100)
      : 0,
  );
</script>

<div class="sync">
  <button
    type="button"
    class="sync-btn"
    class:phase-idle={phase === "idle"}
    class:phase-checking={phase === "checking"}
    class:phase-uptodate={phase === "uptodate"}
    class:phase-error={phase === "error"}
    class:phase-done={phase === "done"}
    aria-busy={phase === "downloading" || phase === "checking"}
    disabled={phase === "checking" || phase === "downloading"}
    onclick={check}
  >
    {#if phase === "checking" || phase === "downloading"}
      <span class="spinner" aria-hidden="true"></span>
    {:else if phase === "uptodate" || phase === "done"}
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M20 6L9 17l-5-5" />
      </svg>
    {:else}
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
        <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
        <path d="M21 4v4h-4" />
        <path d="M3 20v-4h4" />
      </svg>
    {/if}
    <span>{label}</span>
  </button>

  {#if phase === "downloading"}
    <div
      class="progress"
      role="progressbar"
      aria-valuenow={Math.round(progressPct)}
      aria-valuemin="0"
      aria-valuemax="100"
    >
      <div class="progress-fill" style:width="{progressPct}%"></div>
    </div>
  {/if}

  {#if phase === "confirming" && diff}
    <div class="confirm">
      <p>
        {diff.new_remote.length} novos pets disponíveis: {diff.new_remote.slice(0, 3).join(", ")}{#if diff.new_remote.length > 3} e mais {diff.new_remote.length - 3}{/if}.
      </p>
      <div class="confirm-actions">
        <button type="button" class="confirm-btn primary" onclick={confirmInstall}>Baixar agora</button>
        <button type="button" class="confirm-btn ghost" onclick={cancelConfirm}>Cancelar</button>
      </div>
    </div>
  {/if}

  {#if phase === "error" && errorMsg}
    <p class="err" aria-live="polite">{errorMsg}</p>
  {/if}

  {#if phase === "done" && lastReport && lastReport.failed.length > 0}
    <p class="warn" aria-live="polite">
      {lastReport.added.length} adicionados, {lastReport.failed.length} falharam.
    </p>
  {/if}
</div>

<style>
  .sync {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .sync-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 8px 14px;
    background: var(--accent);
    color: var(--on-accent);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    align-self: flex-start;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .sync-btn:hover:not(:disabled) {
    background: var(--accent-lo);
  }

  .sync-btn:disabled {
    cursor: progress;
  }

  .sync-btn.phase-uptodate,
  .sync-btn.phase-done {
    background: var(--success);
  }

  .sync-btn.phase-error {
    background: var(--error);
  }

  .spinner {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid currentColor;
    border-right-color: transparent;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .progress {
    width: 280px;
    height: 4px;
    background: var(--surface-hi);
    border-radius: var(--radius-full);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width var(--duration-fast) var(--ease-out);
  }

  .confirm {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    max-width: 480px;
  }

  .confirm p {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .confirm-actions {
    display: flex;
    gap: var(--space-2);
  }

  .confirm-btn {
    padding: 6px 12px;
    background: var(--surface-hi);
    border: 1px solid var(--border);
    border-radius: var(--radius-xs);
    color: var(--text);
    font-family: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .confirm-btn.primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: transparent;
  }

  .confirm-btn.primary:hover {
    background: var(--accent-lo);
  }

  .confirm-btn.ghost {
    background: transparent;
  }

  .err {
    margin: 0;
    color: var(--error);
    font-size: var(--text-sm);
  }

  .warn {
    margin: 0;
    color: var(--warning);
    font-size: var(--text-sm);
  }
</style>
