<script lang="ts">
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import { getMembers, getUser, searchMessages, userName, type SearchFilters } from "$lib/stores/omnidisc-store.svelte";
  import { snowflakeForTime } from "$lib/omnidisc/snowflake";
  import type { OmnidiscMessage } from "$lib/omnidisc/types";
  import VirtualList from "./VirtualList.svelte";

  let {
    instanceId,
    guildId = null,
    channelId,
    onJump,
    onClose,
  }: {
    instanceId: string;
    guildId?: string | null;
    channelId: string;
    onJump: (channelId: string, messageId: string) => void;
    onClose: () => void;
  } = $props();

  let raw = $state("");
  let scope = $state<"channel" | "guild">("channel");
  let scopeSeeded = false;

  $effect(() => {
    if (scopeSeeded) return;
    scopeSeeded = true;
    if (guildId) scope = "guild";
  });
  let results = $state<OmnidiscMessage[]>([]);
  let total = $state(0);
  let ran = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let unknownFrom = $state<string | null>(null);
  let input = $state<HTMLInputElement | null>(null);

  $effect(() => {
    input?.focus();
  });

  function resolveUser(name: string): string | null {
    const needle = name.trim().toLowerCase();
    if (!needle) return null;
    for (const member of getMembers(guildId)) {
      if (member.name.toLowerCase() === needle) return member.id;
      const user = getUser(instanceId, member.id);
      if (user && (user.username.toLowerCase() === needle || user.displayName.toLowerCase() === needle)) return member.id;
    }
    return null;
  }

  function parseDate(value: string): string | null {
    const parsed = Date.parse(value);
    return Number.isFinite(parsed) ? snowflakeForTime(parsed) : null;
  }

  function parse(): { query: string; filters: SearchFilters } {
    const filters: SearchFilters = {};
    unknownFrom = null;
    const words: string[] = [];
    for (const token of raw.split(/\s+/)) {
      const [key, ...rest] = token.split(":");
      const value = rest.join(":");
      if (!value) {
        if (token) words.push(token);
        continue;
      }
      if (key === "from") {
        const id = resolveUser(value);
        if (id) filters.from = id;
        else unknownFrom = value;
      } else if (key === "has" && (value === "file" || value === "image" || value === "link")) {
        filters.has = value;
      } else if (key === "before") {
        const snow = parseDate(value);
        if (snow) filters.before = snow;
      } else if (key === "after") {
        const snow = parseDate(value);
        if (snow) filters.after = snow;
      } else {
        words.push(token);
      }
    }
    return { query: words.join(" "), filters };
  }

  async function run() {
    const { query, filters } = parse();
    if (!query.trim()) {
      results = [];
      total = 0;
      ran = false;
      return;
    }
    busy = true;
    error = null;
    try {
      const scopeId = scope === "guild" && guildId ? guildId : channelId;
      const outcome = await searchMessages(instanceId, scope === "guild" && guildId ? "guild" : "channel", scopeId, query, filters);
      results = outcome.messages;
      total = outcome.total;
      ran = true;
    } catch (e) {
      error = translateBackendError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), $t);
      results = [];
      total = 0;
      ran = true;
    } finally {
      busy = false;
    }
  }

  function timeLabel(ts: number): string {
    return new Date(ts).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }

  const key = (m: OmnidiscMessage) => m.id;
</script>

