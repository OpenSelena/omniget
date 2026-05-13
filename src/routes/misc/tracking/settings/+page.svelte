<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import {
    getTrackerSettings,
    updateTrackerSettings,
  } from "$lib/tracker-settings.svelte";
  import {
    isNotificationPermissionGranted,
    requestNotificationPermission,
    sendTestNotification,
  } from "$lib/tracker-notifications.svelte";

  type Toast = { id: number; kind: "info" | "success" | "error"; text: string };

  let settings = $derived(getTrackerSettings());
  let permissionGranted = $state<boolean | null>(null);
  let testingNotification = $state(false);
  let toasts = $state<Toast[]>([]);
  let toastSeq = 0;

  onMount(() => {
    void refreshPermissionState();
  });

  async function refreshPermissionState() {
    permissionGranted = await isNotificationPermissionGranted();
  }

  function onToggleNotifications(e: Event) {
    const target = e.currentTarget as HTMLInputElement;
    updateTrackerSettings({ notificationsEnabled: target.checked });
  }

  async function onRequestPermission() {
    const granted = await requestNotificationPermission();
    permissionGranted = granted;
    if (!granted) {
      pushToast("error", $t("tracking.settings.permission_denied_toast"));
    }
  }

  async function onTestNotification() {
    if (testingNotification) return;
    testingNotification = true;
    try {
      const result = await sendTestNotification();
      permissionGranted = !result.denied;
      if (result.denied) {
        pushToast("error", $t("tracking.settings.permission_denied_toast"));
      } else if (result.ok) {
        pushToast("info", $t("tracking.settings.test_toast_sent"));
      } else {
        pushToast("error", $t("tracking.settings.test_failed"));
      }
    } finally {
      testingNotification = false;
    }
  }

  function pushToast(kind: Toast["kind"], text: string) {
    const id = ++toastSeq;
    toasts = [...toasts, { id, kind, text }];
    setTimeout(() => {
      toasts = toasts.filter((x) => x.id !== id);
    }, 4500);
  }

  function dismissToast(id: number) {
    toasts = toasts.filter((x) => x.id !== id);
  }
</script>

<section class="settings-page">
  <a class="back" href="/misc/tracking">{$t("tracking.detail.back")}</a>
  <header class="head">
    <h1>{$t("tracking.settings.title")}</h1>
    <p class="subtitle">{$t("tracking.settings.subtitle")}</p>
  </header>

  {#if permissionGranted === false}
    <div class="banner" role="status">
      <div class="banner-text">
        <strong>{$t("tracking.settings.permission_denied_banner")}</strong>
        <p>{$t("tracking.settings.permission_denied_hint")}</p>
      </div>
      <button
        type="button"
        class="btn-secondary"
        onclick={onRequestPermission}
      >
        {$t("tracking.settings.request_permission")}
      </button>
    </div>
  {/if}

  <div class="card">
    <div class="setting-row">
      <div class="setting-text">
        <label for="notif-toggle" class="setting-label">
          {$t("tracking.settings.notifications_label")}
        </label>
        <p class="setting-hint">{$t("tracking.settings.notifications_hint")}</p>
      </div>
      <label class="switch">
        <input
          id="notif-toggle"
          type="checkbox"
          checked={settings.notificationsEnabled}
          onchange={onToggleNotifications}
        />
        <span class="slider" aria-hidden="true"></span>
      </label>
    </div>

    <div class="actions">
      <button
        type="button"
        class="btn-secondary"
        onclick={onTestNotification}
        disabled={testingNotification || !settings.notificationsEnabled}
      >
        {testingNotification
          ? $t("tracking.settings.test_busy")
          : $t("tracking.settings.test_notification")}
      </button>
    </div>
  </div>
</section>

{#if toasts.length > 0}
  <div class="toast-stack" aria-live="polite" aria-atomic="false">
    {#each toasts as toast (toast.id)}
      <button
        type="button"
        class="toast toast-{toast.kind}"
        onclick={() => dismissToast(toast.id)}
      >
        {toast.text}
      </button>
    {/each}
  </div>
{/if}

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    max-width: 720px;
    margin-inline: auto;
    padding: 16px 20px 80px;
  }

  .back {
    align-self: flex-start;
    font-size: 12px;
    color: var(--tertiary);
    text-decoration: none;
  }
  .back:hover {
    color: var(--accent);
  }

  .head h1 {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 500;
    letter-spacing: -0.5px;
    color: var(--secondary);
  }
  .subtitle {
    margin: 0;
    font-size: 13px;
    color: var(--tertiary);
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 18px;
    border-radius: var(--border-radius);
    border: 1px solid color-mix(in oklab, var(--warning) 50%, var(--input-border));
    background: color-mix(in oklab, var(--warning) 12%, transparent);
    color: var(--secondary);
  }
  .banner-text {
    flex: 1;
    min-width: 0;
  }
  .banner-text strong {
    display: block;
    margin-bottom: 4px;
    color: var(--warning);
    font-size: 13px;
  }
  .banner-text p {
    margin: 0;
    font-size: 12px;
    color: var(--tertiary);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 18px 20px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .setting-text {
    flex: 1;
    min-width: 0;
  }
  .setting-label {
    display: block;
    margin-bottom: 4px;
    font-size: 14px;
    font-weight: 500;
    color: var(--secondary);
  }
  .setting-hint {
    margin: 0;
    font-size: 12px;
    color: var(--tertiary);
  }

  .switch {
    position: relative;
    display: inline-block;
    width: 40px;
    height: 22px;
    flex-shrink: 0;
  }
  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }
  .slider {
    position: absolute;
    inset: 0;
    background: var(--input-border);
    border-radius: 999px;
    transition: background 150ms ease;
    cursor: pointer;
  }
  .slider::before {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    background: var(--surface);
    border-radius: 50%;
    transition: transform 150ms ease;
  }
  .switch input:checked + .slider {
    background: var(--accent);
  }
  .switch input:checked + .slider::before {
    transform: translateX(18px);
  }
  .switch input:focus-visible + .slider {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    border-top: 1px solid var(--input-border);
    padding-top: 16px;
  }

  .btn-secondary {
    padding: 7px 14px;
    border-radius: var(--border-radius);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    font-weight: 500;
    border: 1px solid var(--input-border);
    background: transparent;
    color: var(--secondary);
  }
  .btn-secondary:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .toast-stack {
    position: fixed;
    top: 16px;
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 200;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    padding: 10px 14px;
    border: 1px solid var(--input-border);
    border-radius: var(--border-radius);
    background: var(--surface);
    color: var(--secondary);
    font-size: 12px;
    font-family: inherit;
    text-align: left;
    box-shadow: 0 4px 14px color-mix(in oklab, black 15%, transparent);
    max-width: 320px;
    cursor: pointer;
  }
  .toast-success {
    border-color: color-mix(in oklab, var(--success) 50%, var(--input-border));
  }
  .toast-error {
    border-color: color-mix(in oklab, var(--danger) 50%, var(--input-border));
    color: var(--danger);
  }
</style>
