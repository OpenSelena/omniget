<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { t } from "$lib/i18n";
  import {
    telegramQrStart,
    telegramQrPoll,
    telegramSendCode,
    telegramVerifyCode,
    telegramVerify2fa,
    type TelegramQrStart,
  } from "$lib/study-telegram-bridge";

  const dispatch = createEventDispatcher<{ authenticated: { phone: string } }>();

  type Tab = "qr" | "phone";
  type Subview = "main" | "password";

  let tab = $state<Tab>("qr");
  let subview = $state<Subview>("main");

  let qr = $state<TelegramQrStart | null>(null);
  let qrError = $state("");
  let qrPollTimer: ReturnType<typeof setTimeout> | null = null;

  let phone = $state("");
  let code = $state("");
  let codeSent = $state(false);
  let phoneError = $state("");
  let phoneBusy = $state(false);

  let password = $state("");
  let passwordHint = $state("");
  let passwordError = $state("");
  let passwordBusy = $state(false);

  async function startQr() {
    qrError = "";
    qr = null;
    if (qrPollTimer) clearTimeout(qrPollTimer);
    try {
      qr = await telegramQrStart();
      schedulePoll();
    } catch (e) {
      qrError = e instanceof Error ? e.message : String(e);
    }
  }

  function schedulePoll() {
    if (qrPollTimer) clearTimeout(qrPollTimer);
    qrPollTimer = setTimeout(pollQr, 1500);
  }

  async function pollQr() {
    try {
      const status = await telegramQrPoll();
      if (status === "waiting") {
        schedulePoll();
        return;
      }
      if (status.startsWith("success:")) {
        const phoneOk = status.slice("success:".length);
        dispatch("authenticated", { phone: phoneOk });
        return;
      }
      if (status.startsWith("password_required")) {
        passwordHint = status.includes(":") ? status.split(":", 2)[1] : "";
        subview = "password";
        return;
      }
      qrError = status;
    } catch (e) {
      qrError = e instanceof Error ? e.message : String(e);
      schedulePoll();
    }
  }

  async function sendCode() {
    phoneError = "";
    phoneBusy = true;
    try {
      await telegramSendCode(phone.trim());
      codeSent = true;
    } catch (e) {
      phoneError = e instanceof Error ? e.message : String(e);
    } finally {
      phoneBusy = false;
    }
  }

  async function submitCode() {
    phoneError = "";
    phoneBusy = true;
    try {
      const result = await telegramVerifyCode(code.trim());
      if (result.startsWith("password_required")) {
        passwordHint = result.includes(":") ? result.split(":", 2)[1] : "";
        subview = "password";
        return;
      }
      dispatch("authenticated", { phone: result });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.startsWith("password_required")) {
        passwordHint = msg.includes(":") ? msg.split(":", 2)[1] : "";
        subview = "password";
        return;
      }
      phoneError = msg;
    } finally {
      phoneBusy = false;
    }
  }

  async function submitPassword() {
    passwordError = "";
    passwordBusy = true;
    try {
      const result = await telegramVerify2fa(password);
      dispatch("authenticated", { phone: result });
    } catch (e) {
      passwordError = e instanceof Error ? e.message : String(e);
    } finally {
      passwordBusy = false;
    }
  }

  function setTab(next: Tab) {
    tab = next;
    if (next === "qr" && !qr) void startQr();
  }

  onMount(() => {
    void startQr();
  });

  onDestroy(() => {
    if (qrPollTimer) clearTimeout(qrPollTimer);
  });
</script>

