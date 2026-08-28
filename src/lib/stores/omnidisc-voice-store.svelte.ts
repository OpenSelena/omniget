import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  getChannel,
  getChannelInstanceId,
  getGuild,
  getInstance,
  getInstances,
  isEncryptedChannel,
} from "$lib/stores/omnidisc-store.svelte";
import { updateSettings } from "$lib/stores/settings-store.svelte";
import { showToast } from "$lib/stores/toast-store.svelte";
import { translateBackendError } from "$lib/error-translate";
import { t } from "$lib/i18n";
import { get } from "svelte/store";

export type VoiceConnState = "idle" | "connecting" | "connected" | "reconnecting" | "failed";
export type VoiceQuality = "excellent" | "good" | "poor" | "lost" | "unknown";
export type DeviceKind = "input" | "output";
export type DeviceStatus = "lost" | "recovered" | "switched_to_default" | "listen_only" | "silent";
export type DeviceLoss = "unplugged" | "permission_revoked" | "busy" | "failed";

export interface VoiceSession {
  instanceId: string;
  url: string;
  guildId: string | null;
  channelId: string;
  room: string;
  e2ee: boolean;
}

export interface IncomingCall {
  instanceId: string;
  url: string;
  channelId: string;
  fromUserId: string;
  ringingUntil: number;
}

export interface AudioDevice {
  id: string;
  name: string;
  default: boolean;
}

export interface AudioDevices {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
}

export interface VoiceStatsWire {
  rtt_ms: number | null;
  packet_loss: number | null;
  jitter_ms: number | null;
  bitrate_out_kbps: number;
  bitrate_in_kbps: number;
  participants: number;
}

interface VoiceEventPayload {
  url: string | null;
  type: string;
  state?: VoiceConnState;
  reason?: string | null;
  user_id?: string;
  speaking?: boolean;
  quality?: VoiceQuality;
  rms_db?: number;
  peak?: number;
  code?: string;
  message?: string;
  kind?: DeviceKind;
  status?: DeviceStatus;
  cause?: DeviceLoss;
}

interface DispatchPayload {
  url: string;
  t: string;
  d: unknown;
}

interface VoiceSessionWire {
  url: string;
  guild_id?: string;
  channel_id: string;
  room: string;
  e2ee: boolean;
}

interface JoinResultWire {
  state: VoiceConnState;
  session: VoiceSessionWire;
  mic_error?: string;
  output_error?: string;
}

interface VoiceStatusWire {
  state: VoiceConnState;
  muted: boolean;
  deafened: boolean;
  session?: VoiceSessionWire;
  backend_available: boolean;
}

const VOLUMES_KEY = "omnidisc.voice.volumes";
const STATS_INTERVAL_MS = 5_000;
const MIC_SILENCE_AFTER_MS = 4_000;
const RING_TIMEOUT_MS = 60_000;

let session = $state<VoiceSession | null>(null);
let connState = $state<VoiceConnState>("idle");
let lastError = $state<string | null>(null);
let micError = $state<string | null>(null);
let outputError = $state<string | null>(null);
let muted = $state(false);
let deafened = $state(false);
let busy = $state(false);
let backendAvailable = $state(true);
let speaking = $state<Record<string, boolean>>({});
let quality = $state<VoiceQuality>("unknown");
let stats = $state<VoiceStatsWire | null>(null);
let volumes = $state<Record<string, number>>({});
let volumesLoaded = false;
let devices = $state<AudioDevices>({ inputs: [], outputs: [] });
let devicesLoading = $state(false);
let micTesting = $state(false);
let micLevel = $state<{ rmsDb: number; peak: number; at: number }>({ rmsDb: -100, peak: 0, at: 0 });

let incoming = $state<IncomingCall | null>(null);
let ringNow = $state(Date.now());

let unlisten: UnlistenFn | null = null;
let unlistenDispatch: UnlistenFn | null = null;
let initialized = false;
let statsTimer: ReturnType<typeof setInterval> | null = null;
let ringTimer: ReturnType<typeof setInterval> | null = null;

function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}

