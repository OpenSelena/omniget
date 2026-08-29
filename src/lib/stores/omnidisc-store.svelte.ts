import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AuthMode,
  NotificationLevel,
  OmnidiscAuditEntry,
  OmnidiscBan,
  OmnidiscRelationship,
  OmnidiscRole,
  OmnidiscSession,
  StreamingPolicy,
  ConnectResult,
  ConnectStep,
  GatewayDispatchEvent,
  GatewayStatusEvent,
  InstanceStatus,
  OmnidiscChannel,
  OmnidiscGuild,
  OmnidiscInstance,
  OmnidiscMember,
  OmnidiscMessage,
  OmnidiscUser,
  ChannelKind,
  OmnidiscAttachment,
  OmnidiscDevice,
  OmnidiscGroupStatus,
  PendingAttachment,
} from "$lib/omnidisc/types";
import {
  DEMO_INSTANCE_ID,
  DEMO_INSTANCE_URL,
  makeFixtureGuilds,
  makeFixtureMembers,
  makeFixtureMessages,
} from "$lib/omnidisc/fixtures";
import {
  channelKindCode,
  emojiKey,
  isRecord,
  parseAuditEntry,
  parseBan,
  parseChannel,
  parseDevice,
  parseGuild,
  parseMessage,
  parsePresence,
  parseReadState,
  parseRelationship,
  parseRole,
  parseSession,
  parseUser,
  str,
  withCategories,
} from "$lib/omnidisc/wire";
import { canIn, resolvePermissions, type PermissionName } from "$lib/omnidisc/permissions";
import { compareSnowflakes, isAfter } from "$lib/omnidisc/snowflake";

const STORAGE_KEY = "omnidisc.instances";
const PAGE_SIZE = 50;
const TYPING_TTL_MS = 5_000;
const TYPING_THROTTLE_MS = 8_000;
const READY_TIMEOUT_MS = 20_000;
const ERR_NO_SESSION = "ERR_NO_SESSION";
const ERR_UNAUTHORIZED = "ERR_UNAUTHORIZED";

type PersistedInstance = Pick<OmnidiscInstance, "id" | "url" | "name"> & { insecure?: boolean };

interface ReadState {
  lastReadId?: string;
  mentionCount: number;
}

export interface VoiceMemberState {
  userId: string;
  channelId: string;
  guildId?: string;
  selfMute: boolean;
  selfDeaf: boolean;
  serverMute: boolean;
  serverDeaf: boolean;
  streaming: boolean;
}

interface PendingConnect {
  url: string;
  name: string;
  invite?: string;
  registrationOpen: boolean;
  streaming?: StreamingPolicy;
  insecure: boolean;
}

/// Same rule the backend applies in `omnidisc_connect`: plain http is only
/// harmless on loopback, where the traffic never reaches a network.
export function isInsecureInstanceUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "http:") return false;
    return !["localhost", "127.0.0.1", "[::1]", "::1"].includes(parsed.hostname);
  } catch {
    return true;
  }
}

let instances = $state<OmnidiscInstance[]>([]);
let loaded = false;
let initialized = false;

let selectedInstanceId = $state<string | null>(null);
let selectedGuildId = $state<string | null>(null);
let selectedChannelId = $state<string | null>(null);
let memberListOpen = $state(true);

let drafts = $state<Record<string, string>>({});
let guildsByInstance = $state<Record<string, OmnidiscGuild[]>>({});
let dmsByInstance = $state<Record<string, OmnidiscChannel[]>>({});
let usersByInstance = $state<Record<string, Record<string, OmnidiscUser>>>({});
let presenceByInstance = $state<Record<string, Record<string, string>>>({});
let membersByGuild = $state<Record<string, OmnidiscMember[]>>({});
let messagesByChannel = $state<Record<string, OmnidiscMessage[]>>({});
let readStateByChannel = $state<Record<string, ReadState>>({});
let lastMessageIdByChannel = $state<Record<string, string>>({});
let typingByChannel = $state<Record<string, Record<string, number>>>({});
let voiceStatesByChannel = $state<Record<string, Record<string, VoiceMemberState>>>({});
let voiceChannelByUser: Record<string, string> = {};
let loadingByChannel = $state<Record<string, boolean>>({});
let hasMoreByChannel: Record<string, boolean> = {};
let loadedChannels = new Set<string>();
let oldestSeqByChannel: Record<string, number> = {};
let channelInstance: Record<string, string> = {};
let channelGuild: Record<string, string> = {};
let userFetchInFlight = new Set<string>();
let typingSentAt: Record<string, number> = {};
let typingTimers: Record<string, ReturnType<typeof setTimeout>> = {};
let ackedByChannel: Record<string, string> = {};
let readyWaiters: Record<string, { resolve: () => void; reject: (e: string) => void }[]> = {};

let relationshipsByInstance = $state<Record<string, OmnidiscRelationship[]>>({});
let notesByInstance = $state<Record<string, Record<string, string>>>({});
let pinsByChannel = $state<Record<string, OmnidiscMessage[]>>({});
let ownPresenceByInstance = $state<Record<string, string>>({});
let notificationLevels = $state<Record<string, NotificationLevel>>({});
let localUnread = $state<Record<string, string>>({});
let notifyPrefsLoaded = false;

let connectStep = $state<ConnectStep>("idle");
let connectError = $state<string | null>(null);
let connectedInstance = $state<OmnidiscInstance | null>(null);
let pendingConnect = $state<PendingConnect | null>(null);

function hasStorage(): boolean {
  return typeof localStorage !== "undefined";
}

function persist() {
  if (!hasStorage()) return;
  const slim: PersistedInstance[] = instances
    .filter((i) => i.id !== DEMO_INSTANCE_ID)
    .map(({ id, url, name, insecure }) => ({ id, url, name, insecure }));
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(slim));
  } catch {
    return;
  }
}

function ensureLoaded() {
  if (loaded) return;
  loaded = true;
  if (!hasStorage()) return;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return;
    instances = parsed
      .filter(
        (p): p is PersistedInstance =>
          typeof p === "object" && p !== null && typeof p.id === "string" && typeof p.url === "string" && typeof p.name === "string",
      )
      .map((p) => ({
        id: p.id,
        url: p.url,
        name: p.name,
        status: "disconnected" as InstanceStatus,
        insecure: p.insecure ?? isInsecureInstanceUrl(p.url),
      }));
    if (instances.length > 0 && !selectedInstanceId) {
      selectedInstanceId = instances[0].id;
    }
  } catch {
    instances = [];
  }
}

function makeId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}

function isDemo(instanceId: string | null | undefined): boolean {
  return import.meta.env.DEV && instanceId === DEMO_INSTANCE_ID;
}

function instanceByUrl(url: string): OmnidiscInstance | null {
  return instances.find((i) => i.url === url) ?? null;
}

function instanceForChannel(channelId: string): OmnidiscInstance | null {
  const id = channelInstance[channelId];
  if (!id) return null;
  return instances.find((i) => i.id === id) ?? null;
}

function seedDemoData(instanceId: string) {
  const guilds = makeFixtureGuilds(instanceId);
  guildsByInstance = { ...guildsByInstance, [instanceId]: guilds };
  const members = makeFixtureMembers();
  const nextMembers = { ...membersByGuild };
  const nextMessages = { ...messagesByChannel };
  const now = Date.now();
  for (const guild of guilds) {
    nextMembers[guild.id] = members;
    for (const channel of guild.channels) {
      channelInstance[channel.id] = instanceId;
      channelGuild[channel.id] = guild.id;
      if (channel.kind !== "text") continue;
      const count = channel.id === "c-general" ? 200 : 40;
      nextMessages[channel.id] = makeFixtureMessages(channel.id, count, now, 1000);
      oldestSeqByChannel[channel.id] = 1000;
      loadedChannels.add(channel.id);
    }
  }
  membersByGuild = nextMembers;
  messagesByChannel = nextMessages;
}

export function getInstances(): OmnidiscInstance[] {
  ensureLoaded();
  return instances;
}

export function hasInstances(): boolean {
  ensureLoaded();
  return instances.length > 0;
}

export function getSelectedInstance(): OmnidiscInstance | null {
  ensureLoaded();
  return instances.find((i) => i.id === selectedInstanceId) ?? null;
}

export function getInstanceStreamingPolicy(id: string | null): StreamingPolicy | null {
  return getInstance(id)?.streaming ?? null;
}

export function getInstance(id: string | null): OmnidiscInstance | null {
  if (!id) return null;
  return instances.find((i) => i.id === id) ?? null;
}

export function selectInstance(id: string | null) {
  ensureLoaded();
  selectedInstanceId = id;
  selectedGuildId = null;
  selectedChannelId = null;
}

export function addInstance(input: {
  url: string;
  name: string;
  status?: InstanceStatus;
  id?: string;
  streaming?: StreamingPolicy;
  insecure?: boolean;
}): OmnidiscInstance {
  ensureLoaded();
  const insecure = input.insecure ?? isInsecureInstanceUrl(input.url);
  const existing = instances.find((i) => i.url === input.url);
  if (existing) {
    existing.name = input.name || existing.name;
    existing.status = input.status ?? existing.status;
    if (input.streaming) existing.streaming = input.streaming;
    existing.insecure = insecure;
    persist();
    return existing;
  }
  const instance: OmnidiscInstance = {
    id: input.id ?? makeId(),
    url: input.url,
    name: input.name || input.url,
    status: input.status ?? "disconnected",
    streaming: input.streaming,
    insecure,
  };
  instances = [...instances, instance];
  if (!selectedInstanceId) selectedInstanceId = instance.id;
  persist();
  return instance;
}

function clearInstanceData(instanceId: string) {
  const guilds = guildsByInstance[instanceId] ?? [];
  const nextMessages = { ...messagesByChannel };
  const nextMembers = { ...membersByGuild };
  for (const guild of guilds) {
    delete nextMembers[guild.id];
    for (const channel of guild.channels) {
      delete nextMessages[channel.id];
      delete channelInstance[channel.id];
      delete channelGuild[channel.id];
      loadedChannels.delete(channel.id);
    }
  }
  for (const dm of dmsByInstance[instanceId] ?? []) {
    delete nextMessages[dm.id];
    delete channelInstance[dm.id];
    loadedChannels.delete(dm.id);
  }
  messagesByChannel = nextMessages;
  membersByGuild = nextMembers;
  const nextGuilds = { ...guildsByInstance };
  delete nextGuilds[instanceId];
  guildsByInstance = nextGuilds;
  const nextDms = { ...dmsByInstance };
  delete nextDms[instanceId];
  dmsByInstance = nextDms;
  const nextUsers = { ...usersByInstance };
  delete nextUsers[instanceId];
  usersByInstance = nextUsers;
  const nextPresence = { ...presenceByInstance };
  delete nextPresence[instanceId];
  presenceByInstance = nextPresence;
  clearVoiceStates(instanceId);
}

export function removeInstance(id: string) {
  ensureLoaded();
  const instance = instances.find((i) => i.id === id);
  clearInstanceData(id);
  instances = instances.filter((i) => i.id !== id);
  if (selectedInstanceId === id) {
    selectedInstanceId = instances[0]?.id ?? null;
    selectedGuildId = null;
    selectedChannelId = null;
  }
  persist();
  if (instance && !isDemo(id)) {
    void forgetRemote(instance.url);
  }
}

async function forgetRemote(url: string) {
  try {
    await invoke("omnidisc_gateway_disconnect", { url });
    await invoke("omnidisc_logout", { url });
  } catch (e) {
    console.warn("[omnidisc] could not sign out of", url, errorText(e));
  }
}

export async function signOut(id: string) {
  const instance = instances.find((i) => i.id === id);
  if (!instance || isDemo(id)) return;
  try {
    await invoke("omnidisc_logout", { url: instance.url });
  } catch {
    // token is dropped locally even if the server did not answer
  }
  clearInstanceData(id);
  setInstanceStatus(id, "signed_out");
}

