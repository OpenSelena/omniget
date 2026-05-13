<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open as openShell } from "@tauri-apps/plugin-shell";
  import { save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { t } from "$lib/i18n";
  import { showToast } from "$lib/stores/toast-store.svelte";
  import CookieBucketCard from "$lib/components/cookies/CookieBucketCard.svelte";
  import CookieImportMenu from "$lib/components/cookies/CookieImportMenu.svelte";
  import CookieContentModal from "$lib/components/cookies/CookieContentModal.svelte";
  import CookiePasteModal from "$lib/components/cookies/CookiePasteModal.svelte";

  type Account = {
    slug: string;
    alias: string;
    source_url: string | null;
    source_label: string | null;
    captured_at_ms: number;
    cookie_count: number;
    last_used_at_ms: number | null;
  };
  type BucketEntry = { platform_kind: string; accounts: Account[] };
  type Registry = { buckets: Record<string, BucketEntry> };
  type ListResponse = { registry: Registry; cookies_dir: string };
  type ImportResponse = {
    buckets_written: Array<{ domain: string; cookie_count: number; platform_kind: string }>;
  };

  let registry = $state<Registry>({ buckets: {} });
  let cookiesDir = $state("");
  let loading = $state(true);
  let pasteOpen = $state(false);
  let genericDomain = $state("");
  let genericFormOpen = $state(false);

  let viewOpen = $state(false);
  let viewDomain = $state("");
  let viewSlug = $state("");
  let viewContent = $state("");
  let viewPath = $state("");

  let confirmClearOpen = $state(false);
  let confirmDomain = $state("");
  let confirmSlug = $state("");

  let addAccountOpen = $state(false);
  let addAccountDomain = $state("");

  let bucketList = $derived(
    Object.entries(registry.buckets)
      .map(([domain, b]) => ({ domain, ...b }))
      .sort((a, b) => {
        const aTime = a.accounts[0]?.captured_at_ms ?? 0;
        const bTime = b.accounts[0]?.captured_at_ms ?? 0;
        return bTime - aTime;
      }),
  );

  async function reload() {
    loading = true;
    try {
      const res = await invoke<ListResponse>("cookies_list");
      registry = res.registry;
      cookiesDir = res.cookies_dir;
    } catch (e) {
      showToast("error", String(e));
    } finally {
      loading = false;
    }
  }

  async function importContent(content: string, sourceUrl: string, alias: string, label: string) {
    try {
      const res = await invoke<ImportResponse>("cookies_import", {
        request: {
          content,
          source_url: sourceUrl || null,
          source_label: label,
          alias: alias || null,
        },
      });
      const total = res.buckets_written.reduce((s, b) => s + b.cookie_count, 0);
      showToast(
        "success",
        ($t("settings.cookies.import_success", {
          buckets: String(res.buckets_written.length),
          cookies: String(total),
        }) as string) ?? `Imported ${total} cookies`,
      );
      pasteOpen = false;
      await reload();
    } catch (e) {
      showToast("error", String(e));
    }
  }

  async function handleImportFile(path: string) {
    try {
      const res = await invoke<ImportResponse>("cookies_import_file", {
        request: { path, alias: null },
      });
      const total = res.buckets_written.reduce((s, b) => s + b.cookie_count, 0);
      showToast(
        "success",
        ($t("settings.cookies.import_success", {
          buckets: String(res.buckets_written.length),
          cookies: String(total),
        }) as string) ?? `Imported ${total} cookies`,
      );
      await reload();
    } catch (e) {
      showToast("error", String(e));
    }
  }

  async function handleImportPasteContent(content: string, sourceUrl: string, alias: string) {
    await importContent(content, sourceUrl, alias, "Paste import");
  }

  async function handleView(domain: string, slug: string) {
    try {
      const res = await invoke<{ content: string; path: string }>("cookies_read", {
        request: { domain, slug },
      });
      viewContent = res.content;
      viewPath = res.path;
      viewDomain = domain;
      viewSlug = slug;
      viewOpen = true;
    } catch (e) {
      showToast("error", String(e));
    }
  }

  async function handleExport(domain: string, slug: string) {
    try {
      const saved = await saveDialog({
        title: $t("settings.cookies.export_dialog_title") as string,
        defaultPath: `${domain}-cookies.txt`,
        filters: [{ name: "Netscape Cookies", extensions: ["txt"] }],
      });
      if (saved && typeof saved === "string") {
        await invoke("cookies_export_to", {
          request: { domain, slug, destination_path: saved },
        });
        showToast("success", $t("settings.cookies.export_done") as string);
      }
    } catch (e) {
      showToast("error", String(e));
    }
  }

  async function handleRename(domain: string, slug: string, newAlias: string) {
    try {
      await invoke("cookies_rename", { request: { domain, slug, new_alias: newAlias } });
      await reload();
    } catch (e) {
      showToast("error", String(e));
    }
  }

  function askClear(domain: string, slug: string) {
    confirmDomain = domain;
    confirmSlug = slug;
    confirmClearOpen = true;
  }

  function openAddAccount(domain: string) {
    addAccountDomain = domain;
    addAccountOpen = true;
  }

  async function submitAddAccount(content: string, sourceUrl: string, alias: string) {
    try {
      const res = await invoke<{ slug: string; cookie_count: number }>("cookies_add_account", {
        request: {
          domain: addAccountDomain,
          alias: alias || "Conta",
          content,
          source_url: sourceUrl || null,
        },
      });
      showToast(
        "success",
        ($t("settings.cookies.add_account_success", {
          slug: res.slug,
          count: String(res.cookie_count),
        }) as string) ?? `Account added (${res.slug})`,
      );
      addAccountOpen = false;
      await reload();
    } catch (e) {
      showToast("error", String(e));
    }
  }

  async function confirmClear() {
    try {
      await invoke("cookies_clear", { request: { domain: confirmDomain, slug: confirmSlug } });
      confirmClearOpen = false;
      await reload();
      showToast("success", $t("settings.cookies.cleared_toast") as string);
    } catch (e) {
      showToast("error", String(e));
    }
  }

  async function openCookiesDir() {
    if (!cookiesDir) return;
    try {
      await openShell(cookiesDir);
    } catch (e) {
      console.warn("[cookies] open dir failed", e);
    }
  }

  function showExtensionHint() {
    showToast(
      "info",
      $t("settings.cookies.hint_use_extension") as string,
    );
  }

  onMount(() => {
    void reload();
  });
</script>

<section class="section">
  <header class="section-head">
    <h2 class="section-title">{$t("settings.cookies.title")}</h2>
    <p class="section-intro">{$t("settings.cookies.intro")}</p>
    <div class="head-actions">
      <button type="button" class="ghost-btn" onclick={showExtensionHint}>
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <rect x="4" y="4" width="16" height="16" rx="3"/>
          <path d="M8 8h8M8 12h6M8 16h4"/>
        </svg>
        {$t("settings.cookies.capture_via_extension")}
      </button>
      <button type="button" class="ghost-btn subtle" onclick={openCookiesDir} disabled={!cookiesDir}>
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
        </svg>
        {$t("settings.cookies.open_folder")}
      </button>
      <CookieImportMenu
        onImportFile={handleImportFile}
        onImportPaste={() => pasteOpen = true}
      />
    </div>
    {#if cookiesDir}
      <p class="dir-hint" title={cookiesDir}>
        <span>{$t("settings.cookies.dir_label")}</span>
        <code>{cookiesDir}</code>
      </p>
    {/if}
  </header>

  {#if loading}
    <div class="loading">
      {#each Array(3) as _, i (i)}
        <div class="skel"></div>
      {/each}
    </div>
  {:else if bucketList.length === 0}
    <div class="empty-state">
      <svg viewBox="0 0 24 24" width="42" height="42" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="10"/>
        <circle cx="8" cy="10" r="1"/>
        <circle cx="16" cy="10" r="1"/>
        <circle cx="11" cy="14" r="1"/>
      </svg>
      <h3>{$t("settings.cookies.empty_title")}</h3>
      <p>{$t("settings.cookies.empty_body")}</p>
    </div>
  {:else}
    <div class="buckets">
      {#each bucketList as bucket (bucket.domain)}
        <CookieBucketCard
          {bucket}
          onView={handleView}
          onExport={handleExport}
          onRename={handleRename}
          onClear={askClear}
          onAddAccount={openAddAccount}
        />
      {/each}
    </div>
  {/if}

  <div class="add-generic">
    {#if !genericFormOpen}
      <button type="button" class="add-toggle" onclick={() => genericFormOpen = true}>
        + {$t("settings.cookies.add_generic_title")}
      </button>
      <p class="add-hint">{$t("settings.cookies.add_generic_hint")}</p>
    {:else}
      <div class="add-form">
        <input
          type="text"
          placeholder={$t("settings.cookies.add_generic_placeholder") as string}
          bind:value={genericDomain}
        />
        <button
          type="button"
          class="ghost-btn"
          onclick={() => { genericFormOpen = false; pasteOpen = true; }}
          disabled={!genericDomain.trim()}
        >
          {$t("settings.cookies.add_generic_paste")}
        </button>
        <button type="button" class="ghost-btn subtle" onclick={() => { genericFormOpen = false; genericDomain = ""; }}>
          {$t("settings.cookies.add_generic_cancel")}
        </button>
      </div>
    {/if}
  </div>
</section>

<CookieContentModal
  open={viewOpen}
  domain={viewDomain}
  slug={viewSlug}
  content={viewContent}
  path={viewPath}
  onClose={() => viewOpen = false}
/>

<CookiePasteModal
  open={pasteOpen}
  onSubmit={handleImportPasteContent}
  onClose={() => pasteOpen = false}
/>

<CookiePasteModal
  open={addAccountOpen}
  onSubmit={submitAddAccount}
  onClose={() => addAccountOpen = false}
/>

{#if confirmClearOpen}
  <div class="overlay" onclick={(e) => { if (e.target === e.currentTarget) confirmClearOpen = false; }} role="presentation">
    <div class="confirm" role="dialog" aria-label={$t("settings.cookies.confirm_clear_title") as string}>
      <h3>{$t("settings.cookies.confirm_clear_title")}</h3>
      <p>{$t("settings.cookies.confirm_clear_body", { domain: confirmDomain })}</p>
      <div class="confirm-actions">
        <button type="button" class="ghost-btn" onclick={() => confirmClearOpen = false}>
          {$t("settings.cookies.confirm_clear_cancel")}
        </button>
        <button type="button" class="danger-btn" onclick={confirmClear}>
          {$t("settings.cookies.confirm_clear_action")}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .section { display: flex; flex-direction: column; gap: 16px; }
  .section-head { display: flex; flex-direction: column; gap: 8px; }
  .section-title { margin: 0; font-size: 18px; font-weight: 600; color: var(--primary); }
  .section-intro { margin: 0; color: var(--secondary); font-size: 13px; line-height: 1.5; }
  .head-actions { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; margin-top: 6px; }
  .dir-hint {
    margin: 6px 0 0;
    font-size: 11px;
    color: var(--tertiary);
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .dir-hint code {
    font-family: ui-monospace, "Cascadia Code", monospace;
    background: color-mix(in oklab, var(--button) 50%, transparent);
    padding: 2px 8px;
    border-radius: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .ghost-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    background: transparent;
    border: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
    border-radius: 999px;
    color: var(--secondary);
    font-size: 13px;
    cursor: pointer;
    transition: border-color 120ms, color 120ms;
  }
  .ghost-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .ghost-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .ghost-btn.subtle { color: var(--tertiary); }
  .loading { display: flex; flex-direction: column; gap: 12px; }
  .skel {
    height: 92px;
    border-radius: 14px;
    background: linear-gradient(
      90deg,
      color-mix(in oklab, var(--button) 35%, transparent) 0%,
      color-mix(in oklab, var(--button) 60%, transparent) 50%,
      color-mix(in oklab, var(--button) 35%, transparent) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s ease-in-out infinite;
  }
  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
    padding: 40px 20px;
    color: var(--tertiary);
    background: color-mix(in oklab, var(--button) 15%, transparent);
    border: 1px dashed color-mix(in oklab, var(--content-border) 50%, transparent);
    border-radius: 14px;
  }
  .empty-state svg { color: var(--tertiary); opacity: 0.6; }
  .empty-state h3 { margin: 4px 0 0; font-size: 15px; color: var(--secondary); font-weight: 600; }
  .empty-state p { margin: 0; font-size: 13px; max-width: 380px; line-height: 1.5; }
  .buckets { display: flex; flex-direction: column; gap: 10px; }
  .add-generic {
    padding: 14px 18px;
    border-radius: 14px;
    border: 1px dashed color-mix(in oklab, var(--content-border) 40%, transparent);
    margin-top: 4px;
  }
  .add-toggle {
    background: transparent;
    border: 0;
    color: var(--accent);
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    padding: 0;
  }
  .add-toggle:hover { filter: brightness(1.2); }
  .add-hint { margin: 4px 0 0; font-size: 11px; color: var(--tertiary); }
  .add-form { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .add-form input {
    flex: 1;
    min-width: 200px;
    padding: 7px 12px;
    background: color-mix(in oklab, var(--button) 50%, transparent);
    border: 1px solid color-mix(in oklab, var(--content-border) 40%, transparent);
    border-radius: 8px;
    color: var(--primary);
    font: inherit;
    font-size: 13px;
    outline: none;
  }
  .add-form input:focus { border-color: var(--accent); }
  .overlay {
    position: fixed; inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000;
    padding: 24px;
  }
  .confirm {
    width: min(420px, 100%);
    background: var(--background);
    border: 1px solid color-mix(in oklab, var(--content-border) 50%, transparent);
    border-radius: 14px;
    padding: 20px;
    display: flex; flex-direction: column; gap: 12px;
  }
  .confirm h3 { margin: 0; font-size: 16px; color: var(--primary); }
  .confirm p { margin: 0; font-size: 13px; color: var(--secondary); line-height: 1.5; }
  .confirm-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px; }
  .danger-btn {
    padding: 7px 16px;
    background: #d33;
    color: white;
    border: 0;
    border-radius: 999px;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .danger-btn:hover { filter: brightness(1.1); }
</style>
