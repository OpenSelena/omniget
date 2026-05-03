import type {
  PetMeta,
  PetStateId,
  PetStateMood,
  PetVibe,
  PetVibeMood,
} from "./types";

export const STATE_MOOD: Record<PetStateId, PetStateMood> = {
  idle: "positive",
  waving: "positive",
  jumping: "positive",
  "running-right": "positive",
  "running-left": "positive",
  running: "positive",
  review: "positive",
  waiting: "neutral",
  failed: "negative",
};

export const MOOD_BY_EVENT = {
  cardCorrect: "jumping",
  cardWrong: "failed",
  streakActive: "idle",
  streakMilestone: "waving",
  studying: "review",
  syncing: "waiting",
  syncSuccess: "waving",
  syncError: "failed",
  emptyState: "idle",
  welcome: "waving",
  goalReached: "jumping",
} as const satisfies Record<string, PetStateId>;

export type AnkiMoodEvent = keyof typeof MOOD_BY_EVENT;

export const VIBE_MOOD: Record<PetVibeMood, PetVibe[]> = {
  positive: ["cozy", "playful", "cheerful", "focused", "heroic", "wholesome"],
  ambiguous: ["calm", "mischievous", "mystical"],
  negative: ["edgy", "chaotic", "melancholic"],
};

export function getStateMood(state: PetStateId): PetStateMood {
  return STATE_MOOD[state];
}

export function getStateForEvent(event: AnkiMoodEvent): PetStateId {
  return MOOD_BY_EVENT[event];
}

export function getPetMood(vibes: PetVibe[]): PetVibeMood {
  if (vibes.length === 0) return "ambiguous";
  const counts: Record<PetVibeMood, number> = { positive: 0, ambiguous: 0, negative: 0 };
  for (const vibe of vibes) {
    if (VIBE_MOOD.positive.includes(vibe)) counts.positive += 1;
    else if (VIBE_MOOD.negative.includes(vibe)) counts.negative += 1;
    else counts.ambiguous += 1;
  }
  if (counts.negative > counts.positive && counts.negative >= counts.ambiguous) return "negative";
  if (counts.positive >= counts.negative && counts.positive >= counts.ambiguous) return "positive";
  return "ambiguous";
}

export function pickPetForMood(
  mood: PetVibeMood,
  availablePets: PetMeta[],
): string | null {
  const matches = availablePets.filter((pet) => getPetMood(pet.vibes) === mood);
  const pool = matches.length > 0 ? matches : availablePets;
  if (pool.length === 0) return null;
  return pool[0].slug;
}
