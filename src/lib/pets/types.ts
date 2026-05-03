export type PetStateId =
  | "idle"
  | "running-right"
  | "running-left"
  | "waving"
  | "jumping"
  | "failed"
  | "waiting"
  | "running"
  | "review";

export type PetVibe =
  | "cozy"
  | "calm"
  | "playful"
  | "cheerful"
  | "focused"
  | "mischievous"
  | "heroic"
  | "edgy"
  | "mystical"
  | "wholesome"
  | "chaotic"
  | "melancholic";

export type PetStateMood = "positive" | "neutral" | "negative";
export type PetVibeMood = "positive" | "ambiguous" | "negative";

export type PetStateConfig = {
  id: PetStateId;
  label: string;
  row: number;
  frames: number;
  durationMs: number;
  purpose: string;
};

export type PetMeta = {
  slug: string;
  display_name: string;
  description: string;
  spritesheet_url: string;
  vibes: PetVibe[];
  tags: string[];
};
