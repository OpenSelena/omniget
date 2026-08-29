import { describe, expect, it } from "vitest";
import {
  ALL_PERMISSIONS,
  canIn,
  has,
  perm,
  resolvePermissions,
  toBits,
  type PermissionContext,
} from "./permissions";
import type { OmnidiscChannel, OmnidiscGuild, OmnidiscMember, OmnidiscRole } from "./types";

const EVERYONE: OmnidiscRole = {
  id: "r-everyone",
  name: "everyone",
  permissions: (perm("VIEW_CHANNEL") | perm("SEND_MESSAGES") | perm("READ_HISTORY")).toString(),
  position: 0,
  hoist: false,
  mentionable: false,
  isEveryone: true,
};

const MOD: OmnidiscRole = {
  id: "r-mod",
  name: "Moderators",
  permissions: perm("MANAGE_MESSAGES").toString(),
  position: 1,
  hoist: true,
  mentionable: true,
  isEveryone: false,
};

const ADMIN: OmnidiscRole = {
  id: "r-admin",
  name: "Admins",
  permissions: perm("ADMINISTRATOR").toString(),
  position: 2,
  hoist: true,
  mentionable: false,
  isEveryone: false,
};

function guild(roles: OmnidiscRole[] = [EVERYONE, MOD, ADMIN], ownerId = "owner"): OmnidiscGuild {
  return { id: "g1", instanceId: "i1", name: "Guild", ownerId, channels: [], roles };
}

function channel(overwrites: OmnidiscChannel["overwrites"] = [], parentId?: string): OmnidiscChannel {
  return { id: "c1", name: "general", kind: "text", position: 0, guildId: "g1", parentId, overwrites };
}

function member(roleIds: string[]): OmnidiscMember {
  return { id: "u1", name: "User", online: true, roleIds };
}

function ctx(over: Partial<PermissionContext> = {}): PermissionContext {
  return {
    guild: guild(),
    channel: channel(),
    parent: null,
    member: member([]),
    userId: "u1",
    ...over,
  };
}

describe("omnidisc permission bits", () => {
  it("parses a decimal string and falls back to zero on junk", () => {
    expect(toBits("3")).toBe(3n);
    expect(toBits(undefined)).toBe(0n);
    expect(toBits("not a number")).toBe(0n);
  });

  it("has() needs every bit of the permission", () => {
    expect(has(perm("SEND_MESSAGES"), "SEND_MESSAGES")).toBe(true);
    expect(has(perm("SEND_MESSAGES"), "MANAGE_MESSAGES")).toBe(false);
  });
});

describe("omnidisc permission resolution", () => {
  it("gives the owner everything, whatever the overwrites say", () => {
    const denied = [{ targetId: "r-everyone", targetKind: "role" as const, allow: "0", deny: perm("VIEW_CHANNEL").toString() }];
    const resolved = resolvePermissions(ctx({ userId: "owner", channel: channel(denied) }));
    expect(resolved).toBe(ALL_PERMISSIONS);
  });

  it("a non-member of any guild gets nothing", () => {
    expect(resolvePermissions(ctx({ guild: null }))).toBe(0n);
    expect(resolvePermissions(ctx({ userId: null }))).toBe(0n);
  });

  it("starts from @everyone and adds the member's roles", () => {
    expect(canIn(ctx(), "SEND_MESSAGES")).toBe(true);
    expect(canIn(ctx(), "MANAGE_MESSAGES")).toBe(false);
    expect(canIn(ctx({ member: member(["r-mod"]) }), "MANAGE_MESSAGES")).toBe(true);
  });

  it("an administrator role ignores channel denials", () => {
    const denied = [{ targetId: "r-everyone", targetKind: "role" as const, allow: "0", deny: perm("VIEW_CHANNEL").toString() }];
    expect(canIn(ctx({ member: member(["r-admin"]), channel: channel(denied) }), "VIEW_CHANNEL")).toBe(true);
  });

  it("a member overwrite beats a role denial in the same channel", () => {
    const overwrites = [
      { targetId: "r-mod", targetKind: "role" as const, allow: "0", deny: perm("SEND_MESSAGES").toString() },
      { targetId: "u1", targetKind: "member" as const, allow: perm("SEND_MESSAGES").toString(), deny: "0" },
    ];
    expect(canIn(ctx({ member: member(["r-mod"]), channel: channel(overwrites) }), "SEND_MESSAGES")).toBe(true);
  });

  it("someone with no membership record gets nothing, like the server", () => {
    expect(resolvePermissions(ctx({ member: null }))).toBe(0n);
    expect(canIn(ctx({ member: null }), "VIEW_CHANNEL")).toBe(false);
    expect(canIn(ctx({ member: null }), "SEND_MESSAGES")).toBe(false);
    expect(canIn(ctx({ member: member([]), isMember: false }), "SEND_MESSAGES")).toBe(false);
    // The owner is still the owner even before their member record arrives.
    expect(resolvePermissions(ctx({ member: null, userId: "owner" }))).toBe(ALL_PERMISSIONS);
    // A member whose paged row has not arrived yet is still a member.
    expect(canIn(ctx({ member: null, isMember: true }), "SEND_MESSAGES")).toBe(true);
  });

  it("a timed-out member loses exactly what the server takes away", () => {
    const timedOut: OmnidiscMember = {
      ...member([]),
      mutedUntil: new Date(Date.now() + 60_000).toISOString(),
    };
    expect(canIn(ctx({ member: timedOut }), "SEND_MESSAGES")).toBe(false);
    expect(canIn(ctx({ member: timedOut }), "ADD_REACTIONS")).toBe(false);
    expect(canIn(ctx({ member: timedOut }), "SPEAK")).toBe(false);
    expect(canIn(ctx({ member: timedOut }), "READ_HISTORY")).toBe(true);
    expect(canIn(ctx({ member: timedOut }), "VIEW_CHANNEL")).toBe(true);
  });

  it("an expired timeout, and an admin's timeout, change nothing", () => {
    const expired: OmnidiscMember = {
      ...member([]),
      mutedUntil: new Date(Date.now() - 60_000).toISOString(),
    };
    expect(canIn(ctx({ member: expired }), "SEND_MESSAGES")).toBe(true);
    const admin: OmnidiscMember = {
      ...member(["r-admin"]),
      mutedUntil: new Date(Date.now() + 60_000).toISOString(),
    };
    expect(canIn(ctx({ member: admin }), "SEND_MESSAGES")).toBe(true);
    const junk: OmnidiscMember = { ...member([]), mutedUntil: "whenever" };
    expect(canIn(ctx({ member: junk }), "SEND_MESSAGES")).toBe(true);
  });

  it("the channel tier overrides the category tier", () => {
    const parent = channel([
      { targetId: "r-everyone", targetKind: "role", allow: "0", deny: perm("VIEW_CHANNEL").toString() },
    ]);
    const child = channel([
      { targetId: "r-everyone", targetKind: "role", allow: perm("VIEW_CHANNEL").toString(), deny: "0" },
    ]);
    expect(canIn(ctx({ parent, channel: child }), "VIEW_CHANNEL")).toBe(true);
    expect(canIn(ctx({ parent, channel: channel() }), "VIEW_CHANNEL")).toBe(false);
  });
});
