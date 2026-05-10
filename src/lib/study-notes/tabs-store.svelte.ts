import {
  notesActiveStateGet,
  notesActiveStateSet,
  notesTabsListForWnd,
  notesTabsOpen,
  notesTabsClose,
  notesTabsReorder,
  notesTabsMoveToWnd,
  notesTabsPin,
  notesTabsTouchFocus,
  notesWndsTree,
  notesWndsSplit,
  notesWndsClose,
  notesWndsSetRatio,
  type TabSummary,
  type TabViewKind,
  type WndNode,
} from "$lib/notes-bridge";

const ACTIVE_WND_KEY = "active_wnd_id";
const ACTIVE_TABS_KEY = "active_tab_id_per_wnd";
const RECENTLY_CLOSED_LIMIT = 12;

type RecentlyClosedTab = {
  wndId: number;
  pageId: number | null;
  viewKind: TabViewKind;
};

class TabsStore {
  tree = $state<WndNode | null>(null);
  tabsByWnd = $state<Record<number, TabSummary[]>>({});
  activeWndId = $state<number | null>(null);
  activeTabIdByWnd = $state<Record<number, number | null>>({});
  recentlyClosed = $state<RecentlyClosedTab[]>([]);
  hydrated = $state(false);
  loadingTree = $state(false);

  collectLeafIds(node: WndNode | null, acc: number[] = []): number[] {
    if (!node) return acc;
    if (!node.split_dir) {
      acc.push(node.id);
      return acc;
    }
    for (const child of node.children) this.collectLeafIds(child, acc);
    return acc;
  }

  get leafIds(): number[] {
    return this.collectLeafIds(this.tree);
  }

  get activeTabId(): number | null {
    if (this.activeWndId == null) return null;
    return this.activeTabIdByWnd[this.activeWndId] ?? null;
  }

  get activeTab(): TabSummary | null {
    if (this.activeWndId == null) return null;
    const tabs = this.tabsByWnd[this.activeWndId] ?? [];
    const id = this.activeTabIdByWnd[this.activeWndId];
    return tabs.find((t) => t.id === id) ?? tabs[0] ?? null;
  }

  async hydrate() {
    if (this.hydrated || this.loadingTree) return;
    this.loadingTree = true;
    try {
      const tree = await notesWndsTree();
      this.tree = tree;
      const leafIds = this.collectLeafIds(tree);
      const tabsLists = await Promise.all(
        leafIds.map(async (id) => [id, await notesTabsListForWnd(id)] as const),
      );
      const next: Record<number, TabSummary[]> = {};
      for (const [id, list] of tabsLists) next[id] = list;
      this.tabsByWnd = next;

      const activeWndRes = await notesActiveStateGet(ACTIVE_WND_KEY);
      const activeWnd =
        typeof activeWndRes.value === "number" ? activeWndRes.value : null;
      this.activeWndId =
        activeWnd != null && leafIds.includes(activeWnd)
          ? activeWnd
          : (leafIds[0] ?? null);

      const activeTabsRes = await notesActiveStateGet(ACTIVE_TABS_KEY);
      const stored =
        activeTabsRes.value && typeof activeTabsRes.value === "object"
          ? (activeTabsRes.value as Record<string, number | null>)
          : {};
      const sanitized: Record<number, number | null> = {};
      for (const id of leafIds) {
        const raw = stored[String(id)];
        const tabs = next[id] ?? [];
        const exists =
          typeof raw === "number" && tabs.some((t) => t.id === raw);
        sanitized[id] = exists ? (raw as number) : (tabs[0]?.id ?? null);
      }
      this.activeTabIdByWnd = sanitized;
      this.hydrated = true;
    } finally {
      this.loadingTree = false;
    }
  }

  async refreshWnd(wndId: number) {
    const list = await notesTabsListForWnd(wndId);
    this.tabsByWnd = { ...this.tabsByWnd, [wndId]: list };
    const current = this.activeTabIdByWnd[wndId];
    if (current == null || !list.some((t) => t.id === current)) {
      this.setActiveTab(wndId, list[0]?.id ?? null);
    }
  }

  async refreshTree() {
    const tree = await notesWndsTree();
    this.tree = tree;
    const leafIds = this.collectLeafIds(tree);
    const next: Record<number, TabSummary[]> = {};
    for (const id of leafIds) {
      const existing = this.tabsByWnd[id];
      next[id] = existing ?? (await notesTabsListForWnd(id));
    }
    this.tabsByWnd = next;
    if (this.activeWndId == null || !leafIds.includes(this.activeWndId)) {
      this.activeWndId = leafIds[0] ?? null;
      void this.persistActiveWnd();
    }
    const sanitized: Record<number, number | null> = {};
    for (const id of leafIds) {
      const list = next[id];
      const cur = this.activeTabIdByWnd[id];
      sanitized[id] =
        cur != null && list.some((t) => t.id === cur)
          ? cur
          : (list[0]?.id ?? null);
    }
    this.activeTabIdByWnd = sanitized;
    void this.persistActiveTabs();
  }

