import type { MusicTrack } from "./player-store.svelte";

export type ContextMenuState = {
  open: boolean;
  x: number;
  y: number;
  track: MusicTrack | null;
};

export type RightBarTab = "queue" | "lyrics" | "info";

class MusicUI {
  addToPlaylistTrack = $state<MusicTrack | null>(null);
  rootsOpen = $state(false);
  equalizerOpen = $state(false);
  lastfmOpen = $state(false);
  rightbarTab = $state<RightBarTab | null>(null);
  selectedIds = $state<Set<number>>(new Set());
  selectionAnchor = $state<{ trackId: number; queue: MusicTrack[] } | null>(null);
  contextMenu = $state<ContextMenuState>({ open: false, x: 0, y: 0, track: null });

  get queueOpen(): boolean {
    return this.rightbarTab === "queue";
  }
  get lyricsOpen(): boolean {
    return this.rightbarTab === "lyrics";
  }

  openAddToPlaylist(track: MusicTrack) {
    this.addToPlaylistTrack = track;
  }
  closeAddToPlaylist() {
    this.addToPlaylistTrack = null;
  }
  toggleQueue() {
    this.rightbarTab = this.rightbarTab === "queue" ? null : "queue";
  }
  closeQueue() {
    if (this.rightbarTab === "queue") this.rightbarTab = null;
  }
  toggleLyrics() {
    this.rightbarTab = this.rightbarTab === "lyrics" ? null : "lyrics";
  }
  closeLyrics() {
    if (this.rightbarTab === "lyrics") this.rightbarTab = null;
  }
  toggleInfo() {
    this.rightbarTab = this.rightbarTab === "info" ? null : "info";
  }
  closeRightBar() {
    this.rightbarTab = null;
  }
  openRoots() {
    this.rootsOpen = true;
  }
  closeRoots() {
    this.rootsOpen = false;
  }
  toggleEqualizer() {
    this.equalizerOpen = !this.equalizerOpen;
  }
  closeEqualizer() {
    this.equalizerOpen = false;
  }
  openLastFm() {
    this.lastfmOpen = true;
  }
  closeLastFm() {
    this.lastfmOpen = false;
  }
  themeOpen = $state(false);
  openTheme() {
    this.themeOpen = true;
  }
  closeTheme() {
    this.themeOpen = false;
  }
  discordOpen = $state(false);
  openDiscord() {
    this.discordOpen = true;
  }
  closeDiscord() {
    this.discordOpen = false;
  }
  youtubeOpen = $state(false);
  openYoutube() {
    this.youtubeOpen = true;
  }
  closeYoutube() {
    this.youtubeOpen = false;
  }
  qualityOpen = $state(false);
  openQuality() {
    this.qualityOpen = true;
  }
  closeQuality() {
    this.qualityOpen = false;
  }
  sourcesOpen = $state(false);
  openSources() {
    this.sourcesOpen = true;
  }
  closeSources() {
    this.sourcesOpen = false;
  }
  qobuzOpen = $state(false);
  openQobuz() {
    this.qobuzOpen = true;
  }
  closeQobuz() {
    this.qobuzOpen = false;
  }
  openContextMenu(track: MusicTrack, x: number, y: number) {
    this.contextMenu = { open: true, x, y, track };
  }
  closeContextMenu() {
    this.contextMenu = { open: false, x: 0, y: 0, track: null };
  }
  isSelected(trackId: number): boolean {
    return this.selectedIds.has(trackId);
  }
  toggleSelected(trackId: number, queue?: MusicTrack[]) {
    const next = new Set(this.selectedIds);
    if (next.has(trackId)) {
      next.delete(trackId);
    } else {
      next.add(trackId);
    }
    this.selectedIds = next;
    if (next.size > 0 && queue) {
      this.selectionAnchor = { trackId, queue };
    }
    if (next.size === 0) {
      this.selectionAnchor = null;
    }
  }
  selectRange(trackId: number, queue: MusicTrack[]) {
    if (!this.selectionAnchor || this.selectionAnchor.queue !== queue) {
      this.selectedIds = new Set([trackId]);
      this.selectionAnchor = { trackId, queue };
      return;
    }
    const startIdx = queue.findIndex((t) => t.id === this.selectionAnchor!.trackId);
    const endIdx = queue.findIndex((t) => t.id === trackId);
    if (startIdx < 0 || endIdx < 0) return;
    const lo = Math.min(startIdx, endIdx);
    const hi = Math.max(startIdx, endIdx);
    const next = new Set<number>(this.selectedIds);
    for (let i = lo; i <= hi; i++) next.add(queue[i].id);
    this.selectedIds = next;
  }
  selectOnly(trackId: number, queue?: MusicTrack[]) {
    this.selectedIds = new Set([trackId]);
    if (queue) this.selectionAnchor = { trackId, queue };
  }
  clearSelection() {
    this.selectedIds = new Set();
    this.selectionAnchor = null;
  }
  selectedCount(): number {
    return this.selectedIds.size;
  }
}

export const musicUI = new MusicUI();