function hasStorage(): boolean {
  return typeof localStorage !== "undefined";
}

function loadVolumes() {
  if (volumesLoaded) return;
  volumesLoaded = true;
  if (!hasStorage()) return;
  try {
    const raw = localStorage.getItem(VOLUMES_KEY);
    if (!raw) return;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return;
    const clean: Record<string, number> = {};
    for (const [k, v] of Object.entries(parsed)) {
      if (typeof v === "number" && Number.isFinite(v)) clean[k] = Math.min(2, Math.max(0, v));
    }
    volumes = clean;
  } catch {
    volumes = {};
  }
}

function persistVolumes() {
  if (!hasStorage()) return;
  try {
    localStorage.setItem(VOLUMES_KEY, JSON.stringify(volumes));
  } catch {
    return;
  }
}

function instanceByUrl(url: string) {
  return getInstances().find((i) => i.url === url) ?? null;
}

function localUserId(): string | null {
  if (!session) return null;
  return getInstance(session.instanceId)?.userId ?? null;
}

export function getVoiceSession(): VoiceSession | null {
  return session;
}

export function getVoiceState(): VoiceConnState {
  return connState;
}

export function isVoiceBusy(): boolean {
  return busy;
}

export function getVoiceError(): string | null {
  return lastError;
}

export function getMicError(): string | null {
  return micError;
}

export function getOutputError(): string | null {
  return outputError;
}

export function isMuted(): boolean {
  return muted;
}

export function isDeafened(): boolean {
  return deafened;
}

export function isVoiceBackendAvailable(): boolean {
  return backendAvailable;
}

export function isSpeaking(userId: string): boolean {
  return speaking[userId] === true;
}

export function getVoiceQuality(): VoiceQuality {
  return quality;
}

export function getVoiceStats(): VoiceStatsWire | null {
  return stats;
}

export function getVolume(userId: string): number {
  loadVolumes();
  return volumes[userId] ?? 1;
}

export function getDevices(): AudioDevices {
  return devices;
}

export function isDevicesLoading(): boolean {
  return devicesLoading;
}

export function isMicTesting(): boolean {
  return micTesting;
}

export function getMicLevel(): { rmsDb: number; peak: number; at: number } {
  return micLevel;
}

export function micLevelStale(now = Date.now()): boolean {
  return micTesting && micLevel.at > 0 && now - micLevel.at > MIC_SILENCE_AFTER_MS;
}

export function isInVoiceChannel(channelId: string): boolean {
  return session?.channelId === channelId && connState !== "idle";
}

export function clearVoiceError() {
  lastError = null;
}

function toSession(wire: VoiceSessionWire): VoiceSession | null {
  const instance = instanceByUrl(wire.url);
  if (!instance) return null;
  return {
    instanceId: instance.id,
    url: wire.url,
    guildId: wire.guild_id ?? null,
    channelId: wire.channel_id,
    room: wire.room,
    e2ee: wire.e2ee === true,
  };
}

function applyStatus(s: VoiceStatusWire) {
  connState = s.state;
  muted = s.muted;
  deafened = s.deafened;
  backendAvailable = s.backend_available;
  if (s.session && s.state !== "idle") {
    const mapped = toSession(s.session);
    if (mapped) session = mapped;
  } else if (s.state === "idle") {
    session = null;
  }
  syncStatsPolling();
}

