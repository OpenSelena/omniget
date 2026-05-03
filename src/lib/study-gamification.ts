import { pluginInvoke } from "$lib/plugin-invoke";

export type BonusEntry = {
  source: string;
  amount: number;
};

export type AwardResult = {
  xp_added: number;
  total_xp: number;
  previous_level: number;
  new_level: number;
  leveled_up: boolean;
  achievements_unlocked: string[];
  bonus_xp: number;
  bonus_breakdown: BonusEntry[];
};

let lastToastAt = 0;
const TOAST_COOLDOWN_MS = 800;

export type GamificationToast = {
  kind: "level_up" | "achievement";
  text: string;
};

const listeners = new Set<(toast: GamificationToast) => void>();

export function onGamificationToast(
  listener: (toast: GamificationToast) => void,
): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function fireToast(toast: GamificationToast) {
  const now = Date.now();
  if (now - lastToastAt < TOAST_COOLDOWN_MS) return;
  lastToastAt = now;
  for (const l of listeners) {
    try {
      l(toast);
    } catch {}
  }
}

export async function awardXp(
  source: string,
  amount: number,
  metadata: Record<string, unknown> = {},
): Promise<void> {
  if (amount <= 0) return;
  try {
    const res = await pluginInvoke<AwardResult>(
      "study",
      "study:gamification:award",
      { source, amount, metadata },
    );
    if (!res) return;
    if (res.leveled_up) {
      fireToast({ kind: "level_up", text: `Subiu para nível ${res.new_level}!` });
    }
    for (const code of res.achievements_unlocked ?? []) {
      fireToast({
        kind: "achievement",
        text: `Conquista desbloqueada: ${code}`,
      });
    }
  } catch {}
}

export async function bumpCounter(
  key: string,
  delta: number = 1,
): Promise<void> {
  if (delta === 0) return;
  try {
    await pluginInvoke("study", "study:gamification:counter:bump", {
      key,
      delta,
    });
  } catch {}
}
