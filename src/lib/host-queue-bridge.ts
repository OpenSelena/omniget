import { invoke } from "@tauri-apps/api/core";

export type QueueKind =
  | "video"
  | "audio"
  | "image"
  | "pdf"
  | "book"
  | "webpage"
  | "telegram_media"
  | "course_lesson"
  | "generic";

export type EnqueueExternalArgs = {
  platform: string;
  title: string;
  output_path: string;
  total_bytes?: number;
  thumbnail_url?: string;
  kind?: QueueKind;
  url?: string;
  file_name?: string;
};

export type EnqueueExternalResult = {
  queue_id: number;
};

export type ReportProgressArgs = {
  queue_id: number;
  percent: number;
  downloaded_bytes: number;
  speed_bytes_per_sec: number;
};

export type ReportCompleteArgs = {
  queue_id: number;
  success: boolean;
  file_path?: string;
  error?: string;
  file_size_bytes?: number;
};

export async function hostQueueEnqueueExternal(
  args: EnqueueExternalArgs,
): Promise<EnqueueExternalResult> {
  return invoke<EnqueueExternalResult>("host_queue_enqueue_external", { args });
}

export async function hostQueueReportProgress(args: ReportProgressArgs): Promise<void> {
  await invoke<void>("host_queue_report_progress", { args });
}

export async function hostQueueReportComplete(args: ReportCompleteArgs): Promise<void> {
  await invoke<void>("host_queue_report_complete", { args });
}

export function kindFromPlatform(platform: string): QueueKind {
  const p = platform.toLowerCase();
  if (
    [
      "youtube",
      "vimeo",
      "twitch",
      "bilibili",
      "tiktok",
      "twitter",
      "x",
      "instagram",
      "reddit",
      "bluesky",
      "facebook",
      "generic_ytdlp",
    ].includes(p)
  ) {
    return "video";
  }
  if (["soundcloud", "spotify"].includes(p)) return "audio";
  if (p === "pinterest") return "image";
  if (["magnet", "p2p", "torrent"].includes(p)) return "generic";
  if (["telegram", "telegram_media"].includes(p)) return "telegram_media";
  if (["courses", "course_lesson"].includes(p)) return "course_lesson";
  if (["annas_archive", "book", "libgen", "gutendex"].includes(p)) return "book";
  if (p === "pdf") return "pdf";
  if (["webpage", "embed"].includes(p)) return "webpage";
  return "generic";
}
