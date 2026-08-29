export type InstanceStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "signed_out"
  | "error";

export interface StreamingPolicy {
  max_height: number;
  max_fps: number;
  min_kbps: number;
  max_kbps: number;
  step_kbps: number;
  allow_custom_bitrate: boolean;
  overrides?: Record<string, number>;
  preferred_codec?: string;
  allow_h265?: boolean;
}

export interface OmnidiscInstance {
  id: string;
  url: string;
  name: string;
  status: InstanceStatus;
  error?: string;
  userId?: string;
  streaming?: StreamingPolicy;
  /// Reached over plain http, so the session token and every message are
  /// readable on the wire. Loopback does not count.
  insecure?: boolean;
}

export type ChannelKind = "text" | "voice" | "category" | "dm" | "group_dm" | "link" | "notes";

export interface OmnidiscOverwrite {
  targetId: string;
  targetKind: "role" | "member";
  allow: string;
  deny: string;
}

export interface OmnidiscChannel {
  id: string;
  name: string;
  kind: ChannelKind;
  e2ee?: boolean;
  category?: string;
  parentId?: string;
  position: number;
  guildId?: string;
  recipientIds?: string[];
  lastMessageId?: string;
  topic?: string;
  nsfw?: boolean;
  slowmodeSeconds?: number;
  bitrate?: number;
  userLimit?: number;
  overwrites?: OmnidiscOverwrite[];
}

export interface OmnidiscRole {
  id: string;
  name: string;
  permissions: string;
  position: number;
  color?: number;
  hoist: boolean;
  mentionable: boolean;
  isEveryone: boolean;
}

export interface OmnidiscGuild {
  id: string;
  instanceId: string;
  name: string;
  ownerId: string;
  description?: string;
  channels: OmnidiscChannel[];
  roles: OmnidiscRole[];
}

export interface OmnidiscMember {
  id: string;
  name: string;
  online: boolean;
  role?: string;
  nick?: string;
  roleIds?: string[];
  mutedUntil?: string;
}

export interface OmnidiscUser {
  id: string;
  username: string;
  displayName: string;
  avatar?: string;
  banner?: string;
  accentColor?: number;
  bio?: string;
  pronouns?: string;
}

export type RelationshipKind = "friend" | "incoming_request" | "outgoing_request" | "blocked";

export interface OmnidiscRelationship {
  userId: string;
  kind: RelationshipKind;
  since: string;
}

export interface OmnidiscSession {
  sessionId: string;
  client: string;
  deviceName?: string;
  lastSeen: string;
  current: boolean;
}

export interface OmnidiscBan {
  userId: string;
  bannedBy: string;
  reason?: string;
  createdAt: string;
}

export interface OmnidiscAuditEntry {
  id: string;
  actorId: string;
  action: string;
  targetId?: string;
  reason?: string;
  createdAt: string;
}

export type NotificationLevel = "all" | "mentions" | "nothing";

export interface OmnidiscReaction {
  emoji: string;
  count: number;
  me: boolean;
}

export type MessageDelivery = "sent" | "pending" | "failed";

export interface OmnidiscAttachment {
  id: string;
  filename: string;
  size: number;
  contentType?: string;
  url?: string;
  thumbnailUrl?: string;
  thumbhash?: string;
  width?: number;
  height?: number;
  durationMs?: number;
  encrypted: boolean;
  /// When the server deletes the bytes. Files here are temporary by design.
  expiresAt?: number;
  expired?: boolean;
}

export type StoragePressure = "ok" | "warning" | "critical" | "purged";

export interface OmnidiscStorage {
  usedBytes: number;
  totalBytes: number;
  ratio: number;
  level: StoragePressure;
  attachmentTtlSeconds: number;
  purgedFiles?: number;
}

export interface OmnidiscDevice {
  userId: string;
  deviceId: string;
  publicKey: string;
  fingerprint?: string;
  name?: string;
  createdAt: string;
  lastSeenAt: string;
  revokedAt?: string;
}

export interface OmnidiscGroupMember {
  userId: string;
  deviceId: string;
  fingerprint: string;
  isMe: boolean;
}

export interface OmnidiscGroupStatus {
  ready: boolean;
  groupId: string;
  epoch?: number;
  members: OmnidiscGroupMember[];
}

export type UploadState = "preparing" | "uploading" | "resuming" | "done" | "failed" | "cancelled";

export interface PendingAttachment {
  id: string;
  channelId: string;
  name: string;
  path: string;
  sent: number;
  total: number;
  state: UploadState;
  error?: string;
  mime?: string;
  encrypted: boolean;
  previewUrl?: string;
}

export interface OmnidiscMessage {
  id: string;
  channelId: string;
  authorId: string;
  authorName: string;
  content: string;
  createdAt: number;
  editedAt?: number;
  delivery?: MessageDelivery;
  error?: string;
  replyToId?: string;
  reactions?: OmnidiscReaction[];
  pinned?: boolean;
  mentionsMe?: boolean;
  mentionedUserIds?: string[];
  mentionsEveryone?: boolean;
  attachments?: OmnidiscAttachment[];
  encrypted?: boolean;
  ciphertext?: string;
  awaitingDecryption?: boolean;
  /// Set only on decrypted messages: false means the sending device is not one
  /// the claimed author published, so the name on the bubble is not proof.
  senderVerified?: boolean;
}

export type ConnectStep =
  | "idle"
  | "connecting"
  | "auth"
  | "authenticating"
  | "syncing"
  | "error"
  | "done";

export type AuthMode = "login" | "register";

export interface ConnectResult {
  url: string;
  recognized: boolean;
  insecure?: boolean;
  instance: Record<string, unknown>;
  invite?: string;
}

export interface GatewayStatusEvent {
  url: string;
  status: "connecting" | "ready" | "reconnecting" | "disconnected";
  error?: string | null;
}

export interface GatewayDispatchEvent {
  url: string;
  t: string;
  d: unknown;
}
