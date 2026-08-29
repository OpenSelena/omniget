import type { OmnidiscGuild, OmnidiscMember, OmnidiscMessage } from "./types";

export const DEMO_INSTANCE_ID = "demo";
export const DEMO_INSTANCE_URL = "demo://local";

const AUTHORS = [
  { id: "u1", name: "Loop" },
  { id: "u2", name: "Ana" },
  { id: "u3", name: "Rafael" },
  { id: "u4", name: "Mei" },
  { id: "u5", name: "Kofi" },
];

const LINES = [
  "anyone up for a game later?",
  "just pushed the fix, can you pull and check?",
  "the new build scrolls way smoother on my old laptop",
  "lol",
  "I'll be there in 10",
  "can someone resend the link? I lost it",
  "ok that works",
  "this is a longer message so the list has to deal with rows of different heights, which is exactly what a virtualized list gets wrong if it assumes a fixed size for every item.",
  "gg",
  "brb",
  "what codec are we using for screen share again?",
  "4k120 is wild",
];

export function makeFixtureGuilds(instanceId: string): OmnidiscGuild[] {
  return [
    {
      id: "g-home",
      instanceId,
      name: "Home base",
      ownerId: "u1",
      roles: [],
      channels: [
        { id: "c-general", name: "general", kind: "text", category: "Text", position: 0, guildId: "g-home" },
        { id: "c-downloads", name: "downloads", kind: "text", category: "Text", position: 1, guildId: "g-home" },
        { id: "c-dev", name: "dev", kind: "text", category: "Text", position: 2, guildId: "g-home" },
        { id: "c-lounge", name: "Lounge", kind: "voice", category: "Voice", position: 3, guildId: "g-home" },
        { id: "c-games", name: "Games", kind: "voice", category: "Voice", position: 4, guildId: "g-home" },
      ],
    },
    {
      id: "g-study",
      instanceId,
      name: "Study group",
      ownerId: "u2",
      roles: [],
      channels: [
        { id: "c-notes", name: "notes", kind: "text", category: "Text", position: 0, guildId: "g-study" },
        { id: "c-focus", name: "Focus room", kind: "voice", category: "Voice", position: 1, guildId: "g-study" },
      ],
    },
  ];
}

export function makeFixtureMembers(): OmnidiscMember[] {
  return AUTHORS.map((a, i) => ({ id: a.id, name: a.name, online: i % 2 === 0, role: i === 0 ? "admin" : undefined }));
}

export function makeFixtureMessages(channelId: string, count: number, endAt: number, startSeq: number): OmnidiscMessage[] {
  const out: OmnidiscMessage[] = [];
  let ts = endAt;
  for (let i = count - 1; i >= 0; i--) {
    const seq = startSeq + i;
    const author = AUTHORS[(seq * 7) % AUTHORS.length];
    const gapMin = seq % 9 === 0 ? 8 * 60 : seq % 4 === 0 ? 7 : 1;
    out.unshift({
      id: `${channelId}-${seq}`,
      channelId,
      authorId: author.id,
      authorName: author.name,
      content: LINES[(seq * 5) % LINES.length],
      createdAt: ts,
      delivery: "sent",
    });
    ts -= gapMin * 60_000;
  }
  return out;
}
