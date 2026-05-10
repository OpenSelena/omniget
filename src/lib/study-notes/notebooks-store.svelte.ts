import {
  notesNotebooksList,
  notesNotebooksActiveGet,
  notesNotebooksActiveSet,
  notesNotebooksCreate,
  notesNotebooksRename,
  notesNotebooksClose,
  notesNotebooksReopen,
  notesNotebooksDelete,
  notesNotebooksMovePage,
  notesNotebooksSetCover,
  notesNotebooksSetColor,
  notesNotebooksSetIcon,
  notesNotebooksReorder,
  type Notebook,
  type NotebookDeleteReport,
  type NotebookMoveReport,
} from "$lib/notes-bridge";

export const NOTEBOOK_LUCIDE_OPTIONS = [
  "book",
  "book-open",
  "graduation-cap",
  "briefcase",
  "lightbulb",
  "flask-conical",
  "code",
  "music",
  "feather",
  "compass",
  "sparkles",
  "folder",
] as const;

export const NOTEBOOK_COLOR_SWATCHES = [
  "var(--accent)",
  "#ef4444",
  "#f97316",
  "#eab308",
  "#22c55e",
  "#14b8a6",
  "#3b82f6",
  "#a855f7",
] as const;

class NotebooksStore {
  list = $state<Notebook[]>([]);
  activeId = $state<number>(1);
  hydrated = $state(false);
  loading = $state(false);

  visible = $derived(this.list.filter((n) => !n.closed));
  closed = $derived(this.list.filter((n) => n.closed));
  active = $derived(this.list.find((n) => n.id === this.activeId) ?? null);

  async hydrate() {
    if (this.loading) return;
    this.loading = true;
    try {
      const [rows, active] = await Promise.all([
        notesNotebooksList({ includeClosed: true }),
        notesNotebooksActiveGet(),
      ]);
      this.list = rows;
      this.activeId = active.notebook_id;
    } catch (e) {
      console.warn("notebooksStore.hydrate failed", e);
    } finally {
      this.hydrated = true;
      this.loading = false;
    }
  }

  async refresh() {
    try {
      this.list = await notesNotebooksList({ includeClosed: true });
    } catch (e) {
      console.warn("notebooksStore.refresh failed", e);
    }
  }

  pageCountOf(id: number): number {
    return this.list.find((n) => n.id === id)?.page_count ?? 0;
  }

  byId(id: number): Notebook | null {
    return this.list.find((n) => n.id === id) ?? null;
  }

  async setActive(notebookId: number) {
    if (this.activeId === notebookId) return;
    this.activeId = notebookId;
    try {
      await notesNotebooksActiveSet(notebookId);
    } catch (e) {
      console.warn("setActive failed", e);
    }
  }

  async create(args: {
    name: string;
    color?: string | null;
    iconLucide?: string | null;
  }): Promise<number | null> {
    try {
      const r = await notesNotebooksCreate(args);
      await this.refresh();
      return r.notebook_id;
    } catch (e) {
      console.warn("create notebook failed", e);
      return null;
    }
  }

  async rename(notebookId: number, newName: string) {
    try {
      await notesNotebooksRename({ notebookId, newName });
      await this.refresh();
    } catch (e) {
      console.warn("rename failed", e);
    }
  }

  async close(notebookId: number) {
    try {
      await notesNotebooksClose(notebookId);
      if (this.activeId === notebookId) {
        await this.setActive(1);
      }
      await this.refresh();
    } catch (e) {
      console.warn("close failed", e);
    }
  }

  async reopen(notebookId: number) {
    try {
      await notesNotebooksReopen(notebookId);
      await this.refresh();
    } catch (e) {
      console.warn("reopen failed", e);
    }
  }

  async delete(notebookId: number, force = false): Promise<NotebookDeleteReport> {
    try {
      const r = await notesNotebooksDelete({ notebookId, force });
      if (r.deleted && this.activeId === notebookId) {
        await this.setActive(1);
      }
      await this.refresh();
      return r;
    } catch (e) {
      console.warn("delete failed", e);
      return { deleted: false, page_count: 0 };
    }
  }

  async movePage(args: {
    pageId: number;
    targetNotebookId: number;
    targetName?: string;
  }): Promise<NotebookMoveReport | null> {
    try {
      const r = await notesNotebooksMovePage(args);
      await this.refresh();
      return r;
    } catch (e) {
      console.warn("movePage failed", e);
      return null;
    }
  }

  async setCover(notebookId: number, assetId: number | null) {
    try {
      await notesNotebooksSetCover({ notebookId, assetId });
      await this.refresh();
    } catch (e) {
      console.warn("setCover failed", e);
    }
  }

  async setColor(notebookId: number, color: string | null) {
    try {
      await notesNotebooksSetColor({ notebookId, color });
      await this.refresh();
    } catch (e) {
      console.warn("setColor failed", e);
    }
  }

  async setIcon(notebookId: number, iconLucide: string | null) {
    try {
      await notesNotebooksSetIcon({ notebookId, iconLucide });
      await this.refresh();
    } catch (e) {
      console.warn("setIcon failed", e);
    }
  }

  async reorder(notebookId: number, newSortIdx: number) {
    try {
      await notesNotebooksReorder({ notebookId, newSortIdx });
      await this.refresh();
    } catch (e) {
      console.warn("reorder failed", e);
    }
  }

  switchToIndex(zeroBased: number): boolean {
    const target = this.visible[zeroBased];
    if (!target) return false;
    void this.setActive(target.id);
    return true;
  }
}

export const notebooksStore = new NotebooksStore();
