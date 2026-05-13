import { pluginInvoke } from "$lib/plugin-invoke";
import {
  musicPlayer,
  type MusicTrack,
} from "$lib/study-music/player-store.svelte";
import type { YoutubeSearchVideoItem } from "$lib/study-bridge";

export type PlayableVideo = {
  video_id: string;
  title: string;
  channel_title: string | null;
  thumbnail_url: string | null;
  duration_ms?: number | null;
};

function toPlayable(item: YoutubeSearchVideoItem): PlayableVideo {
  return {
    video_id: item.video_id,
    title: item.title,
    channel_title: item.channel_title,
    thumbnail_url: item.thumbnail_url,
    duration_ms: null,
  };
}

async function ensureTrack(rt: PlayableVideo): Promise<MusicTrack> {
  const ensured = await pluginInvoke<{ id: number }>(
    "study",
    "study:music:youtube:ensure_external_track",
    {
      video_id: rt.video_id,
      title: rt.title,
      artist: rt.channel_title ?? "",
      album: null,
      duration_ms: rt.duration_ms ?? null,
      thumbnail: rt.thumbnail_url ?? null,
    },
  );
  return {
    id: ensured.id,
    path: `external://youtube/${rt.video_id}`,
    title: rt.title,
    artist: rt.channel_title ?? null,
    album: null,
    duration_ms: rt.duration_ms ?? null,
    cover_path: rt.thumbnail_url ?? null,
    source: "youtube",
    youtube_url: `https://www.youtube.com/watch?v=${rt.video_id}`,
    youtube_video_id: rt.video_id,
    youtube_thumbnail: rt.thumbnail_url ?? undefined,
  };
}

export async function playYoutubeVideoItem(
  item: YoutubeSearchVideoItem,
  queue?: YoutubeSearchVideoItem[],
): Promise<void> {
  const items = queue && queue.length > 0 ? queue : [item];
  const playableQueue = items.map(toPlayable);
  const tracks = await Promise.all(playableQueue.map(ensureTrack));
  const target = tracks.find((t) => t.youtube_video_id === item.video_id);
  if (target) await musicPlayer.play(target, tracks);
}

export async function playYoutubeVideoId(
  videoId: string,
  title: string,
  channelTitle: string | null,
  thumbnailUrl: string | null,
): Promise<void> {
  const playable: PlayableVideo = {
    video_id: videoId,
    title,
    channel_title: channelTitle,
    thumbnail_url: thumbnailUrl,
    duration_ms: null,
  };
  const track = await ensureTrack(playable);
  await musicPlayer.play(track, [track]);
}
