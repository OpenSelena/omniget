const PREFIX = "study-music-sc-";
const KEY_FIRST_DONE = `${PREFIX}firstDownloadDone`;
const KEY_LAST_CODEC = `${PREFIX}lastCodec`;
const KEY_LAST_DIR = `${PREFIX}lastDownloadDir`;
const KEY_REMEMBER = `${PREFIX}rememberDefaults`;

function safeGet(key: string): string | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function safeSet(key: string, value: string) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(key, value);
  } catch {
    /* ignore quota / disabled */
  }
}

export function getFirstDownloadDone(): boolean {
  return safeGet(KEY_FIRST_DONE) === "1";
}

export function setFirstDownloadDone() {
  safeSet(KEY_FIRST_DONE, "1");
}

export function getLastCodec(): string | null {
  const v = safeGet(KEY_LAST_CODEC);
  return v && v.trim() ? v : null;
}

export function setLastCodec(codec: string) {
  safeSet(KEY_LAST_CODEC, codec);
}

export function getLastDownloadDir(): string | null {
  const v = safeGet(KEY_LAST_DIR);
  return v && v.trim() ? v : null;
}

export function setLastDownloadDir(dir: string) {
  safeSet(KEY_LAST_DIR, dir);
}

export function getRememberDefaults(): boolean {
  const v = safeGet(KEY_REMEMBER);
  return v === null ? true : v === "1";
}

export function setRememberDefaults(remember: boolean) {
  safeSet(KEY_REMEMBER, remember ? "1" : "0");
}

export type ScDownloadPrefs = {
  firstDone: boolean;
  lastCodec: string | null;
  lastDir: string | null;
  remember: boolean;
};

export function readPrefs(): ScDownloadPrefs {
  return {
    firstDone: getFirstDownloadDone(),
    lastCodec: getLastCodec(),
    lastDir: getLastDownloadDir(),
    remember: getRememberDefaults(),
  };
}
