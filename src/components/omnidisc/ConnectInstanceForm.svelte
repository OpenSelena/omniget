<script lang="ts">
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { translateBackendError } from "$lib/error-translate";
  import {
    connectInstance,
    authenticate,
    backToAuth,
    getConnectStep,
    getConnectError,
    getConnectedInstance,
    getPendingConnect,
    resetConnect,
    addDemoInstance,
    getGuilds,
  } from "$lib/stores/omnidisc-store.svelte";
  import type { AuthMode } from "$lib/omnidisc/types";

  let { compact = false, onDone }: { compact?: boolean; onDone?: () => void } = $props();

  let url = $state("");
  let invite = $state("");
  let urlError = $state<string | null>(null);

  let mode = $state<AuthMode>("register");
  let username = $state("");
  let password = $state("");
  let displayName = $state("");
  let authError = $state<string | null>(null);

  const DEV = import.meta.env.DEV;

  let step = $derived(getConnectStep());
  let error = $derived(getConnectError());
  let connected = $derived(getConnectedInstance());
  let pending = $derived(getPendingConnect());
  let busy = $derived(step === "connecting" || step === "authenticating" || step === "syncing");

  const STEPS = ["connecting", "authenticating", "syncing"] as const;

  function stepIndex(s: string): number {
    return STEPS.indexOf(s as (typeof STEPS)[number]);
  }

  $effect(() => {
    if (pending && !pending.registrationOpen && !pending.invite) mode = "login";
  });

  async function submit(e: Event) {
    e.preventDefault();
    const trimmed = url.trim();
    if (!trimmed) {
      urlError = $t("omnidisc.connect.url_required");
      return;
    }
    urlError = null;
    await connectInstance(trimmed, invite);
  }

  async function submitAuth(e: Event) {
    e.preventDefault();
    if (!username.trim() || !password) {
      authError = $t("omnidisc.connect.auth_required");
      return;
    }
    authError = null;
    await authenticate(mode, username.trim(), password, mode === "register" ? displayName : undefined);
    password = "";
  }

  function openConnected() {
    const instance = connected;
    resetConnect();
    onDone?.();
    if (!instance) return;
    const guild = getGuilds(instance.id)[0];
    const channel = guild?.channels.find((c) => c.kind === "text");
    if (guild && channel) {
      goto(`/omnidisc/g/${guild.id}/${channel.id}`);
    } else {
      goto("/omnidisc");
    }
  }

  function retry() {
    if (pending) backToAuth();
    else resetConnect();
  }

  function changeServer() {
    resetConnect();
    username = "";
    password = "";
    displayName = "";
    authError = null;
  }

  function useDemo() {
    addDemoInstance();
  }

  let registrationBlocked = $derived(pending ? !pending.registrationOpen && !pending.invite : false);
</script>

