import { notesSettingsGet, notesSettingsSet } from "$lib/notes-bridge";

export type NotesEditorMode = "edit" | "source" | "preview";

const STORAGE_KEY = "study.notes.shell.v1";
const RETURN_URL_KEY = "study.notes.return_url";

const DEFAULT_DOCK_LEFT = 240;
const DEFAULT_DOCK_RIGHT = 280;
const DOCK_MIN = 180;
const DOCK_MAX = 480;

type PersistedShape = {
  dockLeftWidth: number;
  dockRightWidth: number;
  dockLeftVisible: boolean;
  dockRightVisible: boolean;
};

class NotesShellStore {
  dockLeftWidth = $state(DEFAULT_DOCK_LEFT);
  dockRightWidth = $state(DEFAULT_DOCK_RIGHT);
  dockLeftVisible = $state(true);
  dockRightVisible = $state(true);

  currentPageName = $state<string | null>(null);
  currentPageTitle = $state<string | null>(null);
  notebookName = $state<string>("Pessoal");
  wordCount = $state(0);
  charCount = $state(0);
  mode = $state<NotesEditorMode>("edit");
  saving = $state(false);
  hydrated = $state(false);

  loadLocal() {
    if (typeof localStorage === "undefined") return;
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const data = JSON.parse(raw) as Partial<PersistedShape>;
      if (typeof data.dockLeftWidth === "number") {
        this.dockLeftWidth = clampDock(data.dockLeftWidth);
      }
      if (typeof data.dockRightWidth === "number") {
        this.dockRightWidth = clampDock(data.dockRightWidth);
      }
      if (typeof data.dockLeftVisible === "boolean") {
        this.dockLeftVisible = data.dockLeftVisible;
      }
      if (typeof data.dockRightVisible === "boolean") {
        this.dockRightVisible = data.dockRightVisible;
      }
    } catch {
      /* ignore */
    }
  }

  async hydrateFromBackend() {
    try {
      const r = await notesSettingsGet("layout_snapshot");
      const v = r?.value;
      if (v && typeof v === "object" && !Array.isArray(v)) {
        const obj = v as Record<string, unknown>;
        if (typeof obj.dockLeftWidth === "number") {
          this.dockLeftWidth = clampDock(obj.dockLeftWidth);
        }
        if (typeof obj.dockRightWidth === "number") {
          this.dockRightWidth = clampDock(obj.dockRightWidth);
        }
        if (typeof obj.dockLeftVisible === "boolean") {
          this.dockLeftVisible = obj.dockLeftVisible;
        }
        if (typeof obj.dockRightVisible === "boolean") {
          this.dockRightVisible = obj.dockRightVisible;
        }
      }
    } catch {
      /* ignore */
    }
    this.hydrated = true;
  }

  persistLocal() {
    if (typeof localStorage === "undefined") return;
    try {
      const payload: PersistedShape = {
        dockLeftWidth: this.dockLeftWidth,
        dockRightWidth: this.dockRightWidth,
        dockLeftVisible: this.dockLeftVisible,
        dockRightVisible: this.dockRightVisible,
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
    } catch {
      /* ignore */
    }
  }

  async persistBackend() {
    try {
      await notesSettingsSet({
        key: "layout_snapshot",
        value: {
          dockLeftWidth: this.dockLeftWidth,
          dockRightWidth: this.dockRightWidth,
          dockLeftVisible: this.dockLeftVisible,
          dockRightVisible: this.dockRightVisible,
        },
      });
    } catch {
      /* ignore */
    }
  }

  setDockLeftWidth(px: number) {
    this.dockLeftWidth = clampDock(px);
    this.persistLocal();
  }

  setDockRightWidth(px: number) {
    this.dockRightWidth = clampDock(px);
    this.persistLocal();
  }

  toggleDockLeft() {
    this.dockLeftVisible = !this.dockLeftVisible;
    this.persistLocal();
    void this.persistBackend();
  }

  toggleDockRight() {
    this.dockRightVisible = !this.dockRightVisible;
    this.persistLocal();
    void this.persistBackend();
  }

  rememberReturnUrl(url: string) {
    if (typeof sessionStorage === "undefined") return;
    if (url.startsWith("/study/notes")) return;
    try {
      sessionStorage.setItem(RETURN_URL_KEY, url);
    } catch {
      /* ignore */
    }
  }

  consumeReturnUrl(): string {
    if (typeof sessionStorage === "undefined") return "/study/library";
    try {
      const stored = sessionStorage.getItem(RETURN_URL_KEY);
      if (stored && !stored.startsWith("/study/notes")) return stored;
    } catch {
      /* ignore */
    }
    return "/study/library";
  }

  setActivePage(name: string | null, title: string | null) {
    this.currentPageName = name;
    this.currentPageTitle = title;
  }

  setCounts(words: number, chars: number) {
    this.wordCount = words;
    this.charCount = chars;
  }

  setMode(mode: NotesEditorMode) {
    this.mode = mode;
  }

  setSaving(saving: boolean) {
    this.saving = saving;
  }
}

function clampDock(px: number): number {
  if (!Number.isFinite(px)) return DEFAULT_DOCK_LEFT;
  return Math.min(DOCK_MAX, Math.max(DOCK_MIN, Math.round(px)));
}

export const notesShell = new NotesShellStore();
export const NOTES_SHELL_DOCK_MIN = DOCK_MIN;
export const NOTES_SHELL_DOCK_MAX = DOCK_MAX;
