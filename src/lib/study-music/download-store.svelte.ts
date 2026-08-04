import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type DownloadJobStage =
  | "pending"
  | "starting"
  | "downloading"
  | "done"
  | "skipped"
  | "error";

export type DownloadJobKind = "single" | "bulk";

export type DownloadJobState = {
  id: string;
  kind: DownloadJobKind;
  trackId?: number;
  playlistId?: number;
  title: string;
  artist?: string;
  artwork?: string | null;
  stage: DownloadJobStage;
  progressPct: number;
  totalCount?: number;
  currentCount?: number;
  successCount?: number;
  failedCount?: number;
  currentTrackTitle?: string;
  failedTracks?: { id: number; title: string; error?: string }[];
  codec?: string;
  outputDir?: string;
  error?: string;
  path?: string;
  permalinkUrl?: string;
  startedAt: number;
  updatedAt: number;
  expanded?: boolean;
};

type SinglePayload = {
  job_id: number;
  track_id?: number;
  title?: string;
  artist?: string;
  duration_ms?: number;
  stage: string;
  progress_ms?: number;
  error?: string;
  path?: string;
  permalink_url?: string;
  reason?: string;
};

type BulkPayload = {
  playlist_id: number;
  stage: string;
  current?: number;
  total?: number;
  success?: number;
  failed?: number;
  title?: string;
  failed_track_id?: number;
  failed_track_title?: string;
  failed_track_error?: string;
};

function pctFromMs(progressMs: number, totalMs: number, fallback: number): number {
  if (totalMs > 0) return Math.min(100, Math.round((progressMs / totalMs) * 100));
  return fallback;
}

function pctFromCount(current: number, total: number, fallback: number): number {
  if (total > 0) return Math.min(100, Math.round((current / total) * 100));
  return fallback;
}

class DownloadStore {
  jobs = $state<Map<string, DownloadJobState>>(new Map());
  drawerOpen = $state(false);

  private pendingByTrackId = new Map<number, string>();
  private listeners: UnlistenFn[] = [];
  private nonce = 0;
  private started = false;

  start() {
    if (this.started) return;
    this.started = true;
    void (async () => {
      try {
        const single = await listen<SinglePayload>(
          "study-soundcloud-download-progress",
          (e) => this.handleSingle(e.payload),
        );
        this.listeners.push(single);
      } catch (err) {
        console.warn("[sc-download-store] listen single failed", err);
      }
      try {
        const bulk = await listen<BulkPayload>(
          "study-soundcloud-download-bulk-progress",
          (e) => this.handleBulk(e.payload),
        );
        this.listeners.push(bulk);
      } catch (err) {
        console.warn("[sc-download-store] listen bulk failed", err);
      }
    })();
  }

  stop() {
    for (const u of this.listeners) {
      try {
        u();
      } catch {
        /* ignore */
      }
    }
    this.listeners = [];
    this.started = false;
  }

  private handleSingle(p: SinglePayload) {
    if (typeof p.job_id !== "number") return;
    const key = `single-${p.job_id}`;
    const trackId = typeof p.track_id === "number" ? p.track_id : undefined;
    const next = new Map(this.jobs);
    let prev = next.get(key);
    if (!prev && trackId !== undefined) {
      const optimisticKey = this.pendingByTrackId.get(trackId);
      if (optimisticKey) {
        const opt = next.get(optimisticKey);
        if (opt) prev = opt;
        next.delete(optimisticKey);
        this.pendingByTrackId.delete(trackId);
      }
    }
    const rawStage = (p.stage ?? "downloading") as DownloadJobStage;
    const stage: DownloadJobStage = rawStage;
    const totalMs = typeof p.duration_ms === "number" ? p.duration_ms : 0;
    const progressMs = typeof p.progress_ms === "number" ? p.progress_ms : 0;
    const fallbackPct = prev?.progressPct ?? 0;
    const progressPct =
      stage === "done" || stage === "skipped"
        ? 100
        : pctFromMs(progressMs, totalMs, fallbackPct);
    const job: DownloadJobState = {
      id: key,
      kind: "single",
      trackId,
      title: p.title ?? prev?.title ?? "Faixa",
      artist: p.artist ?? prev?.artist,
      artwork: prev?.artwork ?? null,
      stage,
      progressPct,
      error: stage === "error" ? p.error ?? "Erro desconhecido" : prev?.error,
      path: p.path ?? prev?.path,
      permalinkUrl: p.permalink_url ?? prev?.permalinkUrl,
      startedAt: prev?.startedAt ?? Date.now(),
      updatedAt: Date.now(),
    };
    next.set(key, job);
    this.jobs = next;
  }