export function setInstanceStatus(id: string, status: InstanceStatus, error?: string) {
  const instance = instances.find((i) => i.id === id);
  if (!instance) return;
  instance.status = status;
  instance.error = status === "error" || status === "signed_out" ? error : undefined;
}

export function getGuilds(instanceId: string | null): OmnidiscGuild[] {
  if (!instanceId) return [];
  return guildsByInstance[instanceId] ?? [];
}

export function getGuild(guildId: string | null): OmnidiscGuild | null {
  if (!guildId) return null;
  for (const list of Object.values(guildsByInstance)) {
    const found = list.find((g) => g.id === guildId);
    if (found) return found;
  }
  return null;
}

export function getDms(instanceId: string | null): OmnidiscChannel[] {
  if (!instanceId) return [];
  return dmsByInstance[instanceId] ?? [];
}

export function getChannel(channelId: string | null): OmnidiscChannel | null {
  if (!channelId) return null;
  const guildId = channelGuild[channelId];
  if (guildId) {
    return getGuild(guildId)?.channels.find((c) => c.id === channelId) ?? null;
  }
  const instanceId = channelInstance[channelId];
  if (!instanceId) return null;
  return (dmsByInstance[instanceId] ?? []).find((c) => c.id === channelId) ?? null;
}

export function getUser(instanceId: string | null, userId: string | null): OmnidiscUser | null {
  if (!instanceId || !userId) return null;
  return usersByInstance[instanceId]?.[userId] ?? null;
}

export function getPresence(instanceId: string | null, userId: string | null): string {
  if (!instanceId || !userId) return "offline";
  return presenceByInstance[instanceId]?.[userId] ?? "offline";
}

export function getMe(instanceId: string | null): OmnidiscUser | null {
  const instance = getInstance(instanceId);
  if (!instance?.userId) return null;
  return getUser(instance.id, instance.userId);
}

export function getChannelInstanceId(channelId: string | null): string | null {
  if (!channelId) return null;
  const guildId = channelGuild[channelId];
  if (guildId) return getGuild(guildId)?.instanceId ?? null;
  return channelInstance[channelId] ?? null;
}

export function userName(instanceId: string, userId: string): string {
  const user = usersByInstance[instanceId]?.[userId];
  if (user) return user.displayName;
  if (!isDemo(instanceId)) void ensureUser(instanceId, userId);
  return "…";
}

export function dmTitle(channel: OmnidiscChannel): string {
  const instanceId = channelInstance[channel.id];
  const instance = getInstance(instanceId ?? null);
  if (!instanceId || !instance) return channel.name || channel.id;
  if (channel.name) return channel.name;
  const others = (channel.recipientIds ?? []).filter((r) => r !== instance.userId);
  if (others.length === 0) return userName(instanceId, instance.userId ?? "");
  return others.map((r) => userName(instanceId, r)).join(", ");
}

async function ensureUser(instanceId: string, userId: string) {
  const key = `${instanceId}:${userId}`;
  if (userFetchInFlight.has(key)) return;
  const instance = getInstance(instanceId);
  if (!instance) return;
  userFetchInFlight.add(key);
  try {
    const raw = await invoke<unknown>("omnidisc_get_user", { url: instance.url, userId });
    const user = parseUser(raw);
    if (user) upsertUser(instanceId, user);
  } catch {
    // name stays as a placeholder; the next render retries when the user reappears
  } finally {
    userFetchInFlight.delete(key);
  }
}

function upsertUser(instanceId: string, user: OmnidiscUser) {
  const current = usersByInstance[instanceId] ?? {};
  usersByInstance = { ...usersByInstance, [instanceId]: { ...current, [user.id]: user } };
}

function setPresence(instanceId: string, userId: string, status: string) {
  const current = presenceByInstance[instanceId] ?? {};
  presenceByInstance = { ...presenceByInstance, [instanceId]: { ...current, [userId]: status } };
}

export function getMembers(guildId: string | null): OmnidiscMember[] {
  if (!guildId) return [];
  const guild = getGuild(guildId);
  const base = membersByGuild[guildId] ?? [];
  if (!guild || isDemo(guild.instanceId)) return base;
  const presence = presenceByInstance[guild.instanceId] ?? {};
  const users = usersByInstance[guild.instanceId] ?? {};
  return base.map((m) => ({
    ...m,
    name: users[m.id]?.displayName ?? m.name,
    online: m.id === getInstance(guild.instanceId)?.userId ? true : (presence[m.id] ?? "offline") !== "offline",
  }));
}

export function getMessages(channelId: string | null): OmnidiscMessage[] {
  if (!channelId) return [];
  const list = messagesByChannel[channelId] ?? [];
  const instanceId = channelInstance[channelId];
  if (!instanceId || isDemo(instanceId)) return list;
  const users = usersByInstance[instanceId] ?? {};
  return list.map((m) => {
    const user = users[m.authorId];
    if (!user) return m.authorName === "…" ? m : { ...m, authorName: m.authorName };
    return user.displayName === m.authorName ? m : { ...m, authorName: user.displayName };
  });
}

export function isChannelLoading(channelId: string | null): boolean {
  return channelId ? (loadingByChannel[channelId] ?? false) : false;
}

export function getTypingNames(channelId: string | null): string[] {
  if (!channelId) return [];
  const instanceId = channelInstance[channelId];
  const instance = getInstance(instanceId ?? null);
  if (!instanceId || !instance) return [];
  const now = Date.now();
  return Object.entries(typingByChannel[channelId] ?? {})
    .filter(([userId, until]) => until > now && userId !== instance.userId)
    .map(([userId]) => userName(instanceId, userId));
}

export function getSelectedGuildId(): string | null {
  return selectedGuildId;
}

export function getSelectedChannelId(): string | null {
  return selectedChannelId;
}

export function selectChannel(guildId: string | null, channelId: string | null) {
  ensureLoaded();
  if (selectedChannelId && selectedChannelId !== channelId) {
    delete suppressAck[selectedChannelId];
    clearUnreadMark(selectedChannelId);
  }
  const guild = getGuild(guildId);
  if (guild && guild.instanceId !== selectedInstanceId) {
    selectedInstanceId = guild.instanceId;
  }
  if (!guild && channelId) {
    const instanceId = channelInstance[channelId];
    if (instanceId && instanceId !== selectedInstanceId) selectedInstanceId = instanceId;
  }
  selectedGuildId = guildId;
  selectedChannelId = channelId;
  if (channelId) {
    void ensureChannelLoaded(channelId);
    void ackChannel(channelId);
    if (guildId) void requestMembers(guildId, channelId);
  }
}

export function isMemberListOpen(): boolean {
  return memberListOpen;
}

export function toggleMemberList() {
  memberListOpen = !memberListOpen;
}

export function getDraft(key: string): string {
  return drafts[key] ?? "";
}

export function setDraft(key: string, text: string) {
  if (text.length === 0) {
    if (key in drafts) {
      const next = { ...drafts };
      delete next[key];
      drafts = next;
    }
    return;
  }
  drafts = { ...drafts, [key]: text };
}

export function isUnread(channelId: string): boolean {
  const last = lastMessageIdByChannel[channelId];
  if (!last) return false;
  return isAfter(last, readStateByChannel[channelId]?.lastReadId);
}

export function getMentionCount(channelId: string): number {
  return readStateByChannel[channelId]?.mentionCount ?? 0;
}

export function getUnreadCount(): number {
  let count = 0;
  for (const channelId of Object.keys(lastMessageIdByChannel)) {
    if (channelId === selectedChannelId) continue;
    const channel = getChannel(channelId);
    if (!channel || channel.kind === "voice" || channel.kind === "category") continue;
    if (isUnread(channelId)) count += 1;
  }
  return count;
}

export function getVoiceMembers(channelId: string | null): VoiceMemberState[] {
  if (!channelId) return [];
  return Object.values(voiceStatesByChannel[channelId] ?? {});
}

export function getVoiceMemberCount(channelId: string): number {
  return Object.keys(voiceStatesByChannel[channelId] ?? {}).length;
}

export function getVoiceChannelOfUser(instanceId: string | null, userId: string | null): string | null {
  if (!instanceId || !userId) return null;
  return voiceChannelByUser[`${instanceId}:${userId}`] ?? null;
}

export function applyVoiceState(instanceId: string, raw: unknown) {
  if (!isRecord(raw)) return;
  const userId = str(raw.user_id);
  if (!userId) return;
  const key = `${instanceId}:${userId}`;
  const channelId = str(raw.channel_id);
  const previous = voiceChannelByUser[key];
  const next = { ...voiceStatesByChannel };
  if (previous && previous !== channelId && next[previous]) {
    const rest = { ...next[previous] };
    delete rest[userId];
    if (Object.keys(rest).length === 0) delete next[previous];
    else next[previous] = rest;
  }
  if (channelId) {
    voiceChannelByUser[key] = channelId;
    next[channelId] = {
      ...(next[channelId] ?? {}),
      [userId]: {
        userId,
        channelId,
        guildId: str(raw.guild_id),
        selfMute: raw.self_mute === true,
        selfDeaf: raw.self_deaf === true,
        serverMute: raw.server_mute === true,
        serverDeaf: raw.server_deaf === true,
        streaming: raw.streaming === true,
      },
    };
    if (!usersByInstance[instanceId]?.[userId] && !isDemo(instanceId)) void ensureUser(instanceId, userId);
  } else {
    delete voiceChannelByUser[key];
  }
  voiceStatesByChannel = next;
}

function clearVoiceStates(instanceId: string) {
  const prefix = `${instanceId}:`;
  const gone = new Set<string>();
  for (const key of Object.keys(voiceChannelByUser)) {
    if (key.startsWith(prefix)) {
      gone.add(key.slice(prefix.length));
      delete voiceChannelByUser[key];
    }
  }
  if (gone.size === 0) return;
  const next: Record<string, Record<string, VoiceMemberState>> = {};
  for (const [channelId, members] of Object.entries(voiceStatesByChannel)) {
    const kept = Object.fromEntries(Object.entries(members).filter(([uid]) => !gone.has(uid)));
    if (Object.keys(kept).length > 0) next[channelId] = kept;
  }
  voiceStatesByChannel = next;
}

function sortMessages(list: OmnidiscMessage[]): OmnidiscMessage[] {
  return [...list].sort((a, b) => {
    if (a.createdAt !== b.createdAt) return a.createdAt - b.createdAt;
    if (a.delivery !== "sent" && b.delivery === "sent") return 1;
    if (b.delivery !== "sent" && a.delivery === "sent") return -1;
    return compareSnowflakes(a.id, b.id);
  });
}

function upsertMessage(message: OmnidiscMessage) {
  const current = messagesByChannel[message.channelId] ?? [];
  const idx = current.findIndex((m) => m.id === message.id);
  const next = idx >= 0 ? current.map((m, i) => (i === idx ? message : m)) : sortMessages([...current, message]);
  messagesByChannel = { ...messagesByChannel, [message.channelId]: next };
}

function removeMessage(channelId: string, messageId: string) {
  const current = messagesByChannel[channelId];
  if (!current) return;
  messagesByChannel = { ...messagesByChannel, [channelId]: current.filter((m) => m.id !== messageId) };
}

function bumpLastMessage(channelId: string, messageId: string) {
  if (isAfter(messageId, lastMessageIdByChannel[channelId])) {
    lastMessageIdByChannel = { ...lastMessageIdByChannel, [channelId]: messageId };
  }
}

function setReadState(channelId: string, state: ReadState) {
  readStateByChannel = { ...readStateByChannel, [channelId]: state };
}

