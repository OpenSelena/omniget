const STORAGE_KEY = "omniget.tracking.privacy";

function readInitial(): boolean {
  if (typeof localStorage === "undefined") return false;
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

let enabled = $state(readInitial());

function persist() {
  try {
    localStorage.setItem(STORAGE_KEY, enabled ? "1" : "0");
  } catch {
    // localStorage unavailable — state still works in-memory
  }
}

export function privacyEnabled(): boolean {
  return enabled;
}

export function togglePrivacy() {
  enabled = !enabled;
  persist();
}

export function setPrivacy(v: boolean) {
  enabled = v;
  persist();
}

export function maskCode(code: string | null | undefined): string {
  if (!code) return "";
  if (!enabled) return code;
  if (code.length <= 6) return "•".repeat(code.length);
  return code.slice(0, 2) + "•".repeat(code.length - 5) + code.slice(-3);
}
