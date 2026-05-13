const STORAGE_KEY = "omniget.tracking.settings";

export type TrackerSettings = {
  notificationsEnabled: boolean;
};

const DEFAULTS: TrackerSettings = {
  notificationsEnabled: true,
};

function readInitial(): TrackerSettings {
  if (typeof localStorage === "undefined") return { ...DEFAULTS };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw) as Partial<TrackerSettings>;
    return { ...DEFAULTS, ...parsed };
  } catch {
    return { ...DEFAULTS };
  }
}

let settings = $state<TrackerSettings>(readInitial());

function persist() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // localStorage unavailable — state still works in-memory
  }
}

export function getTrackerSettings(): TrackerSettings {
  return settings;
}

export function updateTrackerSettings(patch: Partial<TrackerSettings>) {
  settings = { ...settings, ...patch };
  persist();
}