function nameResolver(instanceId: string): (authorId: string) => string {
  const users = usersByInstance[instanceId] ?? {};
  return (authorId) => users[authorId]?.displayName ?? "…";
}

export async function ensureChannelLoaded(channelId: string) {
  if (loadedChannels.has(channelId)) return;
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  loadedChannels.add(channelId);
  loadingByChannel = { ...loadingByChannel, [channelId]: true };
  try {
    const raw = await invoke<unknown>("omnidisc_list_messages", { url: instance.url, channelId, limit: PAGE_SIZE });
    const list = Array.isArray(raw) ? raw : [];
    const resolve = nameResolver(instance.id);
    const parsed = list
      .map((m) => parseMessage(m, resolve))
      .filter((m): m is OmnidiscMessage => m !== null);
    hasMoreByChannel[channelId] = list.length >= PAGE_SIZE;
    const pending = (messagesByChannel[channelId] ?? []).filter((m) => m.delivery !== "sent");
    messagesByChannel = { ...messagesByChannel, [channelId]: sortMessages([...parsed, ...pending]) };
    for (const m of parsed) {
      bumpLastMessage(channelId, m.id);
      if (!usersByInstance[instance.id]?.[m.authorId]) void ensureUser(instance.id, m.authorId);
    }
    void hydrateEncrypted(channelId);
    void refreshGroupStatus(channelId);
  } catch (e) {
    loadedChannels.delete(channelId);
    console.warn("[omnidisc] could not load messages", errorText(e));
  } finally {
    const next = { ...loadingByChannel };
    delete next[channelId];
    loadingByChannel = next;
  }
}

export async function loadOlderMessages(channelId: string, count = PAGE_SIZE): Promise<number> {
  const current = messagesByChannel[channelId] ?? [];
  const instance = instanceForChannel(channelId);
  if (instance && isDemo(instance.id)) {
    const oldestSeq = oldestSeqByChannel[channelId];
    if (oldestSeq === undefined || oldestSeq <= 0) return 0;
    const endAt = (current[0]?.createdAt ?? Date.now()) - 60_000;
    const startSeq = Math.max(0, oldestSeq - count);
    const older = makeFixtureMessages(channelId, oldestSeq - startSeq, endAt, startSeq);
    oldestSeqByChannel[channelId] = startSeq;
    messagesByChannel = { ...messagesByChannel, [channelId]: [...older, ...current] };
    return older.length;
  }
  if (!instance || hasMoreByChannel[channelId] === false || loadingByChannel[channelId]) return 0;
  const oldest = current.find((m) => m.delivery === "sent");
  if (!oldest) return 0;
  loadingByChannel = { ...loadingByChannel, [channelId]: true };
  try {
    const raw = await invoke<unknown>("omnidisc_list_messages", {
      url: instance.url,
      channelId,
      before: oldest.id,
      limit: count,
    });
    const list = Array.isArray(raw) ? raw : [];
    const resolve = nameResolver(instance.id);
    const parsed = list.map((m) => parseMessage(m, resolve)).filter((m): m is OmnidiscMessage => m !== null);
    hasMoreByChannel[channelId] = list.length >= count;
    const known = new Set((messagesByChannel[channelId] ?? []).map((m) => m.id));
    const fresh = parsed.filter((m) => !known.has(m.id));
    messagesByChannel = {
      ...messagesByChannel,
      [channelId]: sortMessages([...fresh, ...(messagesByChannel[channelId] ?? [])]),
    };
    for (const m of fresh) {
      if (!usersByInstance[instance.id]?.[m.authorId]) void ensureUser(instance.id, m.authorId);
    }
    void hydrateEncrypted(channelId);
    return fresh.length;
  } catch (e) {
    console.warn("[omnidisc] could not load older messages", errorText(e));
    return 0;
  } finally {
    const next = { ...loadingByChannel };
    delete next[channelId];
    loadingByChannel = next;
  }
}

export function hasUploadsInFlight(channelId: string): boolean {
  return (uploadsByChannel[channelId] ?? []).some((u) => u.state !== "done" && u.state !== "failed");
}

export async function sendMessage(channelId: string, content: string, replyTo?: string): Promise<void> {
  const instance = instanceForChannel(channelId);
  const text = content.trim();
  const ready = (uploadsByChannel[channelId] ?? []).filter((u) => u.state === "done");
  if (!text && ready.length === 0) return;
  if (hasUploadsInFlight(channelId)) return;
  if (!instance || isDemo(instance.id)) {
    if (text) appendLocalMessage(channelId, text, { id: instance?.userId ?? "me", name: "You" });
    return;
  }
  const me = instance.userId ?? "me";
  const pendingId = `pending:${makeId()}`;
  const attachments: OmnidiscAttachment[] = ready.map((u) => ({
    id: u.id,
    filename: u.name,
    size: u.total,
    contentType: u.mime,
    url: u.previewUrl,
    encrypted: u.encrypted,
  }));
  const optimistic: OmnidiscMessage = {
    id: pendingId,
    channelId,
    authorId: me,
    authorName: getMe(instance.id)?.displayName ?? "…",
    content: text,
    createdAt: Date.now(),
    delivery: "pending",
    replyToId: replyTo,
    attachments,
    encrypted: isEncryptedChannel(channelId),
  };
  const uploadIds = ready.map((u) => u.id);
  clearUploads(channelId);
  upsertMessage(optimistic);
  await deliver(instance, optimistic, uploadIds);
}

const uploadsOfPending: Record<string, string[]> = {};

async function deliver(instance: OmnidiscInstance, optimistic: OmnidiscMessage, uploadIds: string[]) {
  uploadsOfPending[optimistic.id] = uploadIds;
  const channel = getChannel(optimistic.channelId);
  const encrypted = isEncryptedChannel(optimistic.channelId);
  try {
    const raw = await invoke<unknown>("omnidisc_send_message", {
      url: instance.url,
      channelId: optimistic.channelId,
      content: optimistic.content,
      nonce: optimistic.id,
      replyTo: optimistic.replyToId ?? null,
      uploadIds,
      encrypted,
      recipientIds: (channel?.recipientIds ?? []).filter((id) => id !== instance.userId),
    });
    const sent = parseMessage(raw, nameResolver(instance.id));
    removeMessage(optimistic.channelId, optimistic.id);
    delete uploadsOfPending[optimistic.id];
    if (sent) {
      // We wrote the plaintext, so there is nothing to wait for on our own copy.
      const settled: OmnidiscMessage = sent.encrypted
        ? { ...sent, content: optimistic.content, attachments: optimistic.attachments, awaitingDecryption: false }
        : sent;
      upsertMessage(settled);
      bumpLastMessage(settled.channelId, settled.id);
      ackedByChannel[settled.channelId] = settled.id;
      setReadState(settled.channelId, { lastReadId: settled.id, mentionCount: 0 });
      if (encrypted) void refreshGroupStatus(settled.channelId);
    }
  } catch (e) {
    upsertMessage({ ...optimistic, delivery: "failed", error: errorText(e) });
  }
}

export async function retryMessage(channelId: string, messageId: string) {
  const instance = instanceForChannel(channelId);
  const current = (messagesByChannel[channelId] ?? []).find((m) => m.id === messageId);
  if (!instance || !current || current.delivery !== "failed") return;
  upsertMessage({ ...current, delivery: "pending", error: undefined });
  await deliver(instance, { ...current, delivery: "pending", error: undefined }, uploadsOfPending[messageId] ?? []);
}

export function discardMessage(channelId: string, messageId: string) {
  removeMessage(channelId, messageId);
}

export function appendLocalMessage(channelId: string, content: string, author: { id: string; name: string }) {
  const message: OmnidiscMessage = {
    id: `${channelId}-local-${makeId()}`,
    channelId,
    authorId: author.id,
    authorName: author.name,
    content,
    createdAt: Date.now(),
    delivery: "sent",
  };
  messagesByChannel = { ...messagesByChannel, [channelId]: [...(messagesByChannel[channelId] ?? []), message] };
}

export async function editMessage(channelId: string, messageId: string, content: string) {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  const raw = await invoke<unknown>("omnidisc_edit_message", { url: instance.url, channelId, messageId, content });
  const parsed = parseMessage(raw, nameResolver(instance.id));
  if (parsed) upsertMessage(parsed);
}

export async function deleteMessage(channelId: string, messageId: string) {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) {
    removeMessage(channelId, messageId);
    return;
  }
  await invoke("omnidisc_delete_message", { url: instance.url, channelId, messageId });
  removeMessage(channelId, messageId);
}

export async function toggleReaction(channelId: string, messageId: string, emoji: string) {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  const message = (messagesByChannel[channelId] ?? []).find((m) => m.id === messageId);
  const mine = message?.reactions?.find((r) => r.emoji === emoji)?.me ?? false;
  const command = mine ? "omnidisc_remove_reaction" : "omnidisc_add_reaction";
  await invoke(command, { url: instance.url, channelId, messageId, emoji: emoji.split(":")[0] });
}

export function notifyTyping(channelId: string) {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id) || instance.status !== "connected") return;
  const now = Date.now();
  if (now - (typingSentAt[channelId] ?? 0) < TYPING_THROTTLE_MS) return;
  typingSentAt[channelId] = now;
  invoke("omnidisc_typing", { url: instance.url, channelId }).catch(() => {
    typingSentAt[channelId] = 0;
  });
}

async function ackChannel(channelId: string) {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  if (suppressAck[channelId]) return;
  const last = lastMessageIdByChannel[channelId];
  if (!last) return;
  const read = readStateByChannel[channelId]?.lastReadId;
  if (!isAfter(last, read)) return;
  if (ackedByChannel[channelId] === last) return;
  ackedByChannel[channelId] = last;
  setReadState(channelId, { lastReadId: last, mentionCount: 0 });
  try {
    await invoke("omnidisc_ack", { url: instance.url, channelId, messageId: last });
  } catch {
    delete ackedByChannel[channelId];
  }
}

async function requestMembers(guildId: string, channelId: string) {
  const guild = getGuild(guildId);
  const instance = guild ? getInstance(guild.instanceId) : null;
  if (!guild || !instance || isDemo(instance.id) || instance.status !== "connected") return;
  try {
    await invoke("omnidisc_gateway_send", {
      url: instance.url,
      op: 14,
      d: { guild_id: guildId, channel_id: channelId, ranges: [[0, 99]] },
    });
  } catch {
    // member list stays as it was; nothing to show the user beyond the existing list
  }
}

function registerChannels(instanceId: string, guildId: string | null, channels: OmnidiscChannel[]) {
  for (const channel of channels) {
    channelInstance[channel.id] = instanceId;
    if (guildId) channelGuild[channel.id] = guildId;
    else delete channelGuild[channel.id];
    if (channel.lastMessageId) bumpLastMessage(channel.id, channel.lastMessageId);
  }
}

function upsertGuild(instanceId: string, guild: OmnidiscGuild) {
  const list = guildsByInstance[instanceId] ?? [];
  const idx = list.findIndex((g) => g.id === guild.id);
  const next = idx >= 0 ? list.map((g, i) => (i === idx ? guild : g)) : [...list, guild];
  guildsByInstance = { ...guildsByInstance, [instanceId]: next };
  registerChannels(instanceId, guild.id, guild.channels);
}

function removeGuild(instanceId: string, guildId: string) {
  const list = guildsByInstance[instanceId] ?? [];
  const guild = list.find((g) => g.id === guildId);
  if (guild) {
    for (const c of guild.channels) {
      delete channelInstance[c.id];
      delete channelGuild[c.id];
    }
  }
  guildsByInstance = { ...guildsByInstance, [instanceId]: list.filter((g) => g.id !== guildId) };
  const nextMembers = { ...membersByGuild };
  delete nextMembers[guildId];
  membersByGuild = nextMembers;
  if (selectedGuildId === guildId) {
    selectedGuildId = null;
    selectedChannelId = null;
  }
}

