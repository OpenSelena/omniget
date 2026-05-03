<script lang="ts">
  import Mascot from "./Mascot.svelte";
  import type { LocalPetEntry } from "$lib/pets/sync";

  let {
    pet,
    active = false,
    busy = false,
    onActivate,
    onRemove,
    onRefresh,
  }: {
    pet: LocalPetEntry;
    active?: boolean;
    busy?: boolean;
    onActivate: (slug: string) => void;
    onRemove: (slug: string) => void;
    onRefresh: (slug: string) => void;
  } = $props();
</script>

<article class="pet-card" class:active class:busy>
  <div class="pet-stage">
    <Mascot slug={pet.slug} state="idle" scale={0.45} />
    {#if pet.source === "manual"}
      <span class="badge manual">manual</span>
    {/if}
    {#if active}
      <span class="badge active">ativo</span>
    {/if}
  </div>

  <div class="pet-meta">
    <strong class="pet-name">{pet.display_name}</strong>
    <div class="pet-tags">
      {#if pet.kind}
        <span class="tag">{pet.kind}</span>
      {/if}
      {#if pet.vibes.length > 0}
        <span class="tag vibe">{pet.vibes[0]}</span>
      {/if}
    </div>
  </div>

  <div class="pet-actions">
    {#if !active}
      <button type="button" class="action primary" disabled={busy} onclick={() => onActivate(pet.slug)}>
        Definir como ativo
      </button>
    {/if}
    <button type="button" class="action" disabled={busy} onclick={() => onRefresh(pet.slug)}>
      Atualizar
    </button>
    <button type="button" class="action ghost" disabled={busy} onclick={() => onRemove(pet.slug)}>
      Remover
    </button>
  </div>
</article>

<style>
  .pet-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: border-color var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
  }

  .pet-card:hover:not(.busy) {
    border-color: var(--accent);
    box-shadow: var(--elev-1);
  }

  .pet-card.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent) inset;
  }

  .pet-card.busy {
    opacity: 0.6;
  }

  .pet-stage {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-hi);
    border-radius: var(--radius-sm);
    padding: var(--space-3);
    min-height: 110px;
  }

  .badge {
    position: absolute;
    top: 6px;
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 1px 8px;
    border-radius: var(--radius-full);
    background: var(--surface);
    color: var(--text-muted);
    border: 1px solid var(--border);
  }

  .badge.active {
    right: 6px;
    background: var(--accent);
    color: var(--on-accent);
    border-color: transparent;
  }

  .badge.manual {
    left: 6px;
  }

  .pet-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pet-name {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
  }

  .pet-tags {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .tag {
    font-size: 10.5px;
    font-weight: 500;
    padding: 1px 8px;
    border-radius: var(--radius-full);
    background: var(--surface-hi);
    color: var(--text-muted);
  }

  .tag.vibe {
    color: var(--text);
  }

  .pet-actions {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .action {
    padding: 4px 10px;
    font-size: var(--text-xs);
    background: var(--surface-hi);
    border: 1px solid transparent;
    border-radius: var(--radius-xs);
    color: var(--text);
    font-family: inherit;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out);
  }

  .action:hover:not(:disabled) {
    background: var(--surface-mut);
  }

  .action.primary {
    background: var(--accent);
    color: var(--on-accent);
  }

  .action.primary:hover:not(:disabled) {
    background: var(--accent-lo);
  }

  .action.ghost {
    background: transparent;
    border-color: var(--border);
    color: var(--text-muted);
  }

  .action.ghost:hover:not(:disabled) {
    color: var(--error);
    border-color: var(--error);
  }

  .action:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