function handleEvent(p: VoiceEventPayload) {
  switch (p.type) {
    case "state": {
      if (!p.state) return;
      connState = p.state;
      if (p.state === "idle") {
        session = null;
        speaking = {};
        quality = "unknown";
        stats = null;
      } else if (p.state === "failed") {
        lastError = p.reason ?? "ERR_VOICE_DISCONNECTED";
        speaking = {};
      } else if (p.state === "connected") {
        lastError = null;
      }
      syncStatsPolling();
      return;
    }
    case "speaking": {
      if (!p.user_id) return;
      if (speaking[p.user_id] === (p.speaking === true)) return;
      speaking = { ...speaking, [p.user_id]: p.speaking === true };
      return;
    }
    case "participant_left": {
      if (p.user_id && p.user_id in speaking) {
        const next = { ...speaking };
        delete next[p.user_id];
        speaking = next;
      }
      return;
    }
    case "quality": {
      if (p.quality && p.user_id && p.user_id === localUserId()) quality = p.quality;
      return;
    }
    case "level": {
      micLevel = { rmsDb: p.rms_db ?? -100, peak: p.peak ?? 0, at: Date.now() };
      return;
    }
    case "device": {
      if (!p.kind || !p.status) return;
      handleDeviceEvent(p.kind, p.status, p.cause);
      return;
    }
    case "error": {
      const code = p.code ?? "ERR_VOICE_DISCONNECTED";
      if (code === "ERR_VOICE_MIC_LOST") micError = code;
      else if (code === "ERR_VOICE_OUTPUT_LOST") outputError = code;
      else lastError = code;
      return;
    }
    default:
      return;
  }
}

function deviceErrorCode(kind: DeviceKind, cause: DeviceLoss | undefined): string {
  if (cause === "permission_revoked") {
    return kind === "input" ? "ERR_VOICE_MIC_PERMISSION" : "ERR_VOICE_OUTPUT_PERMISSION";
  }
  if (cause === "busy") return "ERR_VOICE_DEVICE_BUSY";
  return kind === "input" ? "ERR_VOICE_MIC_LOST" : "ERR_VOICE_OUTPUT_LOST";
}

function handleDeviceEvent(kind: DeviceKind, status: DeviceStatus, cause?: DeviceLoss) {
  const translate = get(t);
  switch (status) {
    case "lost":
      if (kind === "input") micError = "ERR_VOICE_MIC_LOST";
      else outputError = "ERR_VOICE_OUTPUT_LOST";
      return;
    case "recovered":
      if (kind === "input") micError = null;
      else outputError = null;
      showToast("success", translate(kind === "input" ? "omnidisc.voice.mic_back" : "omnidisc.voice.output_back"));
      return;
    case "switched_to_default":
      if (kind === "input") micError = null;
      else outputError = null;
      showToast(
        "info",
        translate(kind === "input" ? "omnidisc.voice.mic_switched" : "omnidisc.voice.output_switched"),
      );
      return;
    case "listen_only":
      micError = deviceErrorCode("input", cause);
      showToast("error", `${translate("omnidisc.voice.listen_only")} ${translateBackendError(micError, translate)}`);
      return;
    case "silent":
      outputError = deviceErrorCode("output", cause);
      showToast("error", `${translate("omnidisc.voice.no_output")} ${translateBackendError(outputError, translate)}`);
      return;
    default:
      return;
  }
}

function syncRingClock() {
  if (incoming && !ringTimer) {
    ringNow = Date.now();
    ringTimer = setInterval(() => {
      ringNow = Date.now();
      if (incoming && ringNow >= incoming.ringingUntil) dismissIncomingCall();
    }, 1_000);
  } else if (!incoming && ringTimer) {
    clearInterval(ringTimer);
    ringTimer = null;
  }
}

function handleDispatch(p: DispatchPayload) {
  if (p.t !== "CALL_RING" || typeof p.d !== "object" || p.d === null) return;
  const d = p.d as Record<string, unknown>;
  const channelId = typeof d.channel_id === "string" ? d.channel_id : null;
  const fromUserId = typeof d.from_user_id === "string" ? d.from_user_id : null;
  if (!channelId || !fromUserId) return;
  const instance = instanceByUrl(p.url);
  if (!instance) return;
  if (session?.channelId === channelId && connState !== "idle") return;
  incoming = {
    instanceId: instance.id,
    url: p.url,
    channelId,
    fromUserId,
    ringingUntil: Date.now() + RING_TIMEOUT_MS,
  };
  syncRingClock();
}

export function getIncomingCall(): IncomingCall | null {
  return incoming;
}

export function ringSecondsLeft(): number {
  if (!incoming) return 0;
  return Math.max(0, Math.ceil((incoming.ringingUntil - ringNow) / 1000));
}

