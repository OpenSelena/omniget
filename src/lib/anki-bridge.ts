import { pluginInvoke } from "$lib/plugin-invoke";

export type AnkiPet = {
  slug: string;
  display_name: string;
  description: string;
  spritesheet_url: string;
  vibes: string[];
  tags: string[];
};

export type AnkiStreak = {
  current: number;
  longest: number;
  reviewed_today: number;
  last_review_secs: number;
  milestones: Array<{ days: number; label: string; achieved: boolean }>;
};

export type AnkiPendingCounts = {
  notes: number;
  cards: number;
  revlog: number;
  decks: number;
  deck_config: number;
  notetypes: number;
  tags: number;
  graves: number;
  total: number;
};

export type AnkiDeckTreeNode = {
  deck_id: number;
  name: string;
  full_name: string;
  level: number;
  collapsed: boolean;
  filtered: boolean;
  children: AnkiDeckTreeNode[];
};

export type AnkiDeckSummary = {
  deck_id: number;
  name: string;
  full_name: string;
  level: number;
  due: number;
  new_count: number;
  learning: number;
  total: number;
};

export type AnkiDashboardState = {
  deck_tree: AnkiDeckTreeNode[];
  deck_summary: AnkiDeckSummary[];
  due_today_total: number;
  new_today_total: number;
  learning_total: number;
  reviewed_today: number;
  recent_reviews: number;
  streak: AnkiStreak;
  pet: AnkiPet;
  pending: AnkiPendingCounts;
  motd: string;
};

export type ProviderConfig =
  | { kind: "local_folder"; path: string }
  | { kind: "webdav"; url: string; username: string; password: string }
  | { kind: "colpkg"; dir: string }
  | { kind: "none" };

export type ProviderInfo = {
  provider: ProviderConfig;
  kind: "local_folder" | "webdav" | "colpkg" | "none";
  display: string;
  last_sync_secs: number;
};

export type SyncOutcome = {
  action: string;
  remote_manifest: {
    schema_version: number;
    collection_sha256: string;
    collection_size: number;
    media_sha256: string;
    media_size: number;
    uploaded_at: number;
    source_device: string;
  } | null;
  bytes_uploaded: number;
  bytes_downloaded: number;
  message: string;
};

export function ankiOpen(): Promise<unknown> {
  return pluginInvoke("study", "study:anki:storage:open");
}

export function ankiDashboard(): Promise<AnkiDashboardState> {
  return pluginInvoke<AnkiDashboardState>("study", "study:anki:dashboard:state");
}

export function ankiPetsList(): Promise<AnkiPet[]> {
  return pluginInvoke<AnkiPet[]>("study", "study:anki:pets:list");
}

export function ankiPetActive(): Promise<AnkiPet> {
  return pluginInvoke<AnkiPet>("study", "study:anki:pets:active");
}

export function ankiPetSetActive(slug: string): Promise<AnkiPet> {
  return pluginInvoke<AnkiPet>("study", "study:anki:pets:set_active", { slug });
}

export function ankiStreak(): Promise<AnkiStreak> {
  return pluginInvoke<AnkiStreak>("study", "study:anki:streak:current");
}

export function ankiSyncProviderGet(): Promise<ProviderInfo> {
  return pluginInvoke<ProviderInfo>("study", "study:anki:sync:provider_get");
}

export function ankiSyncProviderSave(
  provider: ProviderConfig,
): Promise<{ kind: string; display: string }> {
  return pluginInvoke<{ kind: string; display: string }>(
    "study",
    "study:anki:sync:provider_save",
    { provider },
  );
}

export function ankiSyncProviderTest(provider: ProviderConfig): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>("study", "study:anki:sync:provider_test", { provider });
}

export function ankiSyncRun(): Promise<SyncOutcome> {
  return pluginInvoke<SyncOutcome>("study", "study:anki:sync:run");
}

export function ankiPending(): Promise<AnkiPendingCounts> {
  return pluginInvoke<AnkiPendingCounts>("study", "study:anki:sync:pending_count");
}
