import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getInstanceStreamingPolicy } from "$lib/stores/omnidisc-store.svelte";
import { getVoiceSession } from "$lib/stores/omnidisc-voice-store.svelte";
import type { StreamingPolicy } from "$lib/omnidisc/types";

export type SourceKind = "display" | "window";
export type AudioMode = { mode: "none" } | { mode: "app"; pid: number } | { mode: "system" };
export type StreamMode = "text" | "game";

export interface SourceId {
  kind: SourceKind | "synthetic";
  id?: number;
  width?: number;
  height?: number;
}

export interface StreamSource {
  id: SourceId;
  title: string;
  app_name?: string;
  width: number;
  height: number;
  thumbnail?: string;
}

export interface AudioApp {
  pid: number;
  name: string;
  bundle_id: string;
}

export interface StreamSources {
  displays: StreamSource[];
  windows: StreamSource[];
  apps: AudioApp[];
  app_audio_supported: boolean;
  system_audio_supported: boolean;
}

export interface PublishStats {
  width: number;
  height: number;
  fps_captured: number;
  fps_encoded: number;
  fps_sent: number;
  bitrate_kbps: number;
  target_kbps: number;
  configured_kbps: number;
  codec?: "h264" | "h265";
  encoder: string;
  hardware?: boolean;
  encode_ms: number;
  keyframes: number;
  frames_dropped: number;
  rtt_ms?: number;
  packet_loss?: number;
  quality_limitation: string;
  audio: AudioMode;
}

export interface WatchStats {
  user_id: string;
  width: number;
  height: number;
  fps_received: number;
  fps_rendered: number;
  bitrate_kbps: number;
  codec: string;
  decoder: string;
  packet_loss?: number;
  jitter_ms?: number;
  frames_dropped: number;
  freeze_count: number;
}

export interface StreamStats {
  publishing: PublishStats | null;
  watching: WatchStats[];
}

export interface StartArgs {
  source: SourceId;
  fps: number;
  height?: number;
  audio: AudioMode;
  bitrate_kbps?: number;
  mode: StreamMode;
  cursor: boolean;
  policy?: StreamingPolicy;
}

export interface MediaCapabilities {
  voice: boolean;
  screen_share: boolean;
  stream_viewer: boolean;
}

/**
 * What the interface assumes until the backend answers — which it does on
 * mount, before anyone can click. Assuming "yes" keeps the buttons from
 * blinking into existence on the platforms where they do work; when the answer
 * is "no" they end up disabled with the reason attached, never silently gone.
 */
const ASSUMED_CAPABILITIES: MediaCapabilities = {
  voice: true,
  screen_share: true,
  stream_viewer: true,
};

const RESOLUTIONS = [540, 720, 1080, 1440, 2160];
const FRAMERATES = [15, 30, 60, 90, 120, 144];

let publishing = $state<PublishStats | null>(null);
let watchingIds = $state<string[]>([]);
let stats = $state<StreamStats | null>(null);
let busy = $state(false);
let lastError = $state<string | null>(null);
let streamers = $state<Record<string, boolean>>({});
let capabilities = $state<MediaCapabilities>(ASSUMED_CAPABILITIES);

let unlisten: UnlistenFn | null = null;
let initialized = false;

function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}

export function isPublishing(): boolean {
  return publishing !== null;
}

export function getPublishStats(): PublishStats | null {
  return publishing;
}

export function getStreamStats(): StreamStats | null {
  return stats;
}

export function isWatching(userId: string): boolean {
  return watchingIds.includes(userId);
}

export function isStreamBusy(): boolean {
  return busy;
}

export function getStreamError(): string | null {
  return lastError;
}

export function clearStreamError() {
  lastError = null;
}

export function getMediaCapabilities(): MediaCapabilities {
  return capabilities;
}

export async function refreshMediaCapabilities(): Promise<void> {
  try {
    capabilities = await invoke<MediaCapabilities>("omnidisc_media_capabilities");
  } catch (e) {
    console.warn("[omnidisc] media capabilities unavailable", errorText(e));
  }
}

export function isStreamer(userId: string): boolean {
  return streamers[userId] === true;
}

export function markStreamer(userId: string, on: boolean) {
  if (streamers[userId] === on) return;
  streamers = { ...streamers, [userId]: on };
}

export function allowedResolutions(policy: StreamingPolicy | null): number[] {
  const max = policy?.max_height ?? 2160;
  return RESOLUTIONS.filter((h) => h <= max);
}