<div class="auth-gate">
  <div class="card">
    <header class="card-head">
      <h2>{$t("study.library.telegram.auth.title")}</h2>
      <p class="muted small">{$t("study.library.telegram.auth.subtitle")}</p>
    </header>

    {#if subview === "password"}
      <div class="subview">
        <h3>{$t("study.library.telegram.auth.password_title")}</h3>
        {#if passwordHint}
          <p class="hint muted small">{$t("study.library.telegram.auth.password_hint", { hint: passwordHint })}</p>
        {/if}
        <input
          type="password"
          bind:value={password}
          placeholder="••••••••"
          autocomplete="off"
          disabled={passwordBusy}
        />
        {#if passwordError}<p class="error small">{passwordError}</p>{/if}
        <div class="actions">
          <button type="button" class="ghost" onclick={() => (subview = "main")}>
            {$t("study.common.cancel")}
          </button>
          <button
            type="button"
            class="primary"
            onclick={submitPassword}
            disabled={passwordBusy || !password}
          >
            {passwordBusy ? $t("study.common.loading") : $t("study.common.confirm")}
          </button>
        </div>
      </div>
    {:else}
      <div class="tabs" role="tablist">
        <button
          type="button"
          role="tab"
          aria-selected={tab === "qr"}
          class:active={tab === "qr"}
          onclick={() => setTab("qr")}
        >
          {$t("study.library.telegram.auth.tab_qr")}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === "phone"}
          class:active={tab === "phone"}
          onclick={() => setTab("phone")}
        >
          {$t("study.library.telegram.auth.tab_phone")}
        </button>
      </div>

      {#if tab === "qr"}
        <div class="qr-pane" role="tabpanel">
          {#if qr}
            <div class="qr-svg">
              {@html qr.svg}
            </div>
            <p class="muted small">{$t("study.library.telegram.auth.qr_hint_scan")}</p>
            <p class="muted small mono">
              {$t("study.library.telegram.auth.qr_expires_in", { sec: qr.expires })}
            </p>
          {:else if qrError}
            <p class="error small">{qrError}</p>
            <button type="button" class="ghost" onclick={startQr}>
              {$t("study.common.retry")}
            </button>
          {:else}
            <p class="muted small">{$t("study.common.loading")}</p>
          {/if}
        </div>
      {:else}
        <div class="phone-pane" role="tabpanel">
          <label>
            <span class="muted small">{$t("study.library.telegram.auth.phone_input")}</span>
            <input
              type="tel"
              bind:value={phone}
              placeholder="+5511999999999"
              autocomplete="off"
              disabled={phoneBusy || codeSent}
            />
          </label>
          {#if codeSent}
            <label>
              <span class="muted small">{$t("study.library.telegram.auth.code_input")}</span>
              <input
                type="text"
                bind:value={code}
                inputmode="numeric"
                placeholder="12345"
                autocomplete="off"
                disabled={phoneBusy}
              />
            </label>
          {/if}
          {#if phoneError}<p class="error small">{phoneError}</p>{/if}
          <div class="actions">
            {#if !codeSent}
              <button
                type="button"
                class="primary"
                onclick={sendCode}
                disabled={phoneBusy || !phone.trim()}
              >
                {phoneBusy ? $t("study.common.loading") : $t("study.library.telegram.auth.send_code")}
              </button>
            {:else}
              <button
                type="button"
                class="ghost"
                onclick={() => { codeSent = false; code = ""; }}
              >
                {$t("study.library.telegram.auth.code_resend")}
              </button>
              <button
                type="button"
                class="primary"
                onclick={submitCode}
                disabled={phoneBusy || !code.trim()}
              >
                {phoneBusy ? $t("study.common.loading") : $t("study.common.confirm")}
              </button>
            {/if}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .auth-gate {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: calc(var(--padding) * 4) var(--padding);
    min-height: 60vh;
  }
  .card {
    width: 100%;
    max-width: 420px;
    padding: calc(var(--padding) * 2.5);
    background: var(--button-elevated);
    border: 1px solid var(--content-border);
    border-radius: calc(var(--border-radius) * 1.4);
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) * 1.5);
  }
  .card-head h2 {
    margin: 0 0 0.4rem 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--secondary);
  }
  .card-head p {
    margin: 0;
  }
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--input-border);
  }
  .tabs button {
    padding: 8px 16px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--tertiary);
    font-family: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
  .tabs button.active {
    color: var(--secondary);
    border-bottom-color: var(--accent);
  }
  .qr-pane,
  .phone-pane,
  .subview {
    display: flex;
    flex-direction: column;
    gap: var(--padding);
    align-items: center;
  }
  .qr-pane .qr-svg {
    width: 220px;
    height: 220px;
    padding: 8px;
    background: white;
    border-radius: var(--border-radius);
  }
  .qr-pane :global(.qr-svg svg) {
    width: 100%;
    height: 100%;
  }
  .phone-pane,
  .subview {
    align-items: stretch;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  input {
    width: 100%;
    padding: 8px 12px;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    font-family: inherit;
    font-size: 14px;
  }
  input:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }
  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .primary,
  .ghost {
    padding: 8px 16px;
    border-radius: var(--border-radius);
    font-family: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .ghost {
    background: transparent;
    color: var(--secondary);
    border-color: var(--input-border);
  }
  .ghost:hover {
    background: var(--sidebar-highlight);
  }
  .muted {
    color: var(--tertiary);
  }
  .small {
    font-size: 12px;
  }
  .error {
    color: var(--error);
  }
  .mono {
    font-family: var(--font-mono, "IBM Plex Mono", monospace);
  }
  .hint {
    font-style: italic;
  }
  .subview h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--secondary);
  }
</style>
