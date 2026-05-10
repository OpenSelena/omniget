export type MusicThemePreset =
  | "default"
  | "indigo"
  | "spotify"
  | "purple"
  | "amber"
  | "custom";

export type MusicTheme = {
  preset: MusicThemePreset;
  accent: string;
  useDominant: boolean;
};

const STORAGE_KEY = "study.music.theme.v1";

export const PRESET_ACCENTS: Record<Exclude<MusicThemePreset, "custom">, string> = {
  default: "",
  indigo: "#6366f1",
  spotify: "#1db954",
  purple: "#a855f7",
  amber: "#f59e0b",
};

const DEFAULT_THEME: MusicTheme = {
  preset: "default",
  accent: "",
  useDominant: true,
};

function isHexColor(value: string): boolean {
  return /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.test(value);
}

class MusicThemeStore {
  preset = $state<MusicThemePreset>("default");
  accent = $state("");
  useDominant = $state(true);

  load() {
    if (typeof localStorage === "undefined") return;
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const data = JSON.parse(raw) as Partial<MusicTheme>;
      if (data.preset && data.preset in PRESET_ACCENTS) {
        this.preset = data.preset as MusicThemePreset;
      } else if (data.preset === "custom") {
        this.preset = "custom";
      }
      if (typeof data.accent === "string" && (isHexColor(data.accent) || data.accent === "")) {
        this.accent = data.accent;
      }
      if (typeof data.useDominant === "boolean") {
        this.useDominant = data.useDominant;
      }
    } catch {
      /* ignore */
    }
  }

  persist() {
    if (typeof localStorage === "undefined") return;
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          preset: this.preset,
          accent: this.accent,
          useDominant: this.useDominant,
        }),
      );
    } catch {
      /* ignore */
    }
  }

  setPreset(preset: MusicThemePreset) {
    this.preset = preset;
    if (preset !== "custom") {
      this.accent = PRESET_ACCENTS[preset];
    }
    this.persist();
  }

  setAccent(value: string) {
    if (!isHexColor(value)) return;
    this.accent = value;
    this.preset = "custom";
    this.persist();
  }

  toggleUseDominant() {
    this.useDominant = !this.useDominant;
    this.persist();
  }

  reset() {
    this.preset = DEFAULT_THEME.preset;
    this.accent = DEFAULT_THEME.accent;
    this.useDominant = DEFAULT_THEME.useDominant;
    this.persist();
  }

  get effectiveAccent(): string | null {
    if (this.preset === "default" || !this.accent) return null;
    return this.accent;
  }
}

export const musicTheme = new MusicThemeStore();
