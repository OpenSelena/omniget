import { pluginInvoke } from "$lib/plugin-invoke";

export type Playlist = {
  id: number;
  name: string;
  description: string | null;
  cover_path: string | null;
  created_at: number;
  updated_at: number;
  track_count: number;
  resolved_cover: string | null;
};

class PlaylistsStore {
  list = $state<Playlist[]>([]);
  loaded = $state(false);
  loading = $state(false);

  async load(force = false) {
    if (this.loaded && !force) return;
    this.loading = true;
    try {
      const res = await pluginInvoke<{ playlists: Playlist[] }>(
        "study",
        "study:music:playlists:list",
      );
      this.list = res.playlists ?? [];
      this.loaded = true;
    } catch (e) {
      if (import.meta.env.DEV) {
        console.warn("[music] playlists load failed", e);
      }
    } finally {
      this.loading = false;
    }
  }

  async create(name: string, description?: string): Promise<number | null> {
    try {
      const res = await pluginInvoke<{ id: number }>(
        "study",
        "study:music:playlists:create",
        { name, description: description ?? null },
      );
      await this.load(true);
      return res.id;
    } catch (e) {
      throw e;
    }
  }

  async deleteOne(id: number) {
    await pluginInvoke("study", "study:music:playlists:delete", { id });
    await this.load(true);
  }

  async addTrack(playlistId: number, trackId: number): Promise<{ duplicate: boolean }> {
    const res = await pluginInvoke<{ duplicate?: boolean }>(
      "study",
      "study:music:playlists:add_track",
      { playlistId, trackId },
    );
    await this.load(true);
    return { duplicate: !!res.duplicate };
  }

  async removeTrack(playlistId: number, trackId: number) {
    await pluginInvoke("study", "study:music:playlists:remove_track", {
      playlistId,
      trackId,
    });
    await this.load(true);
  }

  async reorder(playlistId: number, trackIds: number[]) {
    await pluginInvoke("study", "study:music:playlists:reorder", {
      playlistId,
      trackIds,
    });
    await this.load(true);
  }

  async update(playlistId: number, name?: string, description?: string | null) {
    const args: Record<string, unknown> = { id: playlistId };
    if (name !== undefined) args.name = name;
    if (description !== undefined) args.description = description;
    await pluginInvoke("study", "study:music:playlists:update", args);
    await this.load(true);
  }
}

export const playlistsStore = new PlaylistsStore();
