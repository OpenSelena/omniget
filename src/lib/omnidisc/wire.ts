import type {
  ChannelKind,
  OmnidiscAttachment,
  OmnidiscDevice,
  OmnidiscAuditEntry,
  OmnidiscBan,
  OmnidiscChannel,
  OmnidiscGuild,
  OmnidiscMessage,
  OmnidiscOverwrite,
  OmnidiscReaction,
  OmnidiscRelationship,
  OmnidiscRole,
  OmnidiscSession,
  OmnidiscUser,
  RelationshipKind,
} from "./types";
import { snowflakeTime } from "./snowflake";

type Json = Record<string, unknown>;

export function isRecord(v: unknown): v is Json {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

export function str(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

function num(v: unknown): number | undefined {
  return typeof v === "number" && Number.isFinite(v) ? v : undefined;
}

function strList(v: unknown): string[] {
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
}

const CHANNEL_KINDS: Record<number, ChannelKind> = {
  0: "text",
  1: "dm",
  2: "voice",
  3: "group_dm",
  4: "category",
  5: "link",
  6: "notes",
};

export function channelKindCode(kind: ChannelKind): number {
  for (const [code, k] of Object.entries(CHANNEL_KINDS)) {
    if (k === kind) return Number(code);
  }
  return 0;
}

function parseOverwrites(raw: unknown): OmnidiscOverwrite[] {
  if (!Array.isArray(raw)) return [];
  const out: OmnidiscOverwrite[] = [];
  for (const o of raw) {
    if (!isRecord(o)) continue;
    const targetId = str(o.target_id);
    const targetKind = o.target_kind === "member" ? "member" : "role";
    if (!targetId) continue;
    out.push({ targetId, targetKind, allow: str(o.allow) ?? "0", deny: str(o.deny) ?? "0" });
  }
  return out;
}

export function parseRole(raw: unknown): OmnidiscRole | null {
  if (!isRecord(raw)) return null;
  const id = str(raw.id);
  if (!id) return null;
  return {
    id,
    name: str(raw.name) ?? id,
    permissions: str(raw.permissions) ?? "0",
    position: num(raw.position) ?? 0,
    color: num(raw.color),
    hoist: raw.hoist === true,
    mentionable: raw.mentionable === true,
    isEveryone: raw.is_everyone === true,
  };
}

export function parseChannel(raw: unknown): OmnidiscChannel | null {
  if (!isRecord(raw)) return null;
  const id = str(raw.id);
  if (!id) return null;
  const kind = CHANNEL_KINDS[num(raw.type) ?? -1] ?? "text";
  return {
    id,
    name: str(raw.name) ?? "",
    kind,
    parentId: str(raw.parent_id),
    position: num(raw.position) ?? 0,
    guildId: str(raw.guild_id),
    recipientIds: strList(raw.recipient_ids),
    e2ee: raw.e2ee === true,
    lastMessageId: str(raw.last_message_id),
    topic: str(raw.topic),
    nsfw: raw.nsfw === true,
    slowmodeSeconds: num(raw.slowmode_seconds),
    bitrate: num(raw.bitrate),
    userLimit: num(raw.user_limit),
    overwrites: parseOverwrites(raw.overwrites),
  };
}

export function parseGuild(raw: unknown, instanceId: string): OmnidiscGuild | null {
  if (!isRecord(raw)) return null;
  const id = str(raw.id);
  if (!id) return null;
  const channels = (Array.isArray(raw.channels) ? raw.channels : [])
    .map(parseChannel)
    .filter((c): c is OmnidiscChannel => c !== null);
  const roles = (Array.isArray(raw.roles) ? raw.roles : [])
    .map(parseRole)
    .filter((r): r is OmnidiscRole => r !== null)
    .sort((a, b) => a.position - b.position);
  return {
    id,
    instanceId,
    name: str(raw.name) ?? id,
    ownerId: str(raw.owner_id) ?? "",
    description: str(raw.description),
    channels: withCategories(channels),
    roles,
  };
}

export function withCategories(channels: OmnidiscChannel[]): OmnidiscChannel[] {
  const categories = new Map<string, OmnidiscChannel>();
  for (const c of channels) {
    if (c.kind === "category") categories.set(c.id, c);
  }
  const sorted = [...channels].sort((a, b) => {
    const ca = a.kind === "category" ? a : a.parentId ? categories.get(a.parentId) : undefined;
    const cb = b.kind === "category" ? b : b.parentId ? categories.get(b.parentId) : undefined;
    const pa = ca ? ca.position : -1;
    const pb = cb ? cb.position : -1;
    if (pa !== pb) return pa - pb;
    if (a.kind === "category" && b.kind !== "category") return -1;
    if (b.kind === "category" && a.kind !== "category") return 1;
    if (a.position !== b.position) return a.position - b.position;
    return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
  });
  return sorted.map((c) => ({
    ...c,
    category: c.parentId ? categories.get(c.parentId)?.name : undefined,
  }));
}

export function parseUser(raw: unknown): OmnidiscUser | null {
  if (!isRecord(raw)) return null;
  const id = str(raw.id);
  const username = str(raw.username);
  if (!id || !username) return null;
  const displayName = str(raw.display_name);
  return {
    id,
    username,
    displayName: displayName && displayName.trim().length > 0 ? displayName : username,
    avatar: str(raw.avatar),
    banner: str(raw.banner),
    accentColor: num(raw.accent_color),
    bio: str(raw.bio),
    pronouns: str(raw.pronouns),
  };
}

const RELATIONSHIP_KINDS: RelationshipKind[] = ["friend", "incoming_request", "outgoing_request", "blocked"];

export function parseRelationship(raw: unknown): OmnidiscRelationship | null {
  if (!isRecord(raw)) return null;
  const userId = str(raw.user_id);
  const kind = str(raw.kind);
  if (!userId || !kind) return null;
  const known = RELATIONSHIP_KINDS.find((k) => k === kind);
  if (!known) return null;
  return { userId, kind: known, since: str(raw.since) ?? "" };
}

export function parseSession(raw: unknown): OmnidiscSession | null {
  if (!isRecord(raw)) return null;
  const sessionId = str(raw.session_id);
  if (!sessionId) return null;
  const client = isRecord(raw.client) ? raw.client : {};
  return {
    sessionId,
    client: str(client.client) ?? "",
    deviceName: str(client.device_name),
    lastSeen: str(raw.last_seen) ?? "",
    current: raw.current === true,
  };
}

export function parseBan(raw: unknown): OmnidiscBan | null {
  if (!isRecord(raw)) return null;
  const userId = str(raw.user_id);
  if (!userId) return null;
  return {
    userId,
    bannedBy: str(raw.banned_by) ?? "",
    reason: str(raw.reason),
    createdAt: str(raw.created_at) ?? "",
  };
}

export function parseAuditEntry(raw: unknown): OmnidiscAuditEntry | null {
  if (!isRecord(raw)) return null;
  const id = str(raw.id);
  const action = str(raw.action);
  if (!id || !action) return null;
  return {
    id,
    actorId: str(raw.actor_id) ?? "",
    action,
    targetId: str(raw.target_id),
    reason: str(raw.reason),
    createdAt: str(raw.created_at) ?? "",
  };
}

export function emojiKey(raw: unknown): string {
  if (!isRecord(raw)) return "";
  const name = str(raw.name) ?? "";
  const id = str(raw.id);
  return id ? `${name}:${id}` : name;
}

function parseReactions(raw: unknown): OmnidiscReaction[] {
  if (!Array.isArray(raw)) return [];
  const out: OmnidiscReaction[] = [];
  for (const r of raw) {
    if (!isRecord(r)) continue;
    const emoji = emojiKey(r.emoji);
    if (!emoji) continue;
    out.push({ emoji, count: num(r.count) ?? 0, me: r.me === true });
  }
  return out;
}

export function parseAttachment(raw: unknown): OmnidiscAttachment | null {
  if (!isRecord(raw)) return null;
  const id = str(raw.id);
  const filename = str(raw.filename);
  if (!id || !filename) return null;
  return {
    id,
    filename,
    size: num(raw.size) ?? 0,
    contentType: str(raw.content_type),
    url: str(raw.url),
    thumbnailUrl: str(raw.thumbnail_url),
    thumbhash: str(raw.thumbhash),
    width: num(raw.width),
    height: num(raw.height),
    durationMs: num(raw.duration_ms),
    encrypted: raw.encrypted === true,
  };
}

export function parseDevice(raw: unknown): OmnidiscDevice | null {
  if (!isRecord(raw)) return null;
  const deviceId = str(raw.device_id);
  if (!deviceId) return null;
  return {
    userId: str(raw.user_id) ?? "",
    deviceId,
    publicKey: str(raw.ed25519_pub) ?? "",
    fingerprint: str(raw.fingerprint),
    name: str(raw.name),
    createdAt: str(raw.created_at) ?? "",
    lastSeenAt: str(raw.last_seen_at) ?? "",
    revokedAt: str(raw.revoked_at),
  };
}

export function parseMessage(raw: unknown, resolveName: (authorId: string) => string): OmnidiscMessage | null {
  if (!isRecord(raw)) return null;
  const id = str(raw.id);
  const channelId = str(raw.channel_id);
  const authorId = str(raw.author_id);
  if (!id || !channelId || !authorId) return null;
  const editedRaw = str(raw.edited_at);
  const editedAt = editedRaw ? Date.parse(editedRaw) : NaN;
  const reference = isRecord(raw.reference) ? str(raw.reference.message_id) : undefined;
  const e2ee = isRecord(raw.e2ee) ? str(raw.e2ee.ciphertext) : undefined;
  const attachments = (Array.isArray(raw.attachments) ? raw.attachments : [])
    .map(parseAttachment)
    .filter((a): a is OmnidiscAttachment => a !== null);
  return {
    id,
    channelId,
    authorId,
    authorName: resolveName(authorId),
    content: str(raw.content) ?? "",
    createdAt: snowflakeTime(id),
    editedAt: Number.isFinite(editedAt) ? editedAt : undefined,
    delivery: "sent",
    replyToId: reference,
    reactions: parseReactions(raw.reactions),
    pinned: raw.pinned === true,
    mentionedUserIds: isRecord(raw.mentions) ? strList(raw.mentions.users) : [],
    mentionsEveryone: isRecord(raw.mentions) ? raw.mentions.everyone === true : false,
    attachments,
    encrypted: e2ee !== undefined,
    ciphertext: e2ee,
    awaitingDecryption: e2ee !== undefined,
  };
}

export interface ReadStateWire {
  channelId: string;
  lastReadId?: string;
  mentionCount: number;
}

export function parseReadState(raw: unknown): ReadStateWire | null {
  if (!isRecord(raw)) return null;
  const channelId = str(raw.channel_id);
  if (!channelId) return null;
  return { channelId, lastReadId: str(raw.last_read_id), mentionCount: num(raw.mention_count) ?? 0 };
}

export interface PresenceWire {
  userId: string;
  status: string;
}

export function parsePresence(raw: unknown): PresenceWire | null {
  if (!isRecord(raw)) return null;
  const userId = str(raw.user_id);
  if (!userId) return null;
  return { userId, status: str(raw.status) ?? "offline" };
}