<section class="connect" class:compact>
  {#if step === "done" && connected}
    <div class="state success" role="status">
      <div class="success-icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5" /></svg>
      </div>
      <h3>{$t("omnidisc.connect.success_title", { name: connected.name })}</h3>
      <p>{$t("omnidisc.connect.success_body")}</p>
      <button type="button" class="primary" onclick={openConnected}>{$t("omnidisc.connect.open")}</button>
    </div>
  {:else if step === "error"}
    <div class="state error" role="alert">
      <h3>{$t("omnidisc.connect.error_title")}</h3>
      <p>{translateBackendError(error ?? "", $t)}</p>
      <div class="actions">
        <button type="button" class="primary" onclick={retry}>{$t("omnidisc.connect.retry")}</button>
        {#if pending}
          <button type="button" class="ghost" onclick={changeServer}>{$t("omnidisc.connect.change_server")}</button>
        {/if}
      </div>
    </div>
  {:else if step === "auth" && pending}
    <form class="form" onsubmit={submitAuth}>
      <div class="auth-head">
        <h3>{$t("omnidisc.connect.auth_title", { name: pending.name })}</h3>
        <p>{$t("omnidisc.connect.auth_body")}</p>
        {#if pending.invite}
          <p class="note">{$t("omnidisc.connect.invite_noted")}</p>
        {/if}
        {#if pending.insecure}
          <p class="warning" role="status">
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M10.3 3.6 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.6a2 2 0 0 0-3.4 0z" />
              <path d="M12 9v4M12 17h.01" />
            </svg>
            <span>{$t("omnidisc.connect.insecure")}</span>
          </p>
        {/if}
      </div>

      <div class="segmented" role="tablist" aria-label={$t("omnidisc.connect.auth_title", { name: pending.name })}>
        <button
          type="button"
          role="tab"
          class:on={mode === "register"}
          aria-selected={mode === "register"}
          disabled={registrationBlocked}
          onclick={() => { mode = "register"; authError = null; }}
        >{$t("omnidisc.connect.mode_register")}</button>
        <button
          type="button"
          role="tab"
          class:on={mode === "login"}
          aria-selected={mode === "login"}
          onclick={() => { mode = "login"; authError = null; }}
        >{$t("omnidisc.connect.mode_login")}</button>
      </div>

      {#if registrationBlocked}
        <p class="note">{$t("omnidisc.connect.registration_closed")}</p>
      {/if}

      <label class="field">
        <span class="label">{$t("omnidisc.connect.username_label")}</span>
        <input
          class="input"
          type="text"
          autocomplete="username"
          autocapitalize="off"
          spellcheck="false"
          bind:value={username}
          placeholder={$t("omnidisc.connect.username_placeholder")}
          oninput={() => { if (authError) authError = null; }}
        />
        {#if mode === "register"}
          <span class="hint">{$t("omnidisc.connect.username_hint")}</span>
        {/if}
      </label>

      <label class="field">
        <span class="label">{$t("omnidisc.connect.password_label")}</span>
        <input
          class="input"
          type="password"
          autocomplete={mode === "register" ? "new-password" : "current-password"}
          bind:value={password}
          oninput={() => { if (authError) authError = null; }}
        />
        {#if mode === "register"}
          <span class="hint">{$t("omnidisc.connect.password_hint")}</span>
        {/if}
      </label>

      {#if mode === "register"}
        <label class="field">
          <span class="label">{$t("omnidisc.connect.display_name_label")}</span>
          <input
            class="input"
            type="text"
            autocomplete="nickname"
            bind:value={displayName}
            placeholder={$t("omnidisc.connect.display_name_placeholder")}
          />
        </label>
      {/if}

      {#if authError || error}
        <p class="field-error" role="alert">{authError ?? translateBackendError(error ?? "", $t)}</p>
      {/if}

      <div class="actions">
        <button type="submit" class="primary">
          {mode === "register" ? $t("omnidisc.connect.register_submit") : $t("omnidisc.connect.login_submit")}
        </button>
        <button type="button" class="ghost" onclick={changeServer}>{$t("omnidisc.connect.change_server")}</button>
      </div>
    </form>
  {:else}
    <form class="form" onsubmit={submit} aria-busy={busy}>
      <label class="field">
        <span class="label">{$t("omnidisc.connect.url_label")}</span>
        <input
          class="input"
          type="text"
          inputmode="url"
          autocomplete="url"
          spellcheck="false"
          bind:value={url}
          placeholder={$t("omnidisc.connect.url_placeholder")}
          disabled={busy}
          aria-invalid={urlError ? "true" : undefined}
          aria-describedby={urlError ? "omnidisc-url-error" : undefined}
          oninput={() => { if (urlError) urlError = null; }}
        />
        {#if urlError}
          <span id="omnidisc-url-error" class="field-error">{urlError}</span>
        {/if}
      </label>

      <label class="field">
        <span class="label">{$t("omnidisc.connect.invite_label")}</span>
        <input
          class="input"
          type="text"
          autocomplete="off"
          spellcheck="false"
          bind:value={invite}
          placeholder={$t("omnidisc.connect.invite_placeholder")}
          disabled={busy}
        />
      </label>

      {#if busy}
        <ol class="stepper" aria-label={$t("omnidisc.connect.submitting")}>
          {#each STEPS as s, i (s)}
            <li class="step" class:done={stepIndex(step) > i} class:current={step === s} aria-current={step === s ? "step" : undefined}>
              <span class="step-dot" aria-hidden="true"></span>
              <span>{$t(`omnidisc.connect.step_${s}`)}</span>
            </li>
          {/each}
        </ol>
      {/if}

      <div class="actions">
        <button type="submit" class="primary" disabled={busy}>
          {busy ? $t("omnidisc.connect.submitting") : $t("omnidisc.connect.submit")}
        </button>
        {#if DEV && !busy}
          <button type="button" class="ghost" onclick={useDemo}>{$t("omnidisc.connect.demo")}</button>
        {/if}
      </div>
    </form>
  {/if}
</section>

<style>
  .connect {
    width: 100%;
    max-width: 420px;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .auth-head {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .auth-head h3 {
    margin: 0;
    font-size: var(--text-md);
    color: var(--text);
  }

  .auth-head p,
  .note {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .warning {
    display: flex;
    gap: var(--space-2);
    align-items: flex-start;
    margin: var(--space-2) 0 0;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--warning);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    line-height: 1.5;
    color: var(--text);
    text-align: left;
  }

  .warning svg {
    flex: 0 0 auto;
    margin-top: 2px;
    color: var(--warning);
  }

  .segmented {
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 3px;
    border-radius: var(--radius-md);
    background: var(--fill-1);
  }

  .segmented button {
    padding: 8px var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
  }

  .segmented button.on {
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
  }

  .segmented button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .segmented button:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .label {
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--text);
  }

  .hint {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .input {
    box-sizing: border-box;
    width: 100%;
    padding: 10px var(--space-3);
    border-radius: var(--radius-md);
    border: none;
    background: var(--input-bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-base);
  }

  .input:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .input[aria-invalid="true"] {
    border-color: var(--danger);
  }

  .field-error {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--danger);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .primary {
    padding: 10px var(--space-5);
    border: none;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font: inherit;
    font-size: var(--text-base);
    font-weight: 600;
    cursor: pointer;
  }

  .primary:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .primary:focus-visible,
  .ghost:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-ring-offset);
  }

  .ghost {
    padding: 10px var(--space-3);
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .ghost:hover {
    color: var(--text);
  }

  .stepper {
    list-style: none;
    margin: 0;
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    border-radius: var(--radius-md);
    background: var(--surface);
    border: none;
  }

  .step {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .step.current {
    color: var(--text);
    font-weight: 600;
  }

  .step.done {
    color: var(--success);
  }

  .step-dot {
    width: 10px;
    height: 10px;
    border-radius: var(--radius-full);
    background: var(--border-hi);
  }

  .step.current .step-dot {
    background: var(--accent);
    animation: blink 1.2s ease-in-out infinite;
  }

  .step.done .step-dot {
    background: var(--success);
  }

  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.35; }
  }

  .state {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
    border-radius: var(--radius-md);
    border: none;
    background: var(--surface);
  }

  .state h3 {
    margin: 0;
    font-size: var(--text-md);
    color: var(--text);
  }

  .state p {
    margin: 0 0 var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .state.error {
    border-color: color-mix(in srgb, var(--danger) 40%, var(--border));
  }

  .state.error h3 {
    color: var(--danger);
  }

  .success-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-full);
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--success) 18%, transparent);
    color: var(--success);
  }

  @media (prefers-reduced-motion: reduce) {
    .step.current .step-dot {
      animation: none;
    }
  }
</style>