  setActiveWnd(wndId: number) {
    if (!this.leafIds.includes(wndId)) return;
    this.activeWndId = wndId;
    void this.persistActiveWnd();
  }

  setActiveTab(wndId: number, tabId: number | null) {
    this.activeTabIdByWnd = { ...this.activeTabIdByWnd, [wndId]: tabId };
    void this.persistActiveTabs();
    if (tabId != null) void notesTabsTouchFocus(tabId).catch(() => {});
  }

  async persistActiveWnd() {
    try {
      await notesActiveStateSet({ key: ACTIVE_WND_KEY, value: this.activeWndId });
    } catch {
      /* ignore */
    }
  }

  async persistActiveTabs() {
    try {
      await notesActiveStateSet({
        key: ACTIVE_TABS_KEY,
        value: this.activeTabIdByWnd as unknown as object,
      });
    } catch {
      /* ignore */
    }
  }

  pushRecentlyClosed(entry: RecentlyClosedTab) {
    const next = [entry, ...this.recentlyClosed].slice(0, RECENTLY_CLOSED_LIMIT);
    this.recentlyClosed = next;
  }

  popRecentlyClosed(): RecentlyClosedTab | null {
    if (this.recentlyClosed.length === 0) return null;
    const [head, ...rest] = this.recentlyClosed;
    this.recentlyClosed = rest;
    return head;
  }

  async openTab(args: {
    wndId: number;
    pageId?: number | null;
    viewKind?: TabViewKind;
    activate?: boolean;
  }): Promise<number> {
    const res = await notesTabsOpen({
      wndId: args.wndId,
      pageId: args.pageId,
      viewKind: args.viewKind,
    });
    await this.refreshWnd(args.wndId);
    if (args.activate !== false) {
      this.setActiveWnd(args.wndId);
      this.setActiveTab(args.wndId, res.tab_id);
    }
    return res.tab_id;
  }

  async closeTab(tabId: number) {
    const wndId = this.findWndOfTab(tabId);
    const tab = wndId != null ? this.findTab(wndId, tabId) : null;
    if (tab) {
      this.pushRecentlyClosed({
        wndId: tab.wnd_id,
        pageId: tab.page_id,
        viewKind: tab.view_kind,
      });
    }
    const report = await notesTabsClose(tabId);
    if (wndId != null) {
      await this.refreshWnd(wndId);
      if (report.fallback_tab_id != null) {
        this.setActiveTab(wndId, report.fallback_tab_id);
      }
    }
  }

  async closeOtherTabsInWnd(wndId: number, keepTabId: number) {
    const tabs = this.tabsByWnd[wndId] ?? [];
    for (const t of tabs) {
      if (t.id !== keepTabId && !t.pinned) {
        await notesTabsClose(t.id);
      }
    }
    await this.refreshWnd(wndId);
    this.setActiveTab(wndId, keepTabId);
  }

  async closeTabsToRight(wndId: number, anchorTabId: number) {
    const tabs = (this.tabsByWnd[wndId] ?? []).slice().sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
      return a.sort_idx - b.sort_idx;
    });
    const idx = tabs.findIndex((t) => t.id === anchorTabId);
    if (idx === -1) return;
    for (let i = idx + 1; i < tabs.length; i++) {
      const t = tabs[i];
      if (!t.pinned) await notesTabsClose(t.id);
    }
    await this.refreshWnd(wndId);
  }

  async pinTab(tabId: number, pinned: boolean) {
    await notesTabsPin({ tabId, pinned });
    const wndId = this.findWndOfTab(tabId);
    if (wndId != null) await this.refreshWnd(wndId);
  }

  async reorderTabs(wndId: number, orderedIds: number[]) {
    for (let i = 0; i < orderedIds.length; i++) {
      await notesTabsReorder({ tabId: orderedIds[i], newSortIdx: i });
    }
    await this.refreshWnd(wndId);
  }

  async moveTabToWnd(tabId: number, targetWndId: number) {
    await notesTabsMoveToWnd({ tabId, targetWndId });
    const sourceWnd = this.findWndOfTab(tabId);
    if (sourceWnd != null && sourceWnd !== targetWndId) {
      await this.refreshWnd(sourceWnd);
    }
    await this.refreshWnd(targetWndId);
    this.setActiveWnd(targetWndId);
    this.setActiveTab(targetWndId, tabId);
  }

  async splitWnd(parentWndId: number, direction: "horizontal" | "vertical") {
    const res = await notesWndsSplit({ parentWndId, direction });
    await this.refreshTree();
    this.setActiveWnd(res.new_wnd_id);
    return res.new_wnd_id;
  }

  async closeWnd(wndId: number) {
    if (this.leafIds.length <= 1) return;
    await notesWndsClose(wndId);
    await this.refreshTree();
  }

  async setRatio(internalWndId: number, ratio: number) {
    await notesWndsSetRatio({ wndId: internalWndId, ratio });
    if (this.tree) {
      this.tree = updateRatio(this.tree, internalWndId, ratio);
    }
  }

  findTab(wndId: number, tabId: number): TabSummary | undefined {
    return (this.tabsByWnd[wndId] ?? []).find((t) => t.id === tabId);
  }

  findWndOfTab(tabId: number): number | null {
    for (const [wnd, tabs] of Object.entries(this.tabsByWnd)) {
      if (tabs.some((t) => t.id === tabId)) return Number(wnd);
    }
    return null;
  }

  cycleActiveTab(direction: 1 | -1) {
    if (this.activeWndId == null) return;
    const tabs = this.sortedTabsForWnd(this.activeWndId);
    if (tabs.length === 0) return;
    const cur = this.activeTabIdByWnd[this.activeWndId];
    const idx = tabs.findIndex((t) => t.id === cur);
    const nextIdx =
      idx === -1 ? 0 : (idx + direction + tabs.length) % tabs.length;
    this.setActiveTab(this.activeWndId, tabs[nextIdx].id);
  }

  activateTabIndex(index: number) {
    if (this.activeWndId == null) return;
    const tabs = this.sortedTabsForWnd(this.activeWndId);
    if (index < 0 || index >= tabs.length) return;
    this.setActiveTab(this.activeWndId, tabs[index].id);
  }

  sortedTabsForWnd(wndId: number): TabSummary[] {
    const tabs = (this.tabsByWnd[wndId] ?? []).slice();
    tabs.sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
      return a.sort_idx - b.sort_idx;
    });
    return tabs;
  }

  focusNeighborWnd(direction: "left" | "right" | "up" | "down") {
    if (this.activeWndId == null || this.tree == null) return;
    const target = neighborLeaf(this.tree, this.activeWndId, direction);
    if (target != null) this.setActiveWnd(target);
  }
}