function upsertGuildChannel(instanceId: string, channel: OmnidiscChannel) {
  const guildId = channel.guildId;
  if (!guildId) return;
  const list = guildsByInstance[instanceId] ?? [];
  const guild = list.find((g) => g.id === guildId);
  if (!guild) return;
  const rest = guild.channels.filter((c) => c.id !== channel.id);
  const updated: OmnidiscGuild = { ...guild, channels: withCategories([...rest, channel]) };
  upsertGuild(instanceId, updated);
}

function removeChannel(instanceId: string, channelId: string) {
  const guildId = channelGuild[channelId];
  if (guildId) {
    const guild = (guildsByInstance[instanceId] ?? []).find((g) => g.id === guildId);
    if (guild) {
      upsertGuild(instanceId, { ...guild, channels: guild.channels.filter((c) => c.id !== channelId) });
    }
  } else {
    dmsByInstance = { ...dmsByInstance, [instanceId]: (dmsByInstance[instanceId] ?? []).filter((c) => c.id !== channelId) };
  }
  delete channelInstance[channelId];
  delete channelGuild[channelId];
  if (selectedChannelId === channelId) selectedChannelId = null;
}

function upsertDm(instanceId: string, channel: OmnidiscChannel) {
  const list = dmsByInstance[instanceId] ?? [];
  const idx = list.findIndex((c) => c.id === channel.id);
  const next = idx >= 0 ? list.map((c, i) => (i === idx ? channel : c)) : [channel, ...list];
  dmsByInstance = { ...dmsByInstance, [instanceId]: next };
  registerChannels(instanceId, null, [channel]);
  for (const r of channel.recipientIds ?? []) {
    if (!usersByInstance[instanceId]?.[r]) void ensureUser(instanceId, r);
  }
}

function applyReady(instance: OmnidiscInstance, d: unknown) {
  if (!isRecord(d)) return;
  const me = parseUser(d.user);
  if (me) {
    instance.userId = me.id;
    upsertUser(instance.id, me);
  }
  const guilds = (Array.isArray(d.guilds) ? d.guilds : [])
    .map((g) => parseGuild(g, instance.id))
    .filter((g): g is OmnidiscGuild => g !== null);
  guildsByInstance = { ...guildsByInstance, [instance.id]: guilds };
  for (const g of guilds) registerChannels(instance.id, g.id, g.channels);
  const dms = (Array.isArray(d.private_channels) ? d.private_channels : [])
    .map(parseChannel)
    .filter((c): c is OmnidiscChannel => c !== null);
  dmsByInstance = { ...dmsByInstance, [instance.id]: dms };
  registerChannels(instance.id, null, dms);
  for (const dm of dms) {
    for (const r of dm.recipientIds ?? []) {
      if (!usersByInstance[instance.id]?.[r]) void ensureUser(instance.id, r);
    }
  }
  const relationships = (Array.isArray(d.relationships) ? d.relationships : [])
    .map(parseRelationship)
    .filter((r): r is OmnidiscRelationship => r !== null);
  relationshipsByInstance = { ...relationshipsByInstance, [instance.id]: relationships };
  for (const rel of relationships) {
    if (!usersByInstance[instance.id]?.[rel.userId]) void ensureUser(instance.id, rel.userId);
  }
  for (const rs of Array.isArray(d.read_states) ? d.read_states : []) {
    const parsed = parseReadState(rs);
    if (parsed) setReadState(parsed.channelId, { lastReadId: parsed.lastReadId, mentionCount: parsed.mentionCount });
  }
  const presences: Record<string, string> = {};
  for (const p of Array.isArray(d.presences) ? d.presences : []) {
    const parsed = parsePresence(p);
    if (parsed) presences[parsed.userId] = parsed.status;
  }
  if (me) presences[me.id] = "online";
  presenceByInstance = { ...presenceByInstance, [instance.id]: presences };
  clearVoiceStates(instance.id);
  for (const vs of Array.isArray(d.voice_states) ? d.voice_states : []) applyVoiceState(instance.id, vs);
  loadedChannels = new Set([...loadedChannels].filter((id) => channelInstance[id] !== instance.id));
  if (selectedChannelId && channelInstance[selectedChannelId] === instance.id) {
    void ensureChannelLoaded(selectedChannelId);
    if (selectedGuildId) void requestMembers(selectedGuildId, selectedChannelId);
  }
}

function applyMemberList(instanceId: string, d: unknown) {
  if (!isRecord(d)) return;
  const guildId = str(d.guild_id);
  if (!guildId) return;
  const guild = getGuild(guildId);
  const members: OmnidiscMember[] = [];
  for (const op of Array.isArray(d.ops) ? d.ops : []) {
    if (!isRecord(op) || op.op !== "sync") continue;
    for (const item of Array.isArray(op.items) ? op.items : []) {
      if (!isRecord(item) || item.kind !== "member") continue;
      const user = parseUser(item.user);
      if (!user) continue;
      upsertUser(instanceId, user);
      const presence = parsePresence(item.presence);
      if (presence) setPresence(instanceId, presence.userId, presence.status);
      const memberRaw = isRecord(item.member) ? item.member : {};
      members.push({
        id: user.id,
        name: str(memberRaw.nick) ?? user.displayName,
        online: presence ? presence.status !== "offline" : false,
        role: guild?.ownerId === user.id ? "owner" : undefined,
        nick: str(memberRaw.nick),
        roleIds: Array.isArray(memberRaw.role_ids)
          ? memberRaw.role_ids.filter((r): r is string => typeof r === "string")
          : [],
        mutedUntil: str(memberRaw.muted_until),
      });
    }
  }
  membersByGuild = { ...membersByGuild, [guildId]: members };
}

function handleDispatch(instance: OmnidiscInstance, t: string, d: unknown) {
  switch (t) {
    case "READY":
      applyReady(instance, d);
      return;
    case "RESUMED":
      return;
    case "GUILD_CREATE":
    case "GUILD_UPDATE": {
      const guild = parseGuild(d, instance.id);
      if (guild) upsertGuild(instance.id, guild);
      return;
    }
    case "GUILD_DELETE": {
      if (isRecord(d) && typeof d.id === "string") removeGuild(instance.id, d.id);
      return;
    }
    case "GUILD_MEMBER_ADD":
    case "GUILD_MEMBER_UPDATE":
    case "GUILD_MEMBER_REMOVE": {
      if (!isRecord(d)) return;
      const guildId = str(d.guild_id);
      const userId = str(d.user_id);
      if (userId && t !== "GUILD_MEMBER_REMOVE" && !usersByInstance[instance.id]?.[userId]) void ensureUser(instance.id, userId);
      if (guildId && guildId === selectedGuildId && selectedChannelId) void requestMembers(guildId, selectedChannelId);
      return;
    }
    case "GUILD_MEMBER_LIST_UPDATE":
      applyMemberList(instance.id, d);
      return;
    case "GUILD_ROLE_CREATE":
    case "GUILD_ROLE_UPDATE": {
      const role = parseRole(d);
      const guildId = isRecord(d) ? str(d.guild_id) : undefined;
      const guild = getGuild(guildId ?? null);
      if (!role || !guild) return;
      const rest = guild.roles.filter((r) => r.id !== role.id);
      upsertGuild(instance.id, { ...guild, roles: [...rest, role].sort((a, b) => a.position - b.position) });
      return;
    }
    case "GUILD_ROLE_DELETE": {
      if (!isRecord(d)) return;
      const guild = getGuild(str(d.guild_id) ?? null);
      const roleId = str(d.id);
      if (!guild || !roleId) return;
      upsertGuild(instance.id, { ...guild, roles: guild.roles.filter((r) => r.id !== roleId) });
      return;
    }
    case "RELATIONSHIP_ADD": {
      const rel = parseRelationship(d);
      if (!rel) return;
      const list = (relationshipsByInstance[instance.id] ?? []).filter((r) => r.userId !== rel.userId);
      relationshipsByInstance = { ...relationshipsByInstance, [instance.id]: [...list, rel] };
      if (!usersByInstance[instance.id]?.[rel.userId]) void ensureUser(instance.id, rel.userId);
      return;
    }
    case "RELATIONSHIP_REMOVE": {
      if (!isRecord(d)) return;
      const userId = str(d.user_id);
      if (!userId) return;
      relationshipsByInstance = {
        ...relationshipsByInstance,
        [instance.id]: (relationshipsByInstance[instance.id] ?? []).filter((r) => r.userId !== userId),
      };
      return;
    }
    case "CHANNEL_PINS_UPDATE": {
      if (!isRecord(d)) return;
      const channelId = str(d.channel_id);
      if (channelId && pinsByChannel[channelId]) void loadPins(channelId);
      return;
    }
    case "CHANNEL_CREATE":
    case "CHANNEL_UPDATE": {
      const channel = parseChannel(d);
      if (!channel) return;
      if (channel.guildId) upsertGuildChannel(instance.id, channel);
      else upsertDm(instance.id, channel);
      return;
    }
    case "CHANNEL_DELETE": {
      if (isRecord(d) && typeof d.id === "string") removeChannel(instance.id, d.id);
      return;
    }
    case "MESSAGE_CREATE":
    case "MESSAGE_UPDATE": {
      const parsed = parseMessage(d, nameResolver(instance.id));
      if (!parsed) return;
      const message = fillFromPending(parsed);
      if (!usersByInstance[instance.id]?.[message.authorId]) void ensureUser(instance.id, message.authorId);
      if (t === "MESSAGE_CREATE" || (messagesByChannel[message.channelId] ?? []).some((m) => m.id === message.id)) {
        upsertMessage(message);
      }
      if (t === "MESSAGE_CREATE") {
        bumpLastMessage(message.channelId, message.id);
        const typing = typingByChannel[message.channelId];
        if (typing && message.authorId in typing) {
          const next = { ...typing };
          delete next[message.authorId];
          typingByChannel = { ...typingByChannel, [message.channelId]: next };
        }
        if (message.channelId === selectedChannelId) void ackChannel(message.channelId);
        else void maybeNotify(instance, message);
      }
      return;
    }
    case "MESSAGE_DELETE": {
      if (isRecord(d) && typeof d.id === "string" && typeof d.channel_id === "string") removeMessage(d.channel_id, d.id);
      return;
    }
    case "MESSAGE_DELETE_BULK": {
      if (!isRecord(d) || typeof d.channel_id !== "string" || !Array.isArray(d.ids)) return;
      const ids = new Set(d.ids.filter((x): x is string => typeof x === "string"));
      const current = messagesByChannel[d.channel_id];
      if (current) messagesByChannel = { ...messagesByChannel, [d.channel_id]: current.filter((m) => !ids.has(m.id)) };
      return;
    }
    case "MESSAGE_REACTION_ADD":
    case "MESSAGE_REACTION_REMOVE": {
      if (!isRecord(d)) return;
      const channelId = str(d.channel_id);
      const messageId = str(d.message_id);
      const userId = str(d.user_id);
      const emoji = emojiKey(d.emoji);
      if (!channelId || !messageId || !userId || !emoji) return;
      const current = messagesByChannel[channelId];
      const message = current?.find((m) => m.id === messageId);
      if (!message) return;
      const reactions = [...(message.reactions ?? [])];
      const idx = reactions.findIndex((r) => r.emoji === emoji);
      const delta = t === "MESSAGE_REACTION_ADD" ? 1 : -1;
      const mine = userId === instance.userId;
      if (idx >= 0) {
        const r = reactions[idx];
        const count = Math.max(0, r.count + delta);
        if (count === 0) reactions.splice(idx, 1);
        else reactions[idx] = { ...r, count, me: mine ? delta > 0 : r.me };
      } else if (delta > 0) {
        reactions.push({ emoji, count: 1, me: mine });
      }
      upsertMessage({ ...message, reactions });
      return;
    }
    case "MESSAGE_ACK": {
      const parsed = parseReadState(d);
      if (parsed) setReadState(parsed.channelId, { lastReadId: parsed.lastReadId, mentionCount: parsed.mentionCount });
      return;
    }
    case "TYPING_START": {
      if (!isRecord(d)) return;
      const channelId = str(d.channel_id);
      const userId = str(d.user_id);
      if (!channelId || !userId) return;
      const until = Date.now() + TYPING_TTL_MS;
      typingByChannel = { ...typingByChannel, [channelId]: { ...(typingByChannel[channelId] ?? {}), [userId]: until } };
      const key = `${channelId}:${userId}`;
      if (typingTimers[key]) clearTimeout(typingTimers[key]);
      typingTimers[key] = setTimeout(() => {
        delete typingTimers[key];
        const current = typingByChannel[channelId];
        if (!current || !(userId in current)) return;
        const next = { ...current };
        delete next[userId];
        typingByChannel = { ...typingByChannel, [channelId]: next };
      }, TYPING_TTL_MS);
      return;
    }
    case "PRESENCE_UPDATE": {
      const p = parsePresence(d);
      if (p) setPresence(instance.id, p.userId, p.status);
      return;
    }
    case "PRESENCE_UPDATE_BULK": {
      for (const item of Array.isArray(d) ? d : []) {
        const p = parsePresence(item);
        if (p) setPresence(instance.id, p.userId, p.status);
      }
      return;
    }
    case "USER_UPDATE": {
      const user = parseUser(d);
      if (user) upsertUser(instance.id, user);
      return;
    }
    case "VOICE_STATE_UPDATE":
      applyVoiceState(instance.id, d);
      return;
    case "GATEWAY_ERROR":
      console.warn("[omnidisc] gateway error", instance.url, d);
      return;
    default:
      return;
  }
}

