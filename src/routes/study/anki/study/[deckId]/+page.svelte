<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { pluginInvoke } from "$lib/plugin-invoke";
  import ReviewSession from "../../_ReviewSession.svelte";

  type Deck = {
    id: number;
    name: string;
    kind: { kind: string };
  };

  const deckId = $derived(Number($page.params.deckId));

  let deckName = $state<string | null>(null);
  let resolved = $state(false);

  onMount(async () => {
    try {
      const d = await pluginInvoke<Deck>("study", "study:anki:decks:get", {
        id: deckId,
      });
      deckName = d.name;
    } catch (e) {
      console.error("deck lookup failed", e);
    } finally {
      resolved = true;
    }
  });
</script>

{#if resolved}
  <ReviewSession {deckName} {deckId} />
{/if}
