<script lang="ts">
  import { t } from "$lib/i18n";
  import { EMOJI_GROUPS, rememberEmoji, recentEmojis, searchEmojis } from "$lib/omnidisc/emoji";

  let { onPick, onClose }: { onPick: (emoji: string) => void; onClose: () => void } = $props();

  let query = $state("");
  let recent = $state<string[]>(recentEmojis());
  let input = $state<HTMLInputElement | null>(null);

  $effect(() => {
    input?.focus();
  });

  let matches = $derived(searchEmojis(query));
  let grouped = $derived(
    EMOJI_GROUPS.map((group) => ({ group, items: matches.filter((e) => e.group === group) })).filter(
      (g) => g.items.length > 0,
    ),
  );

  function pick(char: string) {
    recent = rememberEmoji(char);
    onPick(char);
  }
</script>

<div
  class="picker"
  role="dialog"
  tabindex="-1"
  aria-label={$t("omnidisc.emoji.title")}
  onkeydown={(e) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    }
  }}
>
  <input
    bind:this={input}
    bind:value={query}
    class="search"
    type="search"
    placeholder={$t("omnidisc.emoji.search")}
    aria-label={$t("omnidisc.emoji.search")}
    spellcheck="false"
  />
  <div class="scroll">
    {#if matches.length === 0}
      <p class="empty">{$t("omnidisc.emoji.no_match", { query })}</p>
    {:else}
      {#if recent.length > 0 && query.trim().length === 0}
        <h4 class="group-title">{$t("omnidisc.emoji.recent")}</h4>
        <div class="grid">
          {#each recent as char (char)}
            <button type="button" class="emoji" onclick={() => pick(char)} title={char}>{char}</button>
          {/each}
        </div>
      {/if}
      {#each grouped as group (group.group)}
        <h4 class="group-title">{$t(`omnidisc.emoji.group_${group.group}`)}</h4>
        <div class="grid">
          {#each group.items as item (item.char)}
            <button type="button" class="emoji" onclick={() => pick(item.char)} title={item.name}>{item.char}</button>
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .picker {
    width: 268px;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.25));
  }

  .search {
    width: 100%;
    box-sizing: border-box;
    padding: 6px var(--space-2);
    border-radius: var(--radius-sm);
    border: none;
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .search:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .scroll {
    max-height: 240px;
    overflow-y: auto;
  }

  .group-title {
    margin: var(--space-2) 0 var(--space-1);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 2px;
  }

  .emoji {
    aspect-ratio: 1;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-base);
    cursor: pointer;
    color: var(--text);
  }

  .emoji:hover {
    background: var(--fill-1);
  }

  .emoji:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .empty {
    margin: var(--space-3) var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
</style>