function mapStatus(ev: GatewayStatusEvent): { status: InstanceStatus; error?: string } {
  switch (ev.status) {
    case "ready":
      return { status: "connected" };
    case "connecting":
      return { status: "connecting" };
    case "reconnecting":
      return { status: "reconnecting", error: ev.error ?? undefined };
    case "disconnected":
    default:
      if (ev.error === ERR_UNAUTHORIZED || ev.error === ERR_NO_SESSION) return { status: "signed_out", error: ev.error };
      if (ev.error) return { status: "error", error: ev.error };
      return { status: "disconnected" };
  }
}

function settleReady(url: string, error: string | null) {
  const waiters = readyWaiters[url];
  if (!waiters) return;
  delete readyWaiters[url];
  for (const w of waiters) {
    if (error) w.reject(error);
    else w.resolve();
  }
}

function waitForReady(url: string, timeoutMs = READY_TIMEOUT_MS): Promise<void> {
  const instance = instanceByUrl(url);
  if (instance?.status === "connected") return Promise.resolve();
  return new Promise<void>((resolve, reject) => {
    const timer = setTimeout(() => {
      readyWaiters[url] = (readyWaiters[url] ?? []).filter((w) => w.resolve !== wrappedResolve);
      reject("ERR_UNREACHABLE");
    }, timeoutMs);
    const wrappedResolve = () => {
      clearTimeout(timer);
      resolve();
    };
    const wrappedReject = (e: string) => {
      clearTimeout(timer);
      reject(e);
    };
    readyWaiters[url] = [...(readyWaiters[url] ?? []), { resolve: wrappedResolve, reject: wrappedReject }];
  });
}

function handleStatus(ev: GatewayStatusEvent) {
  const instance = instanceByUrl(ev.url);
  if (!instance) return;
  const mapped = mapStatus(ev);
  instance.status = mapped.status;
  instance.error = mapped.status === "error" || mapped.status === "signed_out" ? mapped.error : undefined;
  if (mapped.status === "connected") settleReady(ev.url, null);
  else if (mapped.status === "signed_out" || mapped.status === "error") settleReady(ev.url, mapped.error ?? "ERR_UNREACHABLE");
}

let unlisteners: UnlistenFn[] = [];

export async function initOmnidisc(): Promise<void> {
  ensureLoaded();
  if (initialized) return;
  initialized = true;
  try {
    const offDispatch = await listen<GatewayDispatchEvent>("omnidisc://dispatch", (event) => {
      const payload = event.payload;
      const instance = instanceByUrl(payload.url);
      if (!instance) return;
      try {
        handleDispatch(instance, payload.t, payload.d);
      } catch (e) {
        console.warn("[omnidisc] dispatch handler failed", payload.t, errorText(e));
      }
    });
    const offStatus = await listen<GatewayStatusEvent>("omnidisc://status", (event) => handleStatus(event.payload));
    const offUpload = await listen<unknown>("omnidisc://upload", (event) => handleUploadProgress(event.payload));
    const offDecrypted = await listen<unknown>("omnidisc://decrypted", (event) =>
      handleDecryptedEvent(event.payload),
    );
    unlisteners = [offDispatch, offStatus, offUpload, offDecrypted];
  } catch (e) {
    initialized = false;
    console.warn("[omnidisc] could not subscribe to gateway events", errorText(e));
    return;
  }
  for (const instance of instances) {
    if (isDemo(instance.id)) continue;
    void connectGateway(instance);
  }
}

export function teardownOmnidisc() {
  for (const off of unlisteners) off();
  unlisteners = [];
  initialized = false;
}

export async function connectGateway(instance: OmnidiscInstance): Promise<void> {
  if (isDemo(instance.id)) return;
  instance.status = "connecting";
  instance.error = undefined;
  try {
    const snap = await invoke<{ status: string; error?: string }>("omnidisc_gateway_connect", { url: instance.url });
    handleStatus({ url: instance.url, status: snap.status as GatewayStatusEvent["status"], error: snap.error ?? null });
  } catch (e) {
    const error = errorText(e);
    handleStatus({ url: instance.url, status: "disconnected", error });
  }
}

export async function reconnectInstance(id: string) {
  const instance = getInstance(id);
  if (!instance) return;
  await connectGateway(instance);
}

export function getConnectStep(): ConnectStep {
  return connectStep;
}

export function getConnectError(): string | null {
  return connectError;
}

export function getConnectedInstance(): OmnidiscInstance | null {
  return connectedInstance;
}

export function getPendingConnect(): PendingConnect | null {
  return pendingConnect;
}

export function resetConnect() {
  connectStep = "idle";
  connectError = null;
  connectedInstance = null;
  pendingConnect = null;
}

function failConnect(e: unknown) {
  connectError = errorText(e);
  connectStep = "error";
}

async function finishConnect(pending: PendingConnect, joinInvite: boolean): Promise<OmnidiscInstance | null> {
  const instance = addInstance({
    url: pending.url,
    name: pending.name,
    status: "connecting",
    streaming: pending.streaming,
    insecure: pending.insecure,
  });
  selectedInstanceId = instance.id;
  connectStep = "syncing";
  try {
    await connectGateway(instance);
    if (instance.status !== "connected") await waitForReady(pending.url);
  } catch (e) {
    failConnect(e);
    return null;
  }
  if (joinInvite && pending.invite) {
    try {
      const raw = await invoke<unknown>("omnidisc_join_invite", { url: pending.url, code: pending.invite });
      const guild = parseGuild(raw, instance.id);
      if (guild) upsertGuild(instance.id, guild);
    } catch (e) {
      console.warn("[omnidisc] invite could not be redeemed", errorText(e));
    }
  }
  connectedInstance = instance;
  connectStep = "done";
  return instance;
}

export async function connectInstance(url: string, invite?: string): Promise<OmnidiscInstance | null> {
  ensureLoaded();
  connectStep = "connecting";
  connectError = null;
  connectedInstance = null;
  pendingConnect = null;
  let result: ConnectResult;
  try {
    result = await invoke<ConnectResult>("omnidisc_connect", {
      url,
      invite: invite && invite.trim().length > 0 ? invite.trim() : null,
    });
  } catch (e) {
    failConnect(e);
    return null;
  }
  const rawName = result.instance?.name;
  const name = typeof rawName === "string" && rawName.trim().length > 0 ? rawName.trim() : hostLabel(result.url);
  const registrationOpen = result.instance?.registration_open === true;
  const streaming = (result.instance?.streaming ?? undefined) as StreamingPolicy | undefined;
  const pending: PendingConnect = {
    url: result.url,
    name,
    invite: result.invite,
    registrationOpen,
    streaming,
    insecure: result.insecure ?? isInsecureInstanceUrl(result.url),
  };
  pendingConnect = pending;
  try {
    const session = await invoke<{ has_session: boolean }>("omnidisc_has_session", { url: result.url });
    if (!session.has_session) {
      connectStep = "auth";
      return null;
    }
  } catch (e) {
    failConnect(e);
    return null;
  }
  connectStep = "authenticating";
  const instance = await finishConnect(pending, true);
  const stepAfter = getConnectStep();
  const errorAfter = getConnectError();
  if (!instance && stepAfter === "error" && (errorAfter === ERR_UNAUTHORIZED || errorAfter === ERR_NO_SESSION)) {
    connectError = null;
    connectStep = "auth";
  }
  return instance;
}

export async function authenticate(
  mode: AuthMode,
  username: string,
  password: string,
  displayName?: string,
): Promise<OmnidiscInstance | null> {
  const pending = pendingConnect;
  if (!pending) return null;
  connectStep = "authenticating";
  connectError = null;
  try {
    if (mode === "register") {
      await invoke("omnidisc_register", {
        url: pending.url,
        username,
        password,
        displayName: displayName && displayName.trim().length > 0 ? displayName.trim() : null,
        inviteCode: pending.invite ?? null,
      });
    } else {
      await invoke("omnidisc_login", { url: pending.url, username, password });
    }
  } catch (e) {
    connectError = errorText(e);
    connectStep = "auth";
    return null;
  }
  return finishConnect(pending, mode === "login");
}

export function backToAuth() {
  if (pendingConnect) {
    connectError = null;
    connectStep = "auth";
  } else {
    resetConnect();
  }
}

export async function createGuild(instanceId: string, name: string): Promise<OmnidiscGuild | null> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return null;
  const raw = await invoke<unknown>("omnidisc_create_guild", { url: instance.url, name });
  const guild = parseGuild(raw, instanceId);
  if (guild) upsertGuild(instanceId, guild);
  return guild;
}

export async function createChannel(guildId: string, name: string, kind: ChannelKind): Promise<OmnidiscChannel | null> {
  const guild = getGuild(guildId);
  const instance = guild ? getInstance(guild.instanceId) : null;
  if (!guild || !instance || isDemo(instance.id)) return null;
  const raw = await invoke<unknown>("omnidisc_create_channel", {
    url: instance.url,
    guildId,
    name,
    kind: channelKindCode(kind),
  });
  const channel = parseChannel(raw);
  if (channel) upsertGuildChannel(instance.id, channel);
  return channel;
}

export async function createInviteLink(guildId: string, channelId?: string): Promise<string | null> {
  const guild = getGuild(guildId);
  const instance = guild ? getInstance(guild.instanceId) : null;
  if (!guild || !instance || isDemo(instance.id)) return null;
  const raw = await invoke<unknown>("omnidisc_create_invite", { url: instance.url, guildId, channelId: channelId ?? null });
  const code = isRecord(raw) ? str(raw.code) : undefined;
  if (!code) return null;
  return `${instance.url}/invite/${code}`;
}

export async function joinInvite(instanceId: string, code: string): Promise<OmnidiscGuild | null> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return null;
  const raw = await invoke<unknown>("omnidisc_join_invite", { url: instance.url, code });
  const guild = parseGuild(raw, instanceId);
  if (guild) upsertGuild(instanceId, guild);
  return guild;
}