export function dismissIncomingCall() {
  incoming = null;
  syncRingClock();
}

export async function acceptIncomingCall(): Promise<boolean> {
  const call = incoming;
  if (!call) return false;
  dismissIncomingCall();
  return joinVoice(call.channelId);
}

function syncStatsPolling() {
  const want = connState === "connected" || connState === "reconnecting";
  if (want && !statsTimer) {
    statsTimer = setInterval(() => {
      if (typeof document !== "undefined" && document.visibilityState === "hidden") return;
      void refreshStats();
    }, STATS_INTERVAL_MS);
    void refreshStats();
  } else if (!want && statsTimer) {
    clearInterval(statsTimer);
    statsTimer = null;
    stats = null;
  }
}

async function refreshStats() {
  try {
    stats = await invoke<VoiceStatsWire>("omnidisc_voice_stats");
  } catch {
    stats = null;
  }
}

export async function initVoice(): Promise<void> {
  loadVolumes();
  if (initialized) return;
  initialized = true;
  try {
    unlisten = await listen<VoiceEventPayload>("omnidisc://voice", (event) => {
      try {
        handleEvent(event.payload);
      } catch (e) {
        console.warn("[omnidisc] voice event failed", errorText(e));
      }
    });
  } catch (e) {
    initialized = false;
    console.warn("[omnidisc] could not subscribe to voice events", errorText(e));
    return;
  }
  try {
    unlistenDispatch = await listen<DispatchPayload>("omnidisc://dispatch", (event) => {
      try {
        handleDispatch(event.payload);
      } catch (e) {
        console.warn("[omnidisc] call ring failed", errorText(e));
      }
    });
  } catch (e) {
    console.warn("[omnidisc] could not subscribe to call rings", errorText(e));
  }
  try {
    applyStatus(await invoke<VoiceStatusWire>("omnidisc_voice_status"));
  } catch {
    return;
  }
}

export function teardownVoice() {
  if (unlisten) unlisten();
  unlisten = null;
  if (unlistenDispatch) unlistenDispatch();
  unlistenDispatch = null;
  initialized = false;
  if (statsTimer) clearInterval(statsTimer);
  statsTimer = null;
  if (ringTimer) clearInterval(ringTimer);
  ringTimer = null;
  incoming = null;
}

export async function joinVoice(channelId: string): Promise<boolean> {
  const channel = getChannel(channelId);
  const guild = getGuild(channel?.guildId ?? null);
  const instance = getInstance(guild?.instanceId ?? getChannelInstanceId(channelId));
  if (!channel || !instance) {
    lastError = "ERR_VOICE_NOT_CONNECTED";
    return false;
  }
  if (instance.status !== "connected") {
    lastError = "ERR_NOT_CONNECTED";
    return false;
  }
  busy = true;
  lastError = null;
  micError = null;
  outputError = null;
  connState = "connecting";
  session = {
    instanceId: instance.id,
    url: instance.url,
    guildId: guild?.id ?? null,
    channelId,
    room: "",
    e2ee: false,
  };
  try {
    const result = await invoke<JoinResultWire>("omnidisc_voice_join", {
      url: instance.url,
      guildId: guild?.id ?? null,
      channelId,
      recipientIds: isEncryptedChannel(channelId) ? (channel.recipientIds ?? null) : null,
    });
    connState = result.state;
    session = toSession(result.session) ?? session;
    micError = result.mic_error ?? null;
    outputError = result.output_error ?? null;
    syncStatsPolling();
    return true;
  } catch (e) {
    lastError = errorText(e);
    connState = "failed";
    syncStatsPolling();
    return false;
  } finally {
    busy = false;
  }
}

export async function leaveVoice(): Promise<void> {
  busy = true;
  try {
    await invoke("omnidisc_voice_leave");
  } catch (e) {
    console.warn("[omnidisc] voice leave failed", errorText(e));
  } finally {
    busy = false;
    session = null;
    connState = "idle";
    speaking = {};
    quality = "unknown";
    lastError = null;
    micError = null;
    outputError = null;
    syncStatsPolling();
  }
}