function updateRatio(node: WndNode, id: number, ratio: number): WndNode {
  if (node.id === id) {
    return { ...node, ratio };
  }
  if (!node.split_dir) return node;
  return {
    ...node,
    children: node.children.map((c) => updateRatio(c, id, ratio)),
  };
}

type Rect = { x: number; y: number; w: number; h: number };

function computeRects(node: WndNode, rect: Rect, acc: Map<number, Rect>) {
  if (!node.split_dir) {
    acc.set(node.id, rect);
    return;
  }
  if (node.children.length !== 2) {
    for (const c of node.children) computeRects(c, rect, acc);
    return;
  }
  const ratio = node.ratio;
  if (node.split_dir === "vertical") {
    const w1 = rect.w * ratio;
    computeRects(node.children[0], { ...rect, w: w1 }, acc);
    computeRects(
      node.children[1],
      { x: rect.x + w1, y: rect.y, w: rect.w - w1, h: rect.h },
      acc,
    );
  } else {
    const h1 = rect.h * ratio;
    computeRects(node.children[0], { ...rect, h: h1 }, acc);
    computeRects(
      node.children[1],
      { x: rect.x, y: rect.y + h1, w: rect.w, h: rect.h - h1 },
      acc,
    );
  }
}

function neighborLeaf(
  tree: WndNode,
  fromId: number,
  direction: "left" | "right" | "up" | "down",
): number | null {
  const rects = new Map<number, Rect>();
  computeRects(tree, { x: 0, y: 0, w: 1, h: 1 }, rects);
  const from = rects.get(fromId);
  if (!from) return null;
  let best: { id: number; dist: number } | null = null;
  for (const [id, r] of rects) {
    if (id === fromId) continue;
    let aligned = false;
    let dist = Infinity;
    const cy1 = from.y + from.h / 2;
    const cy2 = r.y + r.h / 2;
    const cx1 = from.x + from.w / 2;
    const cx2 = r.x + r.w / 2;
    if (direction === "left" && r.x + r.w <= from.x + 1e-6) {
      aligned = !(cy2 + r.h / 2 < from.y || cy2 - r.h / 2 > from.y + from.h);
      dist = from.x - (r.x + r.w);
    } else if (direction === "right" && r.x >= from.x + from.w - 1e-6) {
      aligned = !(cy2 + r.h / 2 < from.y || cy2 - r.h / 2 > from.y + from.h);
      dist = r.x - (from.x + from.w);
    } else if (direction === "up" && r.y + r.h <= from.y + 1e-6) {
      aligned = !(cx2 + r.w / 2 < from.x || cx2 - r.w / 2 > from.x + from.w);
      dist = from.y - (r.y + r.h);
    } else if (direction === "down" && r.y >= from.y + from.h - 1e-6) {
      aligned = !(cx2 + r.w / 2 < from.x || cx2 - r.w / 2 > from.x + from.w);
      dist = r.y - (from.y + from.h);
    }
    if (!aligned) continue;
    const tieBreak =
      direction === "left" || direction === "right"
        ? Math.abs(cy1 - cy2)
        : Math.abs(cx1 - cx2);
    const score = dist + tieBreak * 0.001;
    if (best === null || score < best.dist) {
      best = { id, dist: score };
    }
  }
  return best?.id ?? null;
}

export const tabsStore = new TabsStore();