export async function openDm(instanceId: string, userId: string): Promise<OmnidiscChannel | null> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return null;
  const raw = await invoke<unknown>("omnidisc_create_dm", { url: instance.url, recipientIds: [userId] });
  const channel = parseChannel(raw);
  if (channel) upsertDm(instanceId, channel);
  return channel;
}

export function canManageGuild(guildId: string | null): boolean {
  const guild = getGuild(guildId);
  if (!guild) return false;
  const instance = getInstance(guild.instanceId);
  return !!instance?.userId && instance.userId === guild.ownerId;
}

export function addDemoInstance(): OmnidiscInstance | null {
  if (!import.meta.env.DEV) return null;
  ensureLoaded();
  const instance = addInstance({ id: DEMO_INSTANCE_ID, url: DEMO_INSTANCE_URL, name: "Demo server", status: "connected" });
  instance.userId = "u1";
  selectedInstanceId = instance.id;
  if (!guildsByInstance[instance.id]) seedDemoData(instance.id);
  connectedInstance = instance;
  connectStep = "done";
  return instance;
}

export function hostLabel(url: string): string {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}

const NOTIFY_KEY = "omnidisc.notifications";
let suppressAck: Record<string, boolean> = {};
let jumpTarget = $state<{ channelId: string; messageId: string } | null>(null);

function loadNotifyPrefs() {
  if (notifyPrefsLoaded) return;
  notifyPrefsLoaded = true;
  if (!hasStorage()) return;
  try {
    const raw = localStorage.getItem(NOTIFY_KEY);
    if (!raw) return;
    const parsed: unknown = JSON.parse(raw);
    if (!isRecord(parsed)) return;
    const out: Record<string, NotificationLevel> = {};
    for (const [key, value] of Object.entries(parsed)) {
      if (value === "all" || value === "mentions" || value === "nothing") out[key] = value;
    }
    notificationLevels = out;
  } catch {
    notificationLevels = {};
  }
}

function persistNotifyPrefs() {
  if (!hasStorage()) return;
  try {
    localStorage.setItem(NOTIFY_KEY, JSON.stringify(notificationLevels));
  } catch {
    return;
  }
}

export function getNotificationLevel(scopeId: string): NotificationLevel | null {
  loadNotifyPrefs();
  return notificationLevels[scopeId] ?? null;
}

export function setNotificationLevel(scopeId: string, level: NotificationLevel | null) {
  loadNotifyPrefs();
  const next = { ...notificationLevels };
  if (level === null) delete next[scopeId];
  else next[scopeId] = level;
  notificationLevels = next;
  persistNotifyPrefs();
}

export function effectiveNotificationLevel(channelId: string): NotificationLevel {
  loadNotifyPrefs();
  const own = notificationLevels[channelId];
  if (own) return own;
  const guildId = channelGuild[channelId];
  if (guildId && notificationLevels[guildId]) return notificationLevels[guildId];
  return getChannel(channelId)?.guildId ? "mentions" : "all";
}

export function mentionsMe(instanceId: string, message: OmnidiscMessage): boolean {
  const me = getInstance(instanceId)?.userId;
  if (!me || message.authorId === me) return false;
  if (message.mentionsEveryone) return true;
  return (message.mentionedUserIds ?? []).includes(me);
}

async function windowFocused(): Promise<boolean> {
  try {
    const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    return await getCurrentWebviewWindow().isFocused();
  } catch {
    return false;
  }
}

async function maybeNotify(instance: OmnidiscInstance, message: OmnidiscMessage) {
  if (isDemo(instance.id)) return;
  if (message.authorId === instance.userId) return;
  const level = effectiveNotificationLevel(message.channelId);
  if (level === "nothing") return;
  const mention = mentionsMe(instance.id, message);
  const isDm = !channelGuild[message.channelId];
  if (level === "mentions" && !mention && !isDm) return;
  if (message.channelId === selectedChannelId && (await windowFocused())) return;
  const channel = getChannel(message.channelId);
  const where = channel ? (isDm ? dmTitle(channel) : `#${channel.name}`) : "";
  try {
    const n = await import("@tauri-apps/plugin-notification");
    let granted = await n.isPermissionGranted();
    if (!granted) granted = (await n.requestPermission()) === "granted";
    if (!granted) return;
    n.sendNotification({
      title: where ? `${message.authorName} — ${where}` : message.authorName,
      body: message.content.slice(0, 180),
    });
  } catch {
    // notifications are optional; a missing permission must never break the chat
  }
}

export function getMentionTotal(): number {
  let count = 0;
  for (const [channelId, state] of Object.entries(readStateByChannel)) {
    if (channelId === selectedChannelId) continue;
    count += state.mentionCount;
  }
  return count;
}

export function getGuildUnread(guildId: string): { unread: boolean; mentions: number } {
  const guild = getGuild(guildId);
  if (!guild) return { unread: false, mentions: 0 };
  let unread = false;
  let mentions = 0;
  for (const channel of guild.channels) {
    if (channel.kind === "category" || channel.kind === "voice") continue;
    if (isUnread(channel.id)) unread = true;
    mentions += getMentionCount(channel.id);
  }
  return { unread, mentions };
}

function memberOf(guildId: string | null, userId: string | null): OmnidiscMember | null {
  if (!guildId || !userId) return null;
  return (membersByGuild[guildId] ?? []).find((m) => m.id === userId) ?? null;
}

export function getRoles(guildId: string | null): OmnidiscRole[] {
  return getGuild(guildId)?.roles ?? [];
}

export function getMember(guildId: string | null, userId: string | null): OmnidiscMember | null {
  return memberOf(guildId, userId);
}

function permissionContext(channelId: string | null, guildId?: string | null) {
  const gid = guildId ?? (channelId ? channelGuild[channelId] : null) ?? null;
  const guild = getGuild(gid);
  const channel = channelId ? getChannel(channelId) : null;
  const parent = channel?.parentId ? getChannel(channel.parentId) : null;
  const instance = guild ? getInstance(guild.instanceId) : null;
  const userId = instance?.userId ?? null;
  // Membership comes from READY (the gateway only sends guilds this account is
  // in), not from the paged member list, which may simply not have reached us yet.
  return { guild, channel, parent, member: memberOf(gid, userId), userId, isMember: !!guild };
}

export function canInChannel(channelId: string | null, name: PermissionName): boolean {
  if (!channelId) return false;
  const instanceId = channelInstance[channelId];
  if (isDemo(instanceId)) return true;
  if (!channelGuild[channelId]) return true;
  return canIn(permissionContext(channelId), name);
}

export function canInGuild(guildId: string | null, name: PermissionName): boolean {
  const guild = getGuild(guildId);
  if (!guild) return false;
  if (isDemo(guild.instanceId)) return true;
  return canIn(permissionContext(null, guildId), name);
}

export function permissionBitsIn(channelId: string | null, guildId?: string | null): bigint {
  return resolvePermissions(permissionContext(channelId, guildId));
}

export function isGuildOwner(guildId: string | null): boolean {
  const guild = getGuild(guildId);
  if (!guild) return false;
  return getInstance(guild.instanceId)?.userId === guild.ownerId;
}

export function canEditMessage(message: OmnidiscMessage): boolean {
  const instanceId = channelInstance[message.channelId];
  const me = getInstance(instanceId ?? null)?.userId;
  return !!me && message.authorId === me && message.delivery === "sent";
}

export function canDeleteMessage(message: OmnidiscMessage): boolean {
  const instanceId = channelInstance[message.channelId];
  const me = getInstance(instanceId ?? null)?.userId;
  if (me && message.authorId === me) return true;
  return canInChannel(message.channelId, "MANAGE_MESSAGES");
}

export function getPins(channelId: string | null): OmnidiscMessage[] {
  if (!channelId) return [];
  return pinsByChannel[channelId] ?? [];
}

export async function loadPins(channelId: string): Promise<void> {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  const raw = await invoke<unknown>("omnidisc_list_pins", { url: instance.url, channelId });
  const resolve = nameResolver(instance.id);
  const list = (Array.isArray(raw) ? raw : [])
    .map((m) => parseMessage(m, resolve))
    .filter((m): m is OmnidiscMessage => m !== null);
  pinsByChannel = { ...pinsByChannel, [channelId]: list };
  for (const m of list) {
    if (!usersByInstance[instance.id]?.[m.authorId]) void ensureUser(instance.id, m.authorId);
  }
}

export async function setPinned(channelId: string, messageId: string, pinned: boolean): Promise<void> {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_pin_message", { url: instance.url, channelId, messageId, pinned });
  const message = (messagesByChannel[channelId] ?? []).find((m) => m.id === messageId);
  if (message) upsertMessage({ ...message, pinned });
  if (pinsByChannel[channelId]) await loadPins(channelId);
}

export interface SearchFilters {
  from?: string;
  has?: string;
  before?: string;
  after?: string;
  channel?: string;
}

export interface SearchOutcome {
  messages: OmnidiscMessage[];
  total: number;
}

export async function searchMessages(
  instanceId: string,
  scope: "guild" | "channel",
  scopeId: string,
  query: string,
  filters: SearchFilters = {},
): Promise<SearchOutcome> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return { messages: [], total: 0 };
  const raw = await invoke<unknown>("omnidisc_search", {
    url: instance.url,
    scope,
    scopeId,
    query,
    from: filters.from ?? null,
    channel: filters.channel ?? null,
    has: filters.has ?? null,
    before: filters.before ?? null,
    after: filters.after ?? null,
    limit: 100,
  });
  if (!isRecord(raw)) return { messages: [], total: 0 };
  const resolve = nameResolver(instanceId);
  const messages = (Array.isArray(raw.messages) ? raw.messages : [])
    .map((m) => parseMessage(m, resolve))
    .filter((m): m is OmnidiscMessage => m !== null)
    .reverse();
  for (const m of messages) {
    if (!usersByInstance[instanceId]?.[m.authorId]) void ensureUser(instanceId, m.authorId);
  }
  const total = typeof raw.total === "number" ? raw.total : messages.length;
  return { messages, total };
}

export function getJumpTarget(): { channelId: string; messageId: string } | null {
  return jumpTarget;
}

export function clearJumpTarget() {
  jumpTarget = null;
}

export async function jumpToMessage(channelId: string, messageId: string): Promise<void> {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) {
    jumpTarget = { channelId, messageId };
    return;
  }
  const known = (messagesByChannel[channelId] ?? []).some((m) => m.id === messageId);
  if (!known) {
    loadingByChannel = { ...loadingByChannel, [channelId]: true };
    try {
      const raw = await invoke<unknown>("omnidisc_list_messages", {
        url: instance.url,
        channelId,
        around: messageId,
        limit: PAGE_SIZE,
      });
      const resolve = nameResolver(instance.id);
      const parsed = (Array.isArray(raw) ? raw : [])
        .map((m) => parseMessage(m, resolve))
        .filter((m): m is OmnidiscMessage => m !== null);
      const byId = new Map((messagesByChannel[channelId] ?? []).map((m) => [m.id, m]));
      for (const m of parsed) byId.set(m.id, m);
      messagesByChannel = { ...messagesByChannel, [channelId]: sortMessages([...byId.values()]) };
      loadedChannels.add(channelId);
      for (const m of parsed) {
        if (!usersByInstance[instance.id]?.[m.authorId]) void ensureUser(instance.id, m.authorId);
      }
    } finally {
      const next = { ...loadingByChannel };
      delete next[channelId];
      loadingByChannel = next;
    }
  }
  jumpTarget = { channelId, messageId };
}

export function getFirstUnreadId(channelId: string | null): string | null {
  if (!channelId) return null;
  const marked = localUnread[channelId];
  if (marked) return marked;
  const read = readStateByChannel[channelId]?.lastReadId;
  if (!read) return null;
  const list = messagesByChannel[channelId] ?? [];
  const first = list.find((m) => m.delivery === "sent" && isAfter(m.id, read));
  return first?.id ?? null;
}

