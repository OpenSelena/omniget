export type WhisperModel = { id: string; installed: boolean };

/** Best speed/quality first, so the most useful installed model is picked. */
const PREFERENCE = [
  "large-v3-turbo-q5_0",
  "large-v3-turbo",
  "small-q5_1",
  "medium-q5_0",
  "base-q5_1",
  "large-v3-q5_0",
  "large-v3",
  "tiny-q5_1",
];

/** Returns the installed model to use for automatic subtitles, or null. */
export function pickWhisperModel(models: WhisperModel[]): string | null {
  const installed = models.filter((m) => m.installed).map((m) => m.id);
  for (const id of PREFERENCE) {
    if (installed.includes(id)) return id;
  }
  return installed[0] ?? null;
}