export function allowedFramerates(policy: StreamingPolicy | null): number[] {
  const max = policy?.max_fps ?? 120;
  return FRAMERATES.filter((f) => f <= max);
}

export function widthForHeight(height: number): number {
  switch (height) {
    case 540: return 960;
    case 720: return 1280;
    case 1080: return 1920;
    case 1440: return 2560;
    case 2160: return 3840;
    default: return Math.round((height * 16) / 9) & ~1;
  }
}

function defaultKbps(height: number, fps: number): number {
  const base =
    height <= 540 ? 1500 : height <= 720 ? 2500 : height <= 1080 ? 5000 : height <= 1440 ? 9000 : 16000;
  const scale =
    fps <= 15 ? 0.65 : fps <= 30 ? 1.0 : fps <= 60 ? 1.6 : fps <= 90 ? 2.1 : fps <= 120 ? 2.6 : 2.9;
  return Math.max(300, Math.round(base * scale));
}

export function kbpsFor(policy: StreamingPolicy | null, height: number, fps: number): number {
  const key = `${height}p${fps}`;
  const raw = policy?.overrides?.[key] ?? defaultKbps(height, fps);
  const min = policy?.min_kbps ?? 500;
  const max = policy?.max_kbps ?? 20000;
  return Math.min(max, Math.max(min, raw));
}

export function codecFor(policy: StreamingPolicy | null, height: number, fps: number): "h264" | "h265" {
  const allow = policy?.allow_h265 ?? true;
  return allow && height >= 2160 && fps > 60 ? "h265" : "h264";
}

export function currentPolicy(): StreamingPolicy | null {
  const session = getVoiceSession();
  return getInstanceStreamingPolicy(session?.instanceId ?? null);
}

export async function refreshSources(): Promise<StreamSources | null> {
  try {
    return await invoke<StreamSources>("omnidisc_stream_sources");
  } catch (e) {
    lastError = errorText(e);
    return null;
  }
}

export async function startStream(args: StartArgs): Promise<boolean> {
  busy = true;
  lastError = null;
  try {
    const policy = currentPolicy() ?? undefined;
    publishing = await invoke<PublishStats>("omnidisc_stream_start", { args: { ...args, policy } });
    return true;
  } catch (e) {
    lastError = errorText(e);
    return false;
  } finally {
    busy = false;
  }
}

export async function stopStream(): Promise<void> {
  busy = true;
  try {
    await invoke("omnidisc_stream_stop");
  } catch (e) {
    console.warn("[omnidisc] stop stream failed", errorText(e));
  } finally {
    publishing = null;
    busy = false;
  }
}

export async function watchStream(userId: string): Promise<boolean> {
  busy = true;
  lastError = null;
  try {
    await invoke("omnidisc_stream_watch", { userId });
    if (!watchingIds.includes(userId)) watchingIds = [...watchingIds, userId];
    return true;
  } catch (e) {
    lastError = errorText(e);
    return false;
  } finally {
    busy = false;
  }
}

export async function unwatchStream(userId: string): Promise<void> {
  try {
    await invoke("omnidisc_stream_unwatch", { userId });
  } catch (e) {
    console.warn("[omnidisc] unwatch failed", errorText(e));
  } finally {
    watchingIds = watchingIds.filter((id) => id !== userId);
  }
}

export async function setStreamVolume(userId: string, gain: number): Promise<void> {
  try {
    await invoke("omnidisc_stream_set_volume", { userId, gain: Math.min(2, Math.max(0, gain)) });
  } catch (e) {
    console.warn("[omnidisc] stream volume failed", errorText(e));
  }
}

export async function refreshStreamStats(): Promise<StreamStats | null> {
  try {
    stats = await invoke<StreamStats>("omnidisc_stream_stats");
    if (stats.publishing) publishing = stats.publishing;
    return stats;
  } catch {
    return null;
  }
}

interface StreamEventPayload {
  type: string;
  audio?: AudioMode;
}

export async function initStream(): Promise<void> {
  if (initialized) return;
  initialized = true;
  void refreshMediaCapabilities();
  try {
    unlisten = await listen<StreamEventPayload>("omnidisc://voice", (event) => {
      const p = event.payload;
      if (p.type === "stream_stopped") {
        publishing = null;
      }
    });
  } catch (e) {
    initialized = false;
    console.warn("[omnidisc] stream events subscribe failed", errorText(e));
  }
}

export function teardownStream() {
  if (unlisten) unlisten();
  unlisten = null;
  initialized = false;
}
