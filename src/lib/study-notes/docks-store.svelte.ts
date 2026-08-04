import {
  notesDocksState,
  notesDocksSetState,
  notesDocksToggleVisible,
  type DockId,
  type DockPosition,
  type DockState,
  type DockStatePatch,
} from "$lib/notes-bridge";

export type DockMeta = {
  id: DockId;
  label: string;
  iconKey: DockIconKey;
  defaultPosition: DockPosition;
};

export type DockIconKey =
  | "files"
  | "list-tree"
  | "link"
  | "bookmark"
  | "tags"
  | "inbox"
  | "share-2";

export const DOCK_META: DockMeta[] = [
  { id: "files", label: "Arquivos", iconKey: "files", defaultPosition: "left" },
  { id: "outline", label: "Outline", iconKey: "list-tree", defaultPosition: "left" },
  { id: "backlink", label: "Backlinks", iconKey: "link", defaultPosition: "right" },
  { id: "bookmark", label: "Favoritos", iconKey: "bookmark", defaultPosition: "right" },
  { id: "tag", label: "Tags", iconKey: "tags", defaultPosition: "right" },
  { id: "inbox", label: "Inbox", iconKey: "inbox", defaultPosition: "right" },
  { id: "graph", label: "Graph", iconKey: "share-2", defaultPosition: "right" },
];

class DocksStore {
  states = $state<Record<string, DockState>>({});
  hydrated = $state(false);

  async hydrate() {
    try {
      const rows = await notesDocksState();
      const map: Record<string, DockState> = {};
      for (const row of rows) map[row.dock_id] = row;
      this.states = map;
    } catch {
      /* ignore — UI falls back to defaults */
    }
    this.hydrated = true;
  }

  byPosition(position: DockPosition): DockState[] {
    return Object.values(this.states)
      .filter((d) => d.position === position)
      .sort((a, b) => a.sort_idx - b.sort_idx);
  }

  visibleByPosition(position: DockPosition): DockState[] {
    return this.byPosition(position).filter((d) => d.visible);
  }

  isVisible(id: DockId): boolean {
    return this.states[id]?.visible ?? false;
  }

  positionOf(id: DockId): DockPosition {
    return this.states[id]?.position ?? "left";
  }

  widthOf(id: DockId): number | null {
    return this.states[id]?.width_px ?? null;
  }

  async toggle(id: DockId) {
    const prev = this.states[id]?.visible ?? false;
    if (this.states[id]) {
      this.states = { ...this.states, [id]: { ...this.states[id], visible: !prev } };
    }
    try {
      const r = await notesDocksToggleVisible(id);
      if (this.states[id]) {
        this.states = { ...this.states, [id]: { ...this.states[id], visible: r.visible } };
      }
    } catch {
      if (this.states[id]) {
        this.states = { ...this.states, [id]: { ...this.states[id], visible: prev } };
      }
    }
  }

  async setState(id: DockId, patch: DockStatePatch) {
    const prev = this.states[id];
    if (prev) {
      this.states = { ...this.states, [id]: { ...prev, ...patch } as DockState };
    }
    try {
      const r = await notesDocksSetState({ dockId: id, patch });
      this.states = { ...this.states, [id]: r.state };
    } catch {
      if (prev) this.states = { ...this.states, [id]: prev };
    }
  }

  async setWidth(id: DockId, widthPx: number) {
    await this.setState(id, { width_px: widthPx });
  }
}

export const docksStore = new DocksStore();