export async function retryVoice(): Promise<boolean> {
  const target = session?.channelId;
  if (!target) return false;
  return joinVoice(target);
}

export async function toggleMute(): Promise<void> {
  const next = !muted;
  muted = next;
  try {
    applyStatus(await invoke<VoiceStatusWire>("omnidisc_voice_set_mute", { muted: next }));
  } catch (e) {
    muted = !next;
    console.warn("[omnidisc] mute failed", errorText(e));
  }
}

export async function toggleDeafen(): Promise<void> {
  const next = !deafened;
  deafened = next;
  try {
    applyStatus(await invoke<VoiceStatusWire>("omnidisc_voice_set_deaf", { deafened: next }));
  } catch (e) {
    deafened = !next;
    console.warn("[omnidisc] deafen failed", errorText(e));
  }
}

export function setVolume(userId: string, gain: number) {
  loadVolumes();
  const clamped = Math.min(2, Math.max(0, Number.isFinite(gain) ? gain : 1));
  if (clamped === 1) {
    const next = { ...volumes };
    delete next[userId];
    volumes = next;
  } else {
    volumes = { ...volumes, [userId]: clamped };
  }
  persistVolumes();
  invoke("omnidisc_voice_set_volume", { userId, gain: clamped }).catch((e: unknown) => {
    console.warn("[omnidisc] volume failed", errorText(e));
  });
}

export async function refreshDevices(): Promise<void> {
  devicesLoading = true;
  try {
    devices = await invoke<AudioDevices>("omnidisc_voice_devices");
  } catch (e) {
    console.warn("[omnidisc] device list failed", errorText(e));
  } finally {
    devicesLoading = false;
  }
}

export async function setDevice(kind: DeviceKind, id: string | null): Promise<string | null> {
  const key = kind === "input" ? "input_device" : "output_device";
  try {
    await invoke("omnidisc_voice_set_device", { kind, id });
  } catch (e) {
    return errorText(e);
  }
  try {
    await updateSettings({ omnidisc: { voice: { [key]: id } } });
  } catch (e) {
    console.warn("[omnidisc] device preference not saved", errorText(e));
  }
  return null;
}

export async function setNoiseSuppression(enabled: boolean): Promise<void> {
  try {
    await invoke("omnidisc_voice_set_noise_suppression", { enabled });
  } catch (e) {
    console.warn("[omnidisc] noise suppression failed", errorText(e));
  }
  await updateSettings({ omnidisc: { voice: { noise_suppression: enabled } } });
}

export async function setPttKey(key: string): Promise<void> {
  await updateSettings({ omnidisc: { voice: { ptt_key: key } } });
  try {
    await invoke("omnidisc_voice_ptt", { pressed: false });
  } catch {
    return;
  }
}

export async function setVadThreshold(db: number): Promise<void> {
  await updateSettings({ omnidisc: { voice: { vad_threshold_db: db } } });
}

export async function setDucking(percent: number): Promise<void> {
  const clamped = Math.min(100, Math.max(0, Math.round(percent)));
  try {
    await invoke("omnidisc_voice_set_ducking", { percent: clamped });
  } catch (e) {
    console.warn("[omnidisc] ducking failed", errorText(e));
  }
  await updateSettings({ omnidisc: { voice: { ducking_percent: clamped } } });
}

export async function setRelayOnly(enabled: boolean): Promise<void> {
  await updateSettings({ omnidisc: { voice: { relay_only: enabled } } });
}

export function isE2ee(): boolean {
  return session?.e2ee === true;
}

export function pushToTalk(pressed: boolean) {
  invoke("omnidisc_voice_ptt", { pressed }).catch(() => undefined);
}

export async function setMicTest(on: boolean): Promise<string | null> {
  try {
    await invoke("omnidisc_voice_mic_test", { enabled: on });
    micTesting = on;
    if (on) micLevel = { rmsDb: -100, peak: 0, at: Date.now() };
    return null;
  } catch (e) {
    micTesting = false;
    return errorText(e);
  }
}
