<script lang="ts">
  import { onMount } from "svelte";
  import { studySoundcloudHumanizeError } from "$lib/study-bridge";
  import MascotSlot from "$lib/components/MascotSlot.svelte";
  import { petsGetActive } from "$lib/pets/sync";

  type Props = {
    error: string;
    trackUrl?: string;
    onRetry?: () => void;
  };
  let { error, trackUrl, onRetry }: Props = $props();

  let humanized = $state<string>("Erro tecnico. Toca os detalhes pra ver o que aconteceu");
  let petSlug = $state<string | null>(null);

  onMount(() => {
    let active = true;
    petsGetActive()
      .then((slug) => {
        if (active) petSlug = slug;
      })
      .catch(() => {
        if (active) petSlug = null;
      });
    return () => {
      active = false;
    };
  });

  async function resolve(err: string) {
    try {
      const res = await studySoundcloudHumanizeError({ error: err });
      humanized = res.humanized;
    } catch {}
  }

  async function openInSoundcloud() {
    if (!trackUrl) return;
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(trackUrl);
    } catch {
      window.open(trackUrl, "_blank", "noopener,noreferrer");
    }
  }

  $effect(() => {
    void resolve(error);
    console.warn("[study-music] sc error:", error);
  });
</script>

<div class="sc-error" role="alert">
  <div class="pet" aria-hidden="true">
    <MascotSlot slug={petSlug} event="syncError" scale={0.23} fallback="warn" />
  </div>
  <div class="body">
    <p class="kicker">SoundCloud</p>
    <p class="msg">{humanized}</p>
    <p class="hint">Tentamos outras qualidades antes de mostrar esse aviso.</p>
    <div class="actions">
      {#if onRetry}
        <button type="button" class="btn primary" onclick={onRetry}>Tentar de novo</button>
      {/if}
      {#if trackUrl}
        <button type="button" class="btn ghost" onclick={openInSoundcloud}>
          Abrir no SoundCloud
        </button>
      {/if}
    </div>
    <details class="details">
      <summary>Detalhes tecnicos</summary>
      <pre class="raw">{error}</pre>
    </details>
  </div>
</div>

<style>
  .sc-error {
    --sc-accent: oklch(66% 0.22 38);
    --sc-accent-soft: oklch(66% 0.22 38 / 0.12);
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 14px;
    align-items: start;
    padding: 16px;
    background: color-mix(in oklab, var(--sc-accent-soft) 72%, var(--surface, transparent));
    border: 1px solid color-mix(in oklab, var(--sc-accent) 36%, transparent);
    border-radius: 12px;
    color: var(--secondary);
  }

  .pet {
    width: 48px;
    min-height: 48px;
    display: grid;
    place-items: center;
    color: var(--sc-accent);
    overflow: hidden;
  }

  .pet :global(.slot-fallback) {
    color: var(--sc-accent);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .kicker {
    margin: 0;
    color: var(--sc-accent);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .msg {
    margin: 0;
    color: var(--primary);
    font-size: 14px;
    font-weight: 700;
    line-height: 1.35;
  }

  .hint {
    margin: 0;
    color: var(--tertiary);
    font-size: 12px;
    line-height: 1.45;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .btn {
    min-height: 30px;
    padding: 0 12px;
    border-radius: 999px;
    border: 0;
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }

  .btn.primary {
    background: var(--sc-accent);
    color: oklch(99% 0.01 75);
  }

  .btn.primary:hover {
    background: oklch(72% 0.22 38);
  }

  .btn.ghost {
    background: color-mix(in oklab, var(--button) 55%, transparent);
    color: var(--secondary);
    border: 1px solid color-mix(in oklab, var(--content-border) 58%, transparent);
  }

  .btn.ghost:hover {
    color: var(--primary);
    background: color-mix(in oklab, var(--button) 80%, transparent);
  }

  .details {
    color: var(--tertiary);
    font-size: 12px;
  }

  .details summary {
    width: max-content;
    cursor: pointer;
    font-weight: 600;
  }

  .details summary:hover {
    color: var(--secondary);
  }

  .raw {
    margin: 8px 0 0;
    padding: 9px 10px;
    background: color-mix(in oklab, var(--button) 76%, transparent);
    border-radius: 8px;
    font-size: 11px;
    font-family: var(--mono, ui-monospace, monospace);
    color: var(--tertiary);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow: auto;
  }

  @media (max-width: 560px) {
    .sc-error {
      grid-template-columns: 1fr;
    }

    .pet {
      display: none;
    }
  }
</style>
