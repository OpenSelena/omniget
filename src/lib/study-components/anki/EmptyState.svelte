<script lang="ts">
  import Mascot from "$lib/components/Mascot.svelte";

  let {
    title,
    description = "",
    petSlug = "",
    emoji = "",
    children,
  }: {
    title: string;
    description?: string;
    petSlug?: string;
    emoji?: string;
    children?: any;
  } = $props();
</script>

<section class="empty">
  {#if petSlug}
    <Mascot slug={petSlug} state="jumping" scale={1} />
  {:else if emoji}
    <span class="empty-emoji" aria-hidden="true">{emoji}</span>
  {/if}
  <h3 class="empty-title">{title}</h3>
  {#if description}
    <p class="empty-desc">{description}</p>
  {/if}
  {#if children}
    <div class="empty-actions">{@render children?.()}</div>
  {/if}
</section>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-7) var(--space-5);
    text-align: center;
  }

  .empty-emoji {
    font-size: 56px;
    line-height: 1;
  }

  .empty-title {
    margin: 0;
    font-size: var(--text-md);
    font-weight: 600;
    color: var(--text);
  }

  .empty-desc {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    max-width: 360px;
    line-height: 1.5;
  }

  .empty-actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>
