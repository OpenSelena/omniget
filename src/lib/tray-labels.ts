import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { t, locale } from "$lib/i18n";

type Translate = (key: string, values?: Record<string, unknown>) => string;

export type TrayLabels = {
  quit: string;
  channels: string;
  noActiveDownloads: string;
  downloadsLabel: string;
  activeSuffix: string;
  speedUnitKb: string;
  speedUnitMb: string;
  decimalSeparator: string;
};

/**
 * The tray menu is built in the Rust process, where the Svelte `$t` helper is
 * not reachable — that is why its labels used to stay English no matter which
 * language the rest of the app used. This resolves the strings for the active
 * locale and pushes them to the backend, which re-renders the native menu in
 * place (no restart needed).
 */
export function currentTrayLabels(): TrayLabels {
  const tr = get(t) as Translate;
  return {
    quit: tr("tray.quit"),
    channels: tr("settings.channels.tray_header"),
    noActiveDownloads: tr("tray.no_active_downloads"),
    downloadsLabel: tr("nav.downloads"),
    activeSuffix: tr("tray.active_suffix"),
    speedUnitKb: tr("tray.speed_unit_kb"),
    speedUnitMb: tr("tray.speed_unit_mb"),
    decimalSeparator: tr("tray.decimal_separator"),
  };
}

let syncedLocale: string | null = null;

/** Pushes the labels for the active locale to the native tray. */
export async function syncTrayLabels(force = false): Promise<void> {
  const active = get(locale);
  if (!force && active === syncedLocale) {
    return;
  }
  syncedLocale = active;
  try {
    await invoke("sync_tray_labels", { labels: currentTrayLabels() });
  } catch (e) {
    // The tray is a desktop-only nicety: never let it break the UI.
    console.warn("[tray] could not sync labels:", e);
  }
}