<aside class="search-panel" aria-label={$t("omnidisc.search.title")}>
  <header class="head">
    <h3>{$t("omnidisc.search.title")}</h3>
    <button type="button" class="close" onclick={onClose} aria-label={$t("common.close")}>×</button>
  </header>

  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      void run();
    }}
  >
    <input
      bind:this={input}
      bind:value={raw}
      class="input"
      type="search"
      placeholder={$t("omnidisc.search.placeholder")}
      aria-label={$t("omnidisc.search.placeholder")}
      spellcheck="false"
    />
    {#if guildId}
      <div class="scope" role="radiogroup" aria-label={$t("omnidisc.search.scope")}>
        <label class="scope-opt" class:on={scope === "channel"}>
          <input type="radio" name="od-search-scope" value="channel" bind:group={scope} />
          {$t("omnidisc.search.scope_channel")}
        </label>
        <label class="scope-opt" class:on={scope === "guild"}>
          <input type="radio" name="od-search-scope" value="guild" bind:group={scope} />
          {$t("omnidisc.search.scope_guild")}
        </label>
      </div>
    {/if}
    <button type="submit" class="run" disabled={busy || raw.trim().length === 0}>{$t("omnidisc.search.run")}</button>
  </form>

  <p class="hint">{$t("omnidisc.search.filters_hint")}</p>

  {#if unknownFrom}
    <p class="warn" role="status">{$t("omnidisc.search.unknown_from", { name: unknownFrom })}</p>
  {/if}

  {#if error}
    <p class="warn error" role="alert">{error}</p>
  {:else if busy}
    <div class="state" aria-busy="true">
      {#each Array(4) as _, i (i)}
        <span class="skeleton-line"></span>
      {/each}
    </div>
  {:else if !ran}
    <div class="state">
      <p class="state-title">{$t("omnidisc.search.idle_title")}</p>
      <p class="state-body">{$t("omnidisc.search.idle_body")}</p>
    </div>
  {:else if results.length === 0}
    <div class="state">
      <p class="state-title">{$t("omnidisc.search.empty_title")}</p>
      <p class="state-body">{$t("omnidisc.search.empty_body")}</p>
    </div>
  {:else}
    <p class="count">{$t("omnidisc.search.results", { count: total })}</p>
    <div class="results">
      <VirtualList items={results} getKey={key} estimateHeight={72}>
        {#snippet row(item)}
          <button type="button" class="result" onclick={() => onJump(item.channelId, item.id)}>
            <span class="result-head">
              <span class="result-author">{userName(instanceId, item.authorId)}</span>
              <span class="result-time">{timeLabel(item.createdAt)}</span>
            </span>
            <span class="result-text">{item.content.slice(0, 220)}</span>
          </button>
        {/snippet}
      </VirtualList>
    </div>
  {/if}
</aside>

<style>
  .search-panel {
    width: 300px;
    flex: 0 0 300px;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--surface-mut);
    border-left: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .head h3 {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
  }

  .close {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--text-base);
    line-height: 1;
    cursor: pointer;
  }

  .close:hover {
    background: var(--fill-1);
    color: var(--text);
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .input {
    width: 100%;
    box-sizing: border-box;
    padding: 6px var(--space-2);
    border-radius: var(--radius-sm);
    border: 1px solid var(--input-border);
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .input:focus-visible,
  .run:focus-visible,
  .result:focus-visible,
  .close:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .scope {
    display: flex;
    gap: var(--space-1);
  }

  .scope-opt {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    color: var(--text-muted);
    cursor: pointer;
  }

  .scope-opt.on {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-soft);
  }

  .scope-opt input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .run {
    padding: 6px var(--space-3);
    border: none;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--on-accent);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
  }

  .run:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .hint,
  .count {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .warn {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-secondary, var(--text-muted));
  }

  .warn.error {
    color: var(--danger);
  }

  .state {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) 0;
  }

  .state-title {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text);
  }

  .state-body {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .skeleton-line {
    height: 40px;
    border-radius: var(--radius-sm);
    background: var(--fill-2);
    animation: pulse 1.4s ease-in-out infinite;
  }

  .results {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .result {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .result:hover {
    background: var(--fill-1);
  }

  .result-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .result-author {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .result-time {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .result-text {
    font-size: var(--text-sm);
    color: var(--text-secondary, var(--text));
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  @media (prefers-reduced-motion: reduce) {
    .skeleton-line {
      animation: none;
    }
  }
</style>