export function markUnread(channelId: string, messageId: string) {
  const list = messagesByChannel[channelId] ?? [];
  const index = list.findIndex((m) => m.id === messageId);
  const previous = index > 0 ? list[index - 1].id : undefined;
  setReadState(channelId, { lastReadId: previous, mentionCount: 0 });
  localUnread = { ...localUnread, [channelId]: messageId };
  suppressAck[channelId] = true;
  delete ackedByChannel[channelId];
}

export function clearUnreadMark(channelId: string) {
  if (!(channelId in localUnread)) return;
  const next = { ...localUnread };
  delete next[channelId];
  localUnread = next;
  delete suppressAck[channelId];
}

export function messageLink(channelId: string, messageId: string): string {
  const instance = instanceForChannel(channelId);
  const guildId = channelGuild[channelId];
  const base = instance?.url ?? "";
  return guildId ? `${base}/channels/${guildId}/${channelId}/${messageId}` : `${base}/channels/@me/${channelId}/${messageId}`;
}

export function getRelationships(instanceId: string | null): OmnidiscRelationship[] {
  if (!instanceId) return [];
  return relationshipsByInstance[instanceId] ?? [];
}

export async function loadRelationships(instanceId: string): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  const raw = await invoke<unknown>("omnidisc_list_relationships", { url: instance.url });
  const list = (Array.isArray(raw) ? raw : [])
    .map(parseRelationship)
    .filter((r): r is OmnidiscRelationship => r !== null);
  relationshipsByInstance = { ...relationshipsByInstance, [instanceId]: list };
  for (const rel of list) {
    if (!usersByInstance[instanceId]?.[rel.userId]) void ensureUser(instanceId, rel.userId);
  }
}

export async function addFriend(instanceId: string, username: string): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  await invoke("omnidisc_add_relationship", { url: instance.url, username, userId: null });
  await loadRelationships(instanceId);
}

export async function acceptFriend(instanceId: string, userId: string): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  await invoke("omnidisc_accept_relationship", { url: instance.url, userId });
  await loadRelationships(instanceId);
}

export async function removeFriend(instanceId: string, userId: string): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  await invoke("omnidisc_remove_relationship", { url: instance.url, userId });
  relationshipsByInstance = {
    ...relationshipsByInstance,
    [instanceId]: (relationshipsByInstance[instanceId] ?? []).filter((r) => r.userId !== userId),
  };
}

export async function blockUser(instanceId: string, userId: string): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  await invoke("omnidisc_block_user", { url: instance.url, userId });
  await loadRelationships(instanceId);
}

export function relationshipWith(instanceId: string | null, userId: string | null): OmnidiscRelationship | null {
  if (!instanceId || !userId) return null;
  return (relationshipsByInstance[instanceId] ?? []).find((r) => r.userId === userId) ?? null;
}

export function getNote(instanceId: string | null, userId: string | null): string {
  if (!instanceId || !userId) return "";
  return notesByInstance[instanceId]?.[userId] ?? "";
}

export async function loadNotes(instanceId: string): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  const raw = await invoke<unknown>("omnidisc_list_notes", { url: instance.url });
  const notes: Record<string, string> = {};
  for (const n of Array.isArray(raw) ? raw : []) {
    if (!isRecord(n)) continue;
    const userId = str(n.user_id);
    if (userId) notes[userId] = str(n.note) ?? "";
  }
  notesByInstance = { ...notesByInstance, [instanceId]: notes };
}

export async function saveNote(instanceId: string, userId: string, note: string): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  await invoke("omnidisc_put_note", { url: instance.url, userId, note });
  const current = notesByInstance[instanceId] ?? {};
  notesByInstance = { ...notesByInstance, [instanceId]: { ...current, [userId]: note } };
}

export function mutualGuilds(instanceId: string | null, userId: string | null): OmnidiscGuild[] {
  if (!instanceId || !userId) return [];
  return (guildsByInstance[instanceId] ?? []).filter((g) =>
    (membersByGuild[g.id] ?? []).some((m) => m.id === userId),
  );
}

export async function updateMe(instanceId: string, patch: Record<string, unknown>): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  const raw = await invoke<unknown>("omnidisc_update_me", { url: instance.url, patch });
  const user = parseUser(raw);
  if (user) upsertUser(instanceId, user);
}

export function getOwnPresence(instanceId: string | null): string {
  if (!instanceId) return "online";
  return ownPresenceByInstance[instanceId] ?? "online";
}

export async function setOwnPresence(
  instanceId: string,
  status: string,
  custom?: { text?: string; expiresAt?: string },
): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  const payload: Record<string, unknown> = { user_id: instance.userId ?? "0", status };
  if (custom?.text) {
    payload.custom_status = { text: custom.text, expires_at: custom.expiresAt ?? null };
  }
  await invoke("omnidisc_gateway_send", { url: instance.url, op: 3, d: payload });
  ownPresenceByInstance = { ...ownPresenceByInstance, [instanceId]: status };
  if (instance.userId) setPresence(instanceId, instance.userId, status);
}

export async function listSessions(instanceId: string): Promise<OmnidiscSession[]> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return [];
  const raw = await invoke<unknown>("omnidisc_list_sessions", { url: instance.url });
  return (Array.isArray(raw) ? raw : [])
    .map(parseSession)
    .filter((s): s is OmnidiscSession => s !== null);
}

export async function revokeSession(instanceId: string, sessionId: string): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  await invoke("omnidisc_revoke_session", { url: instance.url, sessionId });
}

export async function revokeOtherSessions(instanceId: string): Promise<void> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instanceId)) return;
  await invoke("omnidisc_revoke_other_sessions", { url: instance.url });
}

function guildInstance(guildId: string): OmnidiscInstance | null {
  const guild = getGuild(guildId);
  return guild ? getInstance(guild.instanceId) : null;
}

export async function updateGuild(guildId: string, patch: Record<string, unknown>): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  const raw = await invoke<unknown>("omnidisc_update_guild", { url: instance.url, guildId, patch });
  const guild = parseGuild(raw, instance.id);
  if (guild) upsertGuild(instance.id, guild);
}

export async function deleteGuild(guildId: string): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_delete_guild", { url: instance.url, guildId });
  removeGuild(instance.id, guildId);
}

export async function leaveGuild(guildId: string): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_leave_guild", { url: instance.url, guildId });
  removeGuild(instance.id, guildId);
}

export async function transferGuild(guildId: string, userId: string): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  const raw = await invoke<unknown>("omnidisc_transfer_guild", { url: instance.url, guildId, userId });
  const guild = parseGuild(raw, instance.id);
  if (guild) upsertGuild(instance.id, guild);
}

export async function createRole(guildId: string, name: string, permissions: string): Promise<OmnidiscRole | null> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return null;
  const raw = await invoke<unknown>("omnidisc_create_role", {
    url: instance.url,
    guildId,
    name,
    permissions,
    color: null,
    hoist: false,
    mentionable: false,
  });
  const role = parseRole(raw);
  const guild = getGuild(guildId);
  if (role && guild) {
    upsertGuild(instance.id, { ...guild, roles: [...guild.roles, role].sort((a, b) => a.position - b.position) });
  }
  return role;
}

export async function updateRole(guildId: string, roleId: string, patch: Record<string, unknown>): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  const raw = await invoke<unknown>("omnidisc_update_role", { url: instance.url, guildId, roleId, patch });
  const role = parseRole(raw);
  const guild = getGuild(guildId);
  if (role && guild) {
    const rest = guild.roles.filter((r) => r.id !== role.id);
    upsertGuild(instance.id, { ...guild, roles: [...rest, role].sort((a, b) => a.position - b.position) });
  }
}

export async function deleteRole(guildId: string, roleId: string): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_delete_role", { url: instance.url, guildId, roleId });
  const guild = getGuild(guildId);
  if (guild) upsertGuild(instance.id, { ...guild, roles: guild.roles.filter((r) => r.id !== roleId) });
}

export async function setMemberRole(guildId: string, userId: string, roleId: string, granted: boolean): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_set_member_role", { url: instance.url, guildId, userId, roleId, granted });
}

export async function updateMember(guildId: string, userId: string, patch: Record<string, unknown>): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_update_member", { url: instance.url, guildId, userId, patch });
}

export async function kickMember(guildId: string, userId: string, reason?: string): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_kick_member", { url: instance.url, guildId, userId, reason: reason ?? null });
}

export async function banMember(guildId: string, userId: string, reason?: string): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_ban_member", { url: instance.url, guildId, userId, reason: reason ?? null });
}

export async function unbanMember(guildId: string, userId: string): Promise<void> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_unban_member", { url: instance.url, guildId, userId });
}

export async function listBans(guildId: string): Promise<OmnidiscBan[]> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return [];
  const raw = await invoke<unknown>("omnidisc_list_bans", { url: instance.url, guildId });
  const list = (Array.isArray(raw) ? raw : []).map(parseBan).filter((b): b is OmnidiscBan => b !== null);
  for (const b of list) {
    if (!usersByInstance[instance.id]?.[b.userId]) void ensureUser(instance.id, b.userId);
  }
  return list;
}

export async function loadAuditLog(guildId: string, action?: string): Promise<OmnidiscAuditEntry[]> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return [];
  const raw = await invoke<unknown>("omnidisc_audit_log", {
    url: instance.url,
    guildId,
    action: action ?? null,
    actorId: null,
    before: null,
    limit: 50,
  });
  const list = (Array.isArray(raw) ? raw : [])
    .map(parseAuditEntry)
    .filter((e): e is OmnidiscAuditEntry => e !== null);
  for (const e of list) {
    if (!usersByInstance[instance.id]?.[e.actorId]) void ensureUser(instance.id, e.actorId);
    if (e.targetId && !usersByInstance[instance.id]?.[e.targetId]) void ensureUser(instance.id, e.targetId);
  }
  return list;
}

export async function updateChannelSettings(channelId: string, patch: Record<string, unknown>): Promise<void> {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  const raw = await invoke<unknown>("omnidisc_update_channel", { url: instance.url, channelId, patch });
  const channel = parseChannel(raw);
  if (channel?.guildId) upsertGuildChannel(instance.id, channel);
}

export async function deleteChannel(channelId: string): Promise<void> {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_delete_channel", { url: instance.url, channelId });
  removeChannel(instance.id, channelId);
}

export async function putOverwrite(
  channelId: string,
  targetId: string,
  targetKind: "role" | "member",
  allow: string,
  deny: string,
): Promise<void> {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_put_overwrite", { url: instance.url, channelId, targetId, targetKind, allow, deny });
  const channel = getChannel(channelId);
  if (channel?.guildId) {
    const rest = (channel.overwrites ?? []).filter((o) => o.targetId !== targetId);
    upsertGuildChannel(instance.id, { ...channel, overwrites: [...rest, { targetId, targetKind, allow, deny }] });
  }
}

export async function deleteOverwrite(channelId: string, targetId: string): Promise<void> {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_delete_overwrite", { url: instance.url, channelId, targetId });
  const channel = getChannel(channelId);
  if (channel?.guildId) {
    upsertGuildChannel(instance.id, {
      ...channel,
      overwrites: (channel.overwrites ?? []).filter((o) => o.targetId !== targetId),
    });
  }
}

export async function createInviteWithOptions(
  guildId: string,
  channelId: string | undefined,
  maxAgeSeconds: number | null,
  maxUses: number | null,
): Promise<string | null> {
  const instance = guildInstance(guildId);
  if (!instance || isDemo(instance.id)) return null;
  const raw = await invoke<unknown>("omnidisc_create_invite", {
    url: instance.url,
    guildId,
    channelId: channelId ?? null,
    maxAgeSeconds,
    maxUses,
  });
  const code = isRecord(raw) ? str(raw.code) : undefined;
  return code ? `${instance.url}/invite/${code}` : null;
}

