<script lang="ts">
  import type { SlashCommand, FilteredSlashGroup } from "./slash-commands";

  type Props = {
    groups: FilteredSlashGroup[];
    selectedId: string | null;
    position: { x: number; y: number };
    onPick: (cmd: SlashCommand) => void;
  };

  let { groups, selectedId, position, onPick }: Props = $props();

  const flat = $derived.by(() => {
    const out: SlashCommand[] = [];
    for (const g of groups) for (const c of g.items) out.push(c);
    return out;
  });
</script>

{#if flat.length > 0}
  <div
    class="slash-menu"
    style:left={`${position.x}px`}
    style:top={`${position.y}px`}
    role="listbox"
    aria-label="Comandos slash"
  >
    {#each groups as group (group.group)}
      <div class="group">
        <header class="group-label">{group.label}</header>
        {#each group.items as cmd (cmd.id)}
          <button
            type="button"
            class="row"
            class:active={cmd.id === selectedId}
            role="option"
            aria-selected={cmd.id === selectedId}
            onmousedown={(e) => {
              e.preventDefault();
              onPick(cmd);
            }}
          >
            <span class="indicator">{cmd.indicator}</span>
            <span class="body">
              <span class="label">{cmd.label}</span>
              <span class="hint">{cmd.hint}</span>
            </span>
          </button>
        {/each}
      </div>
    {/each}
  </div>
{/if}

<style>
  .slash-menu {
    position: fixed;
    z-index: 1000;
    min-width: 280px;
    max-width: 360px;
    max-height: 420px;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    box-shadow: 0 12px 32px color-mix(in oklab, black 28%, transparent);
    padding: 4px;
  }
  .group {
    display: flex;
    flex-direction: column;
  }
  .group + .group {
    margin-top: 6px;
    padding-top: 6px;
    border-top: 1px solid color-mix(in oklab, var(--input-border) 60%, transparent);
  }
  .group-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--tertiary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 4px 10px 2px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 10px;
    border: 0;
    border-radius: var(--border-radius);
    background: transparent;
    cursor: pointer;
    text-align: left;
    color: var(--text);
    font: inherit;
  }
  .row:hover {
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .row.active {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
  }
  .indicator {
    flex: 0 0 28px;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    user-select: none;
  }
  .body {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .label {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    font-size: 11px;
    color: var(--tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
