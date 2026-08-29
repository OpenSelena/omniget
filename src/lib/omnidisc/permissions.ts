import type { OmnidiscChannel, OmnidiscGuild, OmnidiscMember, OmnidiscOverwrite } from "./types";

export const PERMISSION_BITS = {
  VIEW_CHANNEL: 0,
  SEND_MESSAGES: 1,
  EMBED_LINKS: 2,
  ATTACH_FILES: 3,
  ADD_REACTIONS: 4,
  USE_EXTERNAL_EMOJI: 5,
  MENTION_EVERYONE: 6,
  READ_HISTORY: 7,
  SEND_TTS: 8,
  BYPASS_SLOWMODE: 9,
  MANAGE_MESSAGES: 16,
  PIN_MESSAGES: 17,
  MANAGE_CHANNELS: 18,
  MANAGE_ROLES: 19,
  MANAGE_EMOJI: 20,
  MANAGE_WEBHOOKS: 21,
  MANAGE_GUILD: 22,
  VIEW_AUDIT_LOG: 23,
  CREATE_INVITES: 24,
  CHANGE_NICKNAME: 25,
  MANAGE_NICKNAMES: 26,
  VIEW_CHANNEL_MEMBERS: 27,
  KICK_MEMBERS: 32,
  BAN_MEMBERS: 33,
  MODERATE_MEMBERS: 34,
  CONNECT: 40,
  SPEAK: 41,
  STREAM: 42,
  VIDEO: 43,
  USE_VAD: 44,
  PRIORITY_SPEAKER: 45,
  MUTE_MEMBERS: 46,
  DEAFEN_MEMBERS: 47,
  MOVE_MEMBERS: 48,
  USE_SOUNDBOARD: 49,
  ADMINISTRATOR: 63,
} as const;

export type PermissionName = keyof typeof PERMISSION_BITS;

export const ALL_PERMISSIONS = ~0n & ((1n << 64n) - 1n);

export function perm(name: PermissionName): bigint {
  return 1n << BigInt(PERMISSION_BITS[name]);
}

export function toBits(raw: string | undefined): bigint {
  if (!raw) return 0n;
  try {
    return BigInt(raw);
  } catch {
    return 0n;
  }
}

export function has(bits: bigint, name: PermissionName): boolean {
  const p = perm(name);
  return (bits & p) === p;
}

export type PermissionGroup = "general" | "text" | "voice" | "moderation";

export const PERMISSION_GROUPS: Record<PermissionGroup, PermissionName[]> = {
  general: [
    "VIEW_CHANNEL",
    "VIEW_CHANNEL_MEMBERS",
    "CREATE_INVITES",
    "CHANGE_NICKNAME",
    "MANAGE_CHANNELS",
    "MANAGE_ROLES",
    "MANAGE_GUILD",
    "VIEW_AUDIT_LOG",
    "ADMINISTRATOR",
  ],
  text: [
    "SEND_MESSAGES",
    "READ_HISTORY",
    "EMBED_LINKS",
    "ATTACH_FILES",
    "ADD_REACTIONS",
    "USE_EXTERNAL_EMOJI",
    "MENTION_EVERYONE",
    "MANAGE_MESSAGES",
    "PIN_MESSAGES",
    "BYPASS_SLOWMODE",
  ],
  voice: [
    "CONNECT",
    "SPEAK",
    "STREAM",
    "VIDEO",
    "USE_VAD",
    "PRIORITY_SPEAKER",
    "USE_SOUNDBOARD",
    "MUTE_MEMBERS",
    "DEAFEN_MEMBERS",
    "MOVE_MEMBERS",
  ],
  moderation: ["KICK_MEMBERS", "BAN_MEMBERS", "MODERATE_MEMBERS", "MANAGE_NICKNAMES"],
};

function overwriteFor(
  overwrites: OmnidiscOverwrite[] | undefined,
  targetId: string,
  kind: "role" | "member",
): { allow: bigint; deny: bigint } | null {
  const found = overwrites?.find((o) => o.targetId === targetId && o.targetKind === kind);
  if (!found) return null;
  return { allow: toBits(found.allow), deny: toBits(found.deny) };
}

function applyTier(
  base: bigint,
  overwrites: OmnidiscOverwrite[] | undefined,
  everyoneRoleId: string | null,
  roleIds: string[],
  userId: string,
): bigint {
  let p = base;
  if (everyoneRoleId) {
    const everyone = overwriteFor(overwrites, everyoneRoleId, "role");
    if (everyone) p = (p & ~everyone.deny) | everyone.allow;
  }
  let allow = 0n;
  let deny = 0n;
  for (const roleId of roleIds) {
    if (roleId === everyoneRoleId) continue;
    const o = overwriteFor(overwrites, roleId, "role");
    if (!o) continue;
    allow |= o.allow;
    deny |= o.deny;
  }
  p = (p & ~deny) | allow;
  const member = overwriteFor(overwrites, userId, "member");
  if (member) p = (p & ~member.deny) | member.allow;
  return p;
}

export interface PermissionContext {
  guild: OmnidiscGuild | null;
  channel: OmnidiscChannel | null;
  parent?: OmnidiscChannel | null;
  member: OmnidiscMember | null;
  userId: string | null;
  /// Whether the user belongs to this guild at all. Defaults to `member !== null`,
  /// which is what a caller with only a member list can know. The store passes it
  /// explicitly because membership and the member *row* arrive from different
  /// places: the row is paged (op 14 asks for ranks 0-99), so a member of a large
  /// guild can be missing from it for a long time and must not be treated as an
  /// outsider.
  isMember?: boolean;
}

/// Mirrors the server's MUTED_DENY: what a timed-out member loses until the
/// timeout expires. Kept in sync by hand with `permissions.rs` on the server.
export const MUTED_DENY =
  perm("SEND_MESSAGES") | perm("ADD_REACTIONS") | perm("SPEAK") | perm("STREAM") | perm("VIDEO");

export function isMuted(member: OmnidiscMember | null, now = Date.now()): boolean {
  if (!member?.mutedUntil) return false;
  const until = Date.parse(member.mutedUntil);
  return Number.isFinite(until) && until > now;
}

export function resolvePermissions(ctx: PermissionContext): bigint {
  const { guild, channel, parent, member, userId } = ctx;
  if (!guild || !userId) return 0n;
  if (guild.ownerId === userId) return ALL_PERMISSIONS;
  // The server answers NONE for someone who is not a member of the guild.
  // Falling through to @everyone here is what makes the UI offer actions that
  // then come back 403.
  if (!(ctx.isMember ?? member !== null)) return 0n;
  const everyoneRole = guild.roles.find((r) => r.isEveryone) ?? null;
  const roleIds = member?.roleIds ?? [];
  let base = toBits(everyoneRole?.permissions);
  for (const roleId of roleIds) {
    const role = guild.roles.find((r) => r.id === roleId);
    if (role) base |= toBits(role.permissions);
  }
  if (has(base, "ADMINISTRATOR")) return ALL_PERMISSIONS;
  const everyoneId = everyoneRole?.id ?? null;
  let p = base;
  if (parent) p = applyTier(p, parent.overwrites, everyoneId, roleIds, userId);
  if (channel) p = applyTier(p, channel.overwrites, everyoneId, roleIds, userId);
  if (!has(p, "ADMINISTRATOR") && isMuted(member)) p &= ~MUTED_DENY;
  return p;
}

export function canIn(ctx: PermissionContext, name: PermissionName): boolean {
  return has(resolvePermissions(ctx), name);
}