  private handleBulk(p: BulkPayload) {
    if (typeof p.playlist_id !== "number") return;
    const key = `bulk-${p.playlist_id}`;
    const next = new Map(this.jobs);
    const prev = next.get(key);
    const rawStage = (p.stage ?? "downloading") as DownloadJobStage;
    const stage: DownloadJobStage = rawStage;
    const total = p.total ?? prev?.totalCount ?? 0;
    const current = p.current ?? prev?.currentCount ?? 0;
    const fallbackPct = prev?.progressPct ?? 0;
    const progressPct =
      stage === "done" ? 100 : pctFromCount(current, total, fallbackPct);
    const currentTrackTitle = stage === "downloading" ? p.title : prev?.currentTrackTitle;
    const failedTracks = (prev?.failedTracks ?? []).slice();
    if (
      typeof p.failed_track_id === "number" &&
      !failedTracks.some((ft) => ft.id === p.failed_track_id)
    ) {
      failedTracks.push({
        id: p.failed_track_id,
        title: p.failed_track_title ?? `Faixa ${p.failed_track_id}`,
        error: p.failed_track_error,
      });
    }
    const job: DownloadJobState = {
      id: key,
      kind: "bulk",
      playlistId: p.playlist_id,
      title: prev?.title ?? `Playlist #${p.playlist_id}`,
      artwork: prev?.artwork ?? null,
      stage,
      progressPct,
      totalCount: total,
      currentCount: current,
      successCount: p.success ?? prev?.successCount,
      failedCount: p.failed ?? prev?.failedCount,
      currentTrackTitle,
      failedTracks,
      codec: prev?.codec,
      outputDir: prev?.outputDir,
      startedAt: prev?.startedAt ?? Date.now(),
      updatedAt: Date.now(),
      expanded: prev?.expanded ?? false,
    };
    next.set(key, job);
    this.jobs = next;
  }

  addOptimisticSingleJob(input: {
    trackId: number;
    title: string;
    artist?: string;
    artwork?: string | null;
  }): string {
    this.nonce += 1;
    const id = `pending-${input.trackId}-${this.nonce}`;
    const next = new Map(this.jobs);
    next.set(id, {
      id,
      kind: "single",
      trackId: input.trackId,
      title: input.title,
      artist: input.artist,
      artwork: input.artwork ?? null,
      stage: "pending",
      progressPct: 0,
      startedAt: Date.now(),
      updatedAt: Date.now(),
    });
    this.jobs = next;
    this.pendingByTrackId.set(input.trackId, id);
    return id;
  }

  addOptimisticBulkJob(input: {
    playlistId: number;
    title: string;
    total: number;
    artwork?: string | null;
    codec?: string;
    outputDir?: string;
  }): string {
    const id = `bulk-${input.playlistId}`;
    const next = new Map(this.jobs);
    const prev = next.get(id);
    next.set(id, {
      id,
      kind: "bulk",
      playlistId: input.playlistId,
      title: input.title,
      artwork: input.artwork ?? null,
      stage: prev?.stage ?? "pending",
      progressPct: prev?.progressPct ?? 0,
      totalCount: input.total,
      currentCount: prev?.currentCount ?? 0,
      successCount: prev?.successCount,
      failedCount: prev?.failedCount,
      currentTrackTitle: prev?.currentTrackTitle,
      failedTracks: prev?.failedTracks ?? [],
      codec: input.codec ?? prev?.codec,
      outputDir: input.outputDir ?? prev?.outputDir,
      startedAt: prev?.startedAt ?? Date.now(),
      updatedAt: Date.now(),
      expanded: prev?.expanded ?? false,
    });
    this.jobs = next;
    return id;
  }

