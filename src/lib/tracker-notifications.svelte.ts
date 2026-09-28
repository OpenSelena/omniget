import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { get } from "svelte/store";

import { onTrackerEvent, type TrackerEventPayload } from "$lib/tracker-bridge";
import { getTrackerSettings } from "$lib/tracker-settings.svelte";
import { privacyEnabled, maskCode } from "$lib/tracker-privacy.svelte";
import { t } from "$lib/i18n";

const NOTIFICATION_DEBOUNCE_MS = 60_000;
const lastNotifiedAt = new Map<number, number>();

let registered = false;
let permissionCache: boolean | null = null;

async function ensurePermissionGranted(): Promise<boolean> {
  if (permissionCache === true) return true;
  try {
    if (await isPermissionGranted()) {
      permissionCache = true;
      return true;
    }
    const status = await requestPermission();
    const granted = status === "granted";
    permissionCache = granted;
    return granted;
  } catch (e) {
    console.warn("[tracker-notifications] permission check failed", e);
    return false;
  }
}

async function windowIsFocused(): Promise<boolean> {
  try {
    return await getCurrentWebviewWindow().isFocused();
  } catch {
    return false;
  }
}

function tStr(key: string, params?: Record<string, unknown>): string {
  const fn = get(t);
  return fn(key, params) as unknown as string;
}

function buildNotificationTitle(code: string, status: string): string {
  const displayCode = privacyEnabled() ? maskCode(code) : code;
  return tStr("tracking.notification.title_template", {
    code: displayCode,
    status,
  });
}

function buildNotificationBody(location: string | null): string {
  if (location && location.trim().length > 0) return location;
  return tStr("tracking.notification.body_fallback");
}

async function handleStatusChanged(p: TrackerEventPayload["status-changed"]) {
  const settings = getTrackerSettings();
  if (!settings.notificationsEnabled) return;
  if (!p.canonical_changed) return;

  // Suppress when foreground — in-app toast on /misc/tracking already covers it.
  if (await windowIsFocused()) return;

  const now = Date.now();
  const last = lastNotifiedAt.get(p.id) ?? 0;
  if (now - last < NOTIFICATION_DEBOUNCE_MS) return;

  if (!(await ensurePermissionGranted())) return;

  try {
    await sendNotification({
      title: buildNotificationTitle(p.code, p.new_status),
      body: buildNotificationBody(p.location),
    });
    lastNotifiedAt.set(p.id, now);
  } catch (e) {
    console.warn("[tracker-notifications] sendNotification failed", e);
  }
}

export async function ensureTrackerNotifications(): Promise<void> {
  if (registered) return;
  registered = true;

  // Best-effort upfront permission probe — don't block startup if it fails.
  void ensurePermissionGranted();

  try {
    await onTrackerEvent("status-changed", (p) => {
      void handleStatusChanged(p);
    });
  } catch (e) {
    console.warn("[tracker-notifications] listener registration failed", e);
    registered = false;
  }
}

export async function requestNotificationPermission(): Promise<boolean> {
  permissionCache = null;
  return await ensurePermissionGranted();
}

export async function isNotificationPermissionGranted(): Promise<boolean> {
  try {
    return await isPermissionGranted();
  } catch {
    return false;
  }
}

export async function sendTestNotification(): Promise<{ ok: boolean; denied: boolean }> {
  if (!(await ensurePermissionGranted())) {
    return { ok: false, denied: true };
  }
  try {
    await sendNotification({
      title: tStr("tracking.notification.test_title"),
      body: tStr("tracking.notification.test_body"),
    });
    return { ok: true, denied: false };
  } catch (e) {
    console.warn("[tracker-notifications] test send failed", e);
    return { ok: false, denied: false };
  }
}
