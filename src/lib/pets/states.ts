import type { PetStateConfig, PetStateId } from "./types";

export const PET_STATES: Record<PetStateId, PetStateConfig> = {
  idle: {
    id: "idle",
    label: "Idle",
    row: 0,
    frames: 6,
    durationMs: 1100,
    purpose: "Neutral breathing and blinking loop",
  },
  "running-right": {
    id: "running-right",
    label: "Run Right",
    row: 1,
    frames: 8,
    durationMs: 1060,
    purpose: "Directional locomotion to the right",
  },
  "running-left": {
    id: "running-left",
    label: "Run Left",
    row: 2,
    frames: 8,
    durationMs: 1060,
    purpose: "Directional locomotion to the left",
  },
  waving: {
    id: "waving",
    label: "Waving",
    row: 3,
    frames: 4,
    durationMs: 700,
    purpose: "Greeting or attention gesture",
  },
  jumping: {
    id: "jumping",
    label: "Jumping",
    row: 4,
    frames: 5,
    durationMs: 840,
    purpose: "Anticipation, lift, peak, descent, settle",
  },
  failed: {
    id: "failed",
    label: "Failed",
    row: 5,
    frames: 8,
    durationMs: 1220,
    purpose: "Readable error or sad reaction",
  },
  waiting: {
    id: "waiting",
    label: "Waiting",
    row: 6,
    frames: 6,
    durationMs: 1010,
    purpose: "Patient idle variant",
  },
  running: {
    id: "running",
    label: "Running",
    row: 7,
    frames: 6,
    durationMs: 820,
    purpose: "Generic in-place run loop",
  },
  review: {
    id: "review",
    label: "Review",
    row: 8,
    frames: 6,
    durationMs: 1030,
    purpose: "Focused inspecting or thinking loop",
  },
};

export const PET_STATE_IDS: PetStateId[] = Object.keys(PET_STATES) as PetStateId[];
