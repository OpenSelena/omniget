// Reading-time theme override for the omniget app theme.
// Instead of an isolated `data-read-theme` system, we swap the actual app theme
// while the reader is open, then restore the user's preference on exit.
//
// Preference key: `study.read.reading_theme`
//   - "app" (default): keep current app theme, no swap
//   - "eink-day" / "eink-sepia" / "eink-night" / any omniget theme id: swap to it on open

const READING_THEME_KEY = "study.read.reading_theme";
const PAPER_KEY = "study.read.paper";
const CURSOR_KEY = "study.read.cursor";

let previousAppTheme: string | null = null;

export function getReadingTheme(): string {
  if (typeof localStorage === "undefined") return "app";
  return localStorage.getItem(READING_THEME_KEY) ?? "app";
}

export function setReadingTheme(v: string) {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(READING_THEME_KEY, v);
}

export function getPaperFilter(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(PAPER_KEY) === "1";
}

export function setPaperFilter(v: boolean) {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(PAPER_KEY, v ? "1" : "0");
}

export function getCursorLine(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(CURSOR_KEY) === "1";
}

export function setCursorLine(v: boolean) {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(CURSOR_KEY, v ? "1" : "0");
}

export function applyFocusMode(on: boolean) {
  if (typeof document === "undefined") return;
  if (on) {
    document.documentElement.dataset.readFocus = "1";
  } else {
    delete document.documentElement.dataset.readFocus;
  }
}

function currentAppTheme(): string | null {
  if (typeof document === "undefined") return null;
  return document.documentElement.getAttribute("data-theme");
}

function setAppTheme(theme: string) {
  if (typeof document === "undefined") return;
  document.documentElement.setAttribute("data-theme", theme);
}

export function pushReadingTheme() {
  if (typeof document === "undefined") return;
  const pref = getReadingTheme();
  if (!pref || pref === "app") return;
  if (previousAppTheme == null) {
    previousAppTheme = currentAppTheme();
  }
  setAppTheme(pref);
}

export function popReadingTheme() {
  if (typeof document === "undefined") return;
  if (previousAppTheme != null) {
    setAppTheme(previousAppTheme);
    previousAppTheme = null;
  }
}

export function applyReadingTheme(pref: string) {
  setReadingTheme(pref);
  if (!pref || pref === "app") {
    popReadingTheme();
    return;
  }
  if (previousAppTheme == null) {
    previousAppTheme = currentAppTheme();
  }
  setAppTheme(pref);
}

export const AVAILABLE_READING_THEMES: {
  id: string;
  label: string;
  swatch: string;
}[] = [
  { id: "app", label: "App default", swatch: "linear-gradient(135deg,#888 50%,#ccc 50%)" },
  { id: "eink-day", label: "E-ink Day", swatch: "#f5f2ea" },
  { id: "eink-sepia", label: "E-ink Sepia", swatch: "#f0e6d2" },
  { id: "eink-night", label: "E-ink Night", swatch: "#0a0a0a" },
  { id: "light", label: "Light", swatch: "#fafafa" },
  { id: "dark", label: "Dark", swatch: "#0a0a0a" },
  { id: "catppuccin-mocha", label: "Catppuccin Mocha", swatch: "#1e1e2e" },
  { id: "dracula", label: "Dracula", swatch: "#22212C" },
  { id: "one-dark-pro", label: "One Dark Pro", swatch: "#282c34" },
];
