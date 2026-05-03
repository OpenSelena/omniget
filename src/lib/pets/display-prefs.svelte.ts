import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PetStateId } from "./types";

export type DisplayPrefs = {
  enabled: boolean;
  motion_enabled: boolean;
  default_state: PetStateId;
  event_overrides: Partial<Record<string, PetStateId>>;
};

const DEFAULT_PREFS: DisplayPrefs = {
  enabled: true,
  motion_enabled: true,
  default_state: "idle",
  event_overrides: {},
};

let prefs = $state<DisplayPrefs>(DEFAULT_PREFS);
let loaded = $state(false);
let listenerActive = false;
let unlisten: UnlistenFn | null = null;

async function ensureListener() {
  if (listenerActive) return;
  listenerActive = true;
  try {
    unlisten = await listen<DisplayPrefs>("pets:prefs:changed", (event) => {
      prefs = { ...DEFAULT_PREFS, ...event.payload };
    });
  } catch {
    listenerActive = false;
  }
}

export async function loadDisplayPrefs(): Promise<DisplayPrefs> {
  if (!loaded) {
    try {
      const res = await invoke<DisplayPrefs>("pets_get_display_prefs");
      prefs = { ...DEFAULT_PREFS, ...res };
    } catch {
      prefs = DEFAULT_PREFS;
    }
    loaded = true;
    void ensureListener();
  }
  return prefs;
}

export async function setDisplayPrefs(next: Partial<DisplayPrefs>): Promise<DisplayPrefs> {
  const merged: DisplayPrefs = { ...prefs, ...next };
  try {
    const saved = await invoke<DisplayPrefs>("pets_set_display_prefs", { prefs: merged });
    prefs = { ...DEFAULT_PREFS, ...saved };
  } catch {
    prefs = merged;
  }
  loaded = true;
  return prefs;
}

export function getDisplayPrefs(): DisplayPrefs {
  return prefs;
}

export function petsEnabled(): boolean {
  return prefs.enabled;
}

export function motionEnabled(): boolean {
  return prefs.motion_enabled;
}

export function petsLoaded(): boolean {
  return loaded;
}
