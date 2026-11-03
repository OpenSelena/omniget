export type MediaKind = "video" | "audio" | "image" | "other";

const VIDEO = ["mp4", "m4v", "mov", "mkv", "webm", "avi", "flv", "ts", "wmv"];
const AUDIO = ["mp3", "m4a", "aac", "flac", "wav", "ogg", "opus", "wma"];
const IMAGE = ["jpg", "jpeg", "png", "webp", "gif", "heic", "avif"];

/** Kind of a downloaded file, from its extension. */
export function mediaKind(path: string): MediaKind {
  const name = path.split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return "other";
  const ext = name.slice(dot + 1).toLowerCase();
  if (VIDEO.includes(ext)) return "video";
  if (AUDIO.includes(ext)) return "audio";
  if (IMAGE.includes(ext)) return "image";
  return "other";
}