// ---------------------------------------------------------------------------
// Attachments, uploads and end-to-end encryption
// ---------------------------------------------------------------------------

let uploadsByChannel = $state<Record<string, PendingAttachment[]>>({});
let devicesByInstance = $state<Record<string, OmnidiscDevice[]>>({});
let thisDeviceByInstance = $state<Record<string, string>>({});
let limitsByInstance: Record<string, { maxUploadBytes: number; maxAttachments: number }> = {};
let groupStatusByChannel = $state<Record<string, OmnidiscGroupStatus>>({});
// Plaintext that arrived before its MESSAGE_CREATE, keyed by ciphertext.
let pendingDecrypts: Record<string, DecryptedPayload> = {};
// Whether that plaintext's sending device matched the published roster.
let unverifiedSenders: Record<string, boolean> = {};

interface DecryptedPayload {
  content: string;
  replyToId?: string;
  attachments: OmnidiscAttachment[];
}

export function isEncryptedChannel(channelId: string | null): boolean {
  const channel = getChannel(channelId);
  return channel?.kind === "dm" || channel?.kind === "group_dm";
}

function readPayload(raw: unknown): DecryptedPayload | null {
  if (!isRecord(raw)) return null;
  const files = Array.isArray(raw.files) ? raw.files : [];
  const attachments: OmnidiscAttachment[] = [];
  for (const f of files) {
    if (!isRecord(f)) continue;
    const id = str(f.attachment_id);
    const filename = str(f.name);
    if (!id || !filename) continue;
    attachments.push({
      id,
      filename,
      size: typeof f.size === "number" ? f.size : 0,
      contentType: str(f.mime),
      encrypted: true,
    });
  }
  return { content: str(raw.content) ?? "", replyToId: str(raw.reply_to), attachments };
}

function applyDecrypted(
  channelId: string,
  ciphertext: string,
  payload: DecryptedPayload,
  senderVerified = true,
) {
  const list = messagesByChannel[channelId] ?? [];
  const target = list.find((m) => m.ciphertext === ciphertext);
  if (!target) {
    pendingDecrypts[ciphertext] = payload;
    unverifiedSenders[ciphertext] = senderVerified;
    return;
  }
  upsertMessage({
    ...target,
    content: payload.content,
    replyToId: payload.replyToId ?? target.replyToId,
    attachments: payload.attachments,
    awaitingDecryption: false,
    senderVerified,
  });
}

function fillFromPending(message: OmnidiscMessage): OmnidiscMessage {
  if (!message.ciphertext) return message;
  const payload = pendingDecrypts[message.ciphertext];
  if (!payload) return message;
  const senderVerified = unverifiedSenders[message.ciphertext] ?? true;
  delete pendingDecrypts[message.ciphertext];
  delete unverifiedSenders[message.ciphertext];
  return {
    ...message,
    content: payload.content,
    replyToId: payload.replyToId ?? message.replyToId,
    attachments: payload.attachments,
    awaitingDecryption: false,
    senderVerified,
  };
}

/// Scrollback: MLS ratchets forward, so old ciphertext can only come from the
/// local cache in Rust. Anything it cannot recall stays honestly blank.
export async function hydrateEncrypted(channelId: string) {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return;
  const waiting = (messagesByChannel[channelId] ?? [])
    .filter((m) => m.awaitingDecryption && m.ciphertext)
    .map((m) => m.ciphertext as string);
  if (waiting.length === 0) return;
  try {
    const found = await invoke<Record<string, unknown>>("omnidisc_mls_recall", {
      url: instance.url,
      ciphertexts: waiting,
    });
    for (const [ciphertext, raw] of Object.entries(found ?? {})) {
      const payload = readPayload(raw);
      if (payload) applyDecrypted(channelId, ciphertext, payload);
    }
  } catch (e) {
    console.warn("[omnidisc] could not read cached plaintext", errorText(e));
  }
}

export function getGroupStatus(channelId: string | null): OmnidiscGroupStatus | null {
  return channelId ? (groupStatusByChannel[channelId] ?? null) : null;
}

export async function refreshGroupStatus(channelId: string) {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id) || !isEncryptedChannel(channelId)) return;
  try {
    const raw = await invoke<unknown>("omnidisc_mls_status", { url: instance.url, channelId });
    if (!isRecord(raw)) return;
    const members = (Array.isArray(raw.members) ? raw.members : [])
      .filter(isRecord)
      .map((m) => ({
        userId: str(m.user_id) ?? "",
        deviceId: str(m.device_id) ?? "",
        fingerprint: str(m.fingerprint) ?? "",
        isMe: m.is_me === true,
      }));
    groupStatusByChannel = {
      ...groupStatusByChannel,
      [channelId]: {
        ready: raw.ready === true,
        groupId: str(raw.group_id) ?? "",
        epoch: typeof raw.epoch === "number" ? raw.epoch : undefined,
        members,
      },
    };
  } catch (e) {
    console.warn("[omnidisc] could not read the group status", errorText(e));
  }
}

export async function getUploadLimits(instanceId: string) {
  const cached = limitsByInstance[instanceId];
  if (cached) return cached;
  const instance = getInstance(instanceId);
  if (!instance) return { maxUploadBytes: 0, maxAttachments: 10 };
  try {
    const raw = await invoke<unknown>("omnidisc_instance_limits", { url: instance.url });
    const limits = isRecord(raw) && isRecord(raw.limits) ? raw.limits : {};
    const value = {
      maxUploadBytes:
        typeof limits.max_upload_bytes === "number"
          ? limits.max_upload_bytes
          : typeof (raw as Record<string, unknown>).max_upload_bytes === "number"
            ? ((raw as Record<string, unknown>).max_upload_bytes as number)
            : 0,
      maxAttachments: typeof limits.max_attachments === "number" ? limits.max_attachments : 10,
    };
    limitsByInstance[instanceId] = value;
    return value;
  } catch {
    return { maxUploadBytes: 0, maxAttachments: 10 };
  }
}

export function getPendingAttachments(channelId: string | null): PendingAttachment[] {
  return channelId ? (uploadsByChannel[channelId] ?? []) : [];
}

function putUpload(next: PendingAttachment) {
  const list = uploadsByChannel[next.channelId] ?? [];
  const index = list.findIndex((u) => u.id === next.id);
  const updated = index >= 0 ? list.map((u) => (u.id === next.id ? next : u)) : [...list, next];
  uploadsByChannel = { ...uploadsByChannel, [next.channelId]: updated };
}

export function removePendingAttachment(channelId: string, id: string) {
  const list = uploadsByChannel[channelId] ?? [];
  const target = list.find((u) => u.id === id);
  if (target?.previewUrl) URL.revokeObjectURL(target.previewUrl);
  uploadsByChannel = { ...uploadsByChannel, [channelId]: list.filter((u) => u.id !== id) };
  void invoke("omnidisc_upload_cancel", { id }).catch(() => {});
}

function clearUploads(channelId: string) {
  for (const upload of uploadsByChannel[channelId] ?? []) {
    if (upload.previewUrl) URL.revokeObjectURL(upload.previewUrl);
  }
  const next = { ...uploadsByChannel };
  delete next[channelId];
  uploadsByChannel = next;
}

export async function attachFile(
  channelId: string,
  file: { path: string; name?: string; previewUrl?: string },
): Promise<string | null> {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return null;
  const limits = await getUploadLimits(instance.id);
  const already = uploadsByChannel[channelId] ?? [];
  if (already.length >= limits.maxAttachments) {
    throw new Error(`ERR_TOO_MANY_ATTACHMENTS:${limits.maxAttachments}`);
  }
  const encrypted = isEncryptedChannel(channelId);
  // Rust owns the size check: it reads the file anyway and knows the instance
  // limit, so the answer arrives before the first byte moves.
  const started = await invoke<{ id: string; size: number; name: string }>(
    "omnidisc_upload_start",
    { url: instance.url, channelId, path: file.path, encrypt: encrypted },
  );
  putUpload({
    id: started.id,
    channelId,
    name: file.name ?? started.name,
    path: file.path,
    sent: 0,
    total: started.size,
    state: "preparing",
    encrypted,
    previewUrl: file.previewUrl,
  });
  return started.id;
}

export async function retryAttachment(channelId: string, id: string) {
  const list = uploadsByChannel[channelId] ?? [];
  const target = list.find((u) => u.id === id);
  if (!target) return;
  removePendingAttachment(channelId, id);
  await attachFile(channelId, {
    path: target.path,
    name: target.name,
    previewUrl: target.previewUrl,
  });
}

function handleUploadProgress(raw: unknown) {
  if (!isRecord(raw)) return;
  const id = str(raw.id);
  const channelId = str(raw.channel_id);
  if (!id || !channelId) return;
  const list = uploadsByChannel[channelId] ?? [];
  const current = list.find((u) => u.id === id);
  if (!current) return;
  const state = (str(raw.state) ?? "uploading") as PendingAttachment["state"];
  if (state === "cancelled") {
    removePendingAttachment(channelId, id);
    return;
  }
  putUpload({
    ...current,
    sent: typeof raw.sent === "number" ? raw.sent : current.sent,
    total: typeof raw.total === "number" && raw.total > 0 ? raw.total : current.total,
    state,
    error: str(raw.error),
    mime: str(raw.mime) ?? current.mime,
  });
}

function handleDecryptedEvent(raw: unknown) {
  if (!isRecord(raw)) return;
  const channelId = str(raw.channel_id);
  const ciphertext = str(raw.ciphertext);
  if (!channelId || !ciphertext) return;
  const payload = readPayload(raw.payload);
  if (payload) applyDecrypted(channelId, ciphertext, payload, raw.sender_verified !== false);
}

export async function downloadAttachment(
  channelId: string,
  message: OmnidiscMessage,
  attachment: OmnidiscAttachment,
): Promise<string | null> {
  const instance = instanceForChannel(channelId);
  if (!instance || isDemo(instance.id)) return null;
  const result = await invoke<{ path: string; name: string }>("omnidisc_download_attachment", {
    url: instance.url,
    attachmentUrl: attachment.url ?? null,
    attachmentId: attachment.id,
    filename: attachment.filename,
    ciphertext: attachment.encrypted ? (message.ciphertext ?? null) : null,
  });
  return result?.path ?? null;
}

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

export function getDevices(instanceId: string | null): OmnidiscDevice[] {
  return instanceId ? (devicesByInstance[instanceId] ?? []) : [];
}

export function getThisDeviceId(instanceId: string | null): string | null {
  return instanceId ? (thisDeviceByInstance[instanceId] ?? null) : null;
}

export async function loadDevices(instanceId: string, userId?: string): Promise<OmnidiscDevice[]> {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instance.id)) return [];
  const [mine, raw] = await Promise.all([
    invoke<{ device_id: string }>("omnidisc_device_fingerprint", { url: instance.url }).catch(
      () => null,
    ),
    invoke<unknown>("omnidisc_list_user_devices", { url: instance.url, userId: userId ?? "@me" }),
  ]);
  if (mine?.device_id) {
    thisDeviceByInstance = { ...thisDeviceByInstance, [instanceId]: mine.device_id };
  }
  const devices = (Array.isArray(raw) ? raw : [])
    .map(parseDevice)
    .filter((d): d is OmnidiscDevice => d !== null);
  if (!userId || userId === "@me") {
    devicesByInstance = { ...devicesByInstance, [instanceId]: devices };
  }
  return devices;
}

export async function revokeDevice(instanceId: string, deviceId: string) {
  const instance = getInstance(instanceId);
  if (!instance || isDemo(instance.id)) return;
  await invoke("omnidisc_revoke_device", { url: instance.url, deviceId });
  await loadDevices(instanceId);
}
