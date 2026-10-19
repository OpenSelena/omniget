<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "$lib/engine/workspace-api";
  import { Layers, X, Trash2, Download, Terminal, Plus, Check } from "lucide-svelte";

  let { children }: { children?: Snippet } = $props();

  let showDrawer = $state(false);
  let stackItems = $state<Array<{ id: string; name: string; tag: string }>>([
    { id: "tools/web-browser", name: "Playwright Web Agent", tag: "tools" },
    { id: "tools/git-flow", name: "Git Worktree Manager", tag: "tools" },
  ]);

  function removeFromStack(id: string) {
    stackItems = stackItems.filter((it) => it.id !== id);
  }

  function clearStack() {
    stackItems = [];
  }
</script>

<div class="catalog-root">
  {#if children}
    {@render children()}
  {/if}

  <!-- Stack Drawer FAB Trigger -->
  <button
    type="button"
    class="stack-fab"
    class:empty={stackItems.length === 0}
    aria-label="View bundle stack"
    onclick={() => (showDrawer = true)}
  >
    <Layers size={18} />
    <span>{t("catalog.stack.title", "Bundle Stack")}</span>
    <span class="n">{stackItems.length}</span>
  </button>

  <!-- Drawer Modal -->
  {#if showDrawer}
    <!-- Scrim Backdrop -->
    <div
      class="scrim"
      role="button"
      tabindex="-1"
      aria-label="Close drawer"
      onclick={() => (showDrawer = false)}
      onkeydown={(e) => e.key === "Escape" && (showDrawer = false)}
    ></div>

    <aside class="drawer" role="dialog" aria-modal="true" aria-labelledby="stack-title">
      <div class="head">
        <h2 id="stack-title">
          <Layers size={20} class="text-blue-400" />
          {t("catalog.stack.heading", "Package Stack")}
          <span class="count">{stackItems.length}</span>
        </h2>
        <button
          type="button"
          class="x"
          aria-label="Close"
          onclick={() => (showDrawer = false)}
        >
          <X size={16} />
        </button>
      </div>

      <div class="body">
        <div class="h3row">
          <h3>{t("catalog.stack.selected_packages", "Selected Items")}</h3>
          {#if stackItems.length > 0}
            <button type="button" class="link danger" onclick={clearStack}>
              {t("catalog.stack.clear", "Clear All")}
            </button>
          {/if}
        </div>

        {#if stackItems.length === 0}
          <p class="empty">{t("catalog.stack.empty", "No packages in the stack. Click 'Add to Stack' on catalog cards.")}</p>
        {:else}
          <ul class="items">
            {#each stackItems as item (item.id)}
              <li>
                <span class="nm" title={item.name}>{item.name}</span>
                <span class="tag">{item.tag}</span>
                <div class="acts">
                  <button
                    type="button"
                    title="Remove from stack"
                    onclick={() => removeFromStack(item.id)}
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              </li>
            {/each}
          </ul>
        {/if}

        <div class="cli-section">
          <h3>{t("catalog.stack.cli_install", "Install via Terminal")}</h3>
          <div class="cli">
            <code>omniget pkg install {stackItems.map((s) => s.id).join(" ")}</code>
          </div>
        </div>
      </div>

      <div class="foot">
        <div class="primary-row">
          <button
            type="button"
            class="btn primary"
            disabled={stackItems.length === 0}
            onclick={() => {
              showDrawer = false;
            }}
          >
            <Download size={15} class="mr-1 inline" />
            {t("catalog.stack.apply", "Install All Packages")}
          </button>
        </div>
        <div class="secondary">
          <button type="button" class="btn" onclick={() => (showDrawer = false)}>
            {t("catalog.stack.close", "Dismiss")}
          </button>
        </div>
      </div>
    </aside>
  {/if}
</div>

<style>
  .catalog-root {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
  }

  .stack-fab {
    position: fixed;
    right: 24px;
    bottom: 24px;
    z-index: 30;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 42px;
    padding: 0 16px 0 14px;
    border: none;
    border-radius: 999px;
    background: var(--cta, #1f6feb);
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    cursor: pointer;
    transition: transform 0.15s ease, background 0.15s ease;
  }

  .stack-fab:hover {
    transform: translateY(-2px);
    background: #388bfd;
  }

  .stack-fab.empty {
    background: var(--surface-hi, #21262d);
    color: var(--text-muted, #8b949e);
  }

  .n {
    min-width: 20px;
    padding: 2px 7px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.2);
    font-size: 12px;
    text-align: center;
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(2px);
  }

  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 41;
    width: min(420px, 96vw);
    display: flex;
    flex-direction: column;
    background: var(--popup-bg, #161b22);
    box-shadow: -8px 0 32px rgba(0, 0, 0, 0.5);
    outline: none;
    color: var(--text, #e6edf3);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .count {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--accent, #1f6feb);
    color: #fff;
    font-size: 12px;
  }

  .x {
    border: none;
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    width: 28px;
    height: 28px;
    border-radius: 50%;
    color: var(--text, #fff);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .x:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .h3row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted, #8b949e);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .empty {
    font-size: 13px;
    color: var(--text-muted, #8b949e);
    margin: 8px 0;
  }

  .items {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .items li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--fill-1, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.06));
  }

  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  .tag {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--fill-2, rgba(255, 255, 255, 0.1));
    color: var(--text-muted, #8b949e);
  }

  .acts button {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 4px;
    background: none;
    color: var(--text-muted, #8b949e);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .acts button:hover {
    color: #f85149;
    background: rgba(248, 81, 73, 0.15);
  }

  .link {
    border: none;
    background: none;
    color: var(--accent-hi, #58a6ff);
    font-size: 12px;
    cursor: pointer;
    padding: 0;
  }

  .link.danger {
    color: #f85149;
  }

  .cli code {
    display: block;
    padding: 10px;
    border-radius: 6px;
    background: var(--surface-mut, #0d1117);
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: #3fb950;
    overflow-x: auto;
  }

  .foot {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
  }

  .primary-row, .secondary {
    display: flex;
    gap: 8px;
  }

  .btn {
    flex: 1;
    height: 38px;
    padding: 0 16px;
    border: none;
    border-radius: 8px;
    background: var(--fill-1, rgba(255, 255, 255, 0.08));
    color: var(--text, #fff);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.15);
  }

  .btn.primary {
    background: var(--cta, #1f6feb);
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    background: #388bfd;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