  consumeFailedTracks(jobId: string): { id: number; title: string; error?: string }[] {
    const job = this.jobs.get(jobId);
    if (!job || !job.failedTracks?.length) return [];
    const taken = job.failedTracks.slice();
    const next = new Map(this.jobs);
    next.set(jobId, { ...job, failedTracks: [], updatedAt: Date.now() });
    this.jobs = next;
    return taken;
  }

  removeFailedTrackFromBulk(jobId: string, failedTrackId: number) {
    const job = this.jobs.get(jobId);
    if (!job || !job.failedTracks?.length) return;
    const next = new Map(this.jobs);
    next.set(jobId, {
      ...job,
      failedTracks: job.failedTracks.filter((ft) => ft.id !== failedTrackId),
      updatedAt: Date.now(),
    });
    this.jobs = next;
  }

  markJobError(jobId: string, error: string) {
    const next = new Map(this.jobs);
    const prev = next.get(jobId);
    if (!prev) return;
    next.set(jobId, {
      ...prev,
      stage: "error",
      error,
      updatedAt: Date.now(),
    });
    this.jobs = next;
    if (prev.trackId !== undefined && this.pendingByTrackId.get(prev.trackId) === jobId) {
      this.pendingByTrackId.delete(prev.trackId);
    }
  }

  removeJob(jobId: string) {
    const next = new Map(this.jobs);
    const prev = next.get(jobId);
    if (!prev) return;
    next.delete(jobId);
    this.jobs = next;
    if (prev.trackId !== undefined && this.pendingByTrackId.get(prev.trackId) === jobId) {
      this.pendingByTrackId.delete(prev.trackId);
    }
  }

  clearCompleted() {
    const next = new Map(this.jobs);
    for (const [id, job] of Array.from(next)) {
      if ((job.stage === "done" || job.stage === "skipped") && !job.error) {
        next.delete(id);
      }
    }
    this.jobs = next;
  }

  toggleExpanded(jobId: string) {
    const next = new Map(this.jobs);
    const prev = next.get(jobId);
    if (!prev) return;
    next.set(jobId, { ...prev, expanded: !prev.expanded });
    this.jobs = next;
  }

  toggleDrawer(force?: boolean) {
    this.drawerOpen = typeof force === "boolean" ? force : !this.drawerOpen;
  }

  get jobsList(): DownloadJobState[] {
    return Array.from(this.jobs.values()).sort((a, b) => b.startedAt - a.startedAt);
  }

  get activeCount(): number {
    let n = 0;
    for (const job of this.jobs.values()) {
      if (job.stage !== "done" && job.stage !== "skipped" && job.stage !== "error") {
        n += 1;
      }
    }
    return n;
  }

  get hasJobs(): boolean {
    return this.jobs.size > 0;
  }

  getJobByTrackId(trackId: number): DownloadJobState | null {
    let active: DownloadJobState | null = null;
    let any: DownloadJobState | null = null;
    for (const job of this.jobs.values()) {
      if (job.trackId !== trackId) continue;
      if (!any || job.updatedAt > any.updatedAt) any = job;
      if (job.stage === "done" || job.stage === "skipped" || job.stage === "error") continue;
      if (!active || job.updatedAt > active.updatedAt) active = job;
    }
    return active ?? any;
  }
}

export const downloadStore = new DownloadStore();
