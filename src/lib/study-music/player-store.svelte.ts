import { convertFileSrc } from "@tauri-apps/api/core";
import { pluginInvoke } from "$lib/plugin-invoke";
import { dominantColorFromPath, type RGB } from "./dominant-color";
import { spotifySdk, type SpotifySdkState } from "./spotify-sdk.svelte";
import { attachHls, type HlsHandle } from "./hls-loader.svelte";

export type TrackSource = "local" | "spotify" | "youtube" | "soundcloud";

export type MusicTrack = {
  id: number;
  path: string;
  title: string | null;
  artist: string | null;
  album: string | null;
  album_artist?: string | null;
  track_number?: number | null;
  disc_number?: number | null;
  year?: number | null;
  genre?: string | null;
  duration_ms: number | null;
  cover_path: string | null;
  file_size?: number;
  sample_rate?: number | null;
  bitrate?: number | null;
  added_at?: number;
  last_played_at?: number | null;
  play_count?: number;
  favorite?: boolean;
  source?: TrackSource;
  spotify_uri?: string;
  spotify_cover_url?: string;
  youtube_url?: string;
  youtube_video_id?: string;
  youtube_thumbnail?: string;
  isrc?: string;
  soundcloud_id?: number;
  soundcloud_artwork_url?: string;
};

export type RepeatMode = "off" | "all" | "one";

export type EqPreset =
  | "flat"
  | "bass"
  | "vocal"
  | "treble"
  | "rock"
  | "pop"
  | "classical"
  | "electronic"
  | "custom";

export const EQ_BAND_FREQUENCIES = [60, 250, 1000, 4000, 12000] as const;

export const EQ_PRESETS: Record<EqPreset, number[]> = {
  flat: [0, 0, 0, 0, 0],
  bass: [6, 4, 0, -2, -3],
  vocal: [-2, -1, 4, 3, 0],
  treble: [-3, -2, 0, 4, 6],
  rock: [4, 2, -2, 3, 5],
  pop: [-1, 2, 4, 2, -1],
  classical: [3, 2, 0, 2, 4],
  electronic: [5, 3, -1, 2, 4],
  custom: [0, 0, 0, 0, 0],
};

const PERSIST_KEY = "study.music.player.v1";
const QUEUE_KEY = "study.music.queue.v1";

class MusicPlayerStore {
  private audio: HTMLAudioElement | null = null;
  private playStartedAt = 0;
  private accumulatedPlayedMs = 0;
  private lastReportTrackId: number | null = null;
  private queueSaveTimer: ReturnType<typeof setTimeout> | null = null;
  private eqContext: AudioContext | null = null;
  private eqSource: MediaElementAudioSourceNode | null = null;
  private eqFilters: BiquadFilterNode[] = [];
  private eqGainNode: GainNode | null = null;

  currentTrack = $state<MusicTrack | null>(null);
  queue = $state<MusicTrack[]>([]);
  queueIndex = $state(0);
  paused = $state(true);
  currentTime = $state(0);
  duration = $state(0);
  volume = $state(1);
  muted = $state(false);
  shuffle = $state(false);
  repeat = $state<RepeatMode>("off");
  ready = $state(false);
  loading = $state(false);
  error = $state<string | null>(null);
  eqEnabled = $state(false);
  eqGains = $state<number[]>([0, 0, 0, 0, 0]);
  eqPreset = $state<EqPreset>("flat");
  dominantColor = $state<RGB | null>(null);
  private mediaSessionInstalled = false;

  init() {
    if (this.audio || typeof window === "undefined") return;
    const audio = new Audio();
    audio.preload = "metadata";
    audio.crossOrigin = "anonymous";

    audio.addEventListener("play", () => {
      this.paused = false;
      this.playStartedAt = Date.now();
      if (typeof navigator !== "undefined" && "mediaSession" in navigator) {
        navigator.mediaSession.playbackState = "playing";
      }
    });
    audio.addEventListener("pause", () => {
      this.paused = true;
      if (this.playStartedAt > 0) {
        this.accumulatedPlayedMs += Date.now() - this.playStartedAt;
        this.playStartedAt = 0;
      }
      this.scheduleQueueSave();
      if (typeof navigator !== "undefined" && "mediaSession" in navigator) {
        navigator.mediaSession.playbackState = "paused";
      }
    });
    audio.addEventListener("timeupdate", () => {
      this.currentTime = audio.currentTime;
      this.scheduleQueueSave();
      this.updateMediaSessionPosition();
    });
    audio.addEventListener("loadedmetadata", () => {
      this.duration = audio.duration || 0;
      this.ready = true;
      this.loading = false;
      this.updateMediaSessionPosition();
    });
    audio.addEventListener("loadstart", () => {
      this.loading = true;
      this.ready = false;
    });
    audio.addEventListener("canplay", () => {
      this.loading = false;
    });
    audio.addEventListener("ended", () => {
      this.handleEnded();
    });
    audio.addEventListener("error", () => {
      const err = audio.error;
      let msg = "Erro ao carregar áudio";
      if (err) {
        const codeMap: Record<number, string> = {
          1: "abortado",
          2: "rede falhou",
          3: "decodificação falhou",
          4: "formato não suportado",
        };
        const reason = codeMap[err.code] ?? `code ${err.code}`;
        msg = `Erro de áudio: ${reason}${err.message ? ` (${err.message})` : ""}`;
        console.error(
          "[player] audio error",
          { code: err.code, message: err.message, src: audio.src.slice(0, 200) },
        );
      }
      this.error = msg;
      this.loading = false;
    });
    audio.addEventListener("volumechange", () => {
      this.volume = audio.volume;
      this.muted = audio.muted;
    });

    try {
      const raw = localStorage.getItem(PERSIST_KEY);
      if (raw) {
        const data = JSON.parse(raw) as {
          volume?: number;
          shuffle?: boolean;
          repeat?: RepeatMode;
          muted?: boolean;
          eqEnabled?: boolean;
          eqGains?: number[];
          eqPreset?: EqPreset;
        };
        if (typeof data.volume === "number") {
          audio.volume = Math.min(1, Math.max(0, data.volume));
        }
        if (typeof data.shuffle === "boolean") this.shuffle = data.shuffle;
        if (data.repeat === "off" || data.repeat === "all" || data.repeat === "one") {
          this.repeat = data.repeat;
        }
        if (typeof data.muted === "boolean") audio.muted = data.muted;
        if (typeof data.eqEnabled === "boolean") this.eqEnabled = data.eqEnabled;
        if (Array.isArray(data.eqGains) && data.eqGains.length === 5) {
          this.eqGains = data.eqGains.map((g) =>
            Math.max(-12, Math.min(12, Number(g) || 0)),
          );
        }
        if (data.eqPreset && data.eqPreset in EQ_PRESETS) {
          this.eqPreset = data.eqPreset;
        }
      }
    } catch {
      /* ignore */
    }

    this.audio = audio;
    this.volume = audio.volume;
    this.muted = audio.muted;
    this.installMediaSession();
    this.restoreQueue();
    this.installSpotifySync();
  }

  private spotifyUnsub: (() => void) | null = null;
  private installSpotifySync() {
    if (this.spotifyUnsub) return;
    this.spotifyUnsub = spotifySdk.onState((state) => this.applySpotifyState(state));
  }

  private applySpotifyState(state: SpotifySdkState | null) {
    if (this.currentTrack?.source !== "spotify") return;
    if (!state) return;
    this.paused = state.paused;
    this.currentTime = state.position / 1000;
    this.duration = (state.duration || this.currentTrack.duration_ms || 0) / 1000;
    this.ready = true;
    this.loading = false;
    this.updateMediaSessionPosition();

    const sdkUri = state.track_window?.current_track?.uri;
    if (
      sdkUri &&
      this.currentTrack.spotify_uri &&
      sdkUri !== this.currentTrack.spotify_uri
    ) {
      const inQueue = this.queue.find((t) => t.spotify_uri === sdkUri);
      if (inQueue) {
        this.currentTrack = inQueue;
        this.queueIndex = this.queue.indexOf(inQueue);
        this.updateMediaSessionMetadata(inQueue);
      }
    }
  }

  private isSpotify(): boolean {
    return this.currentTrack?.source === "spotify";
  }

  private installMediaSession() {
    if (
      this.mediaSessionInstalled ||
      typeof navigator === "undefined" ||
      !("mediaSession" in navigator)
    ) {
      return;
    }
    try {
      navigator.mediaSession.setActionHandler("play", () => this.resume());
      navigator.mediaSession.setActionHandler("pause", () => this.pause());
      navigator.mediaSession.setActionHandler("previoustrack", () => this.prev());
      navigator.mediaSession.setActionHandler("nexttrack", () => this.next());
      navigator.mediaSession.setActionHandler("seekbackward", (e) => {
        const off = e.seekOffset ?? 10;
        this.seek(Math.max(0, this.currentTime - off));
      });
      navigator.mediaSession.setActionHandler("seekforward", (e) => {
        const off = e.seekOffset ?? 10;
        this.seek(this.currentTime + off);
      });
      navigator.mediaSession.setActionHandler("seekto", (e) => {
        if (typeof e.seekTime === "number") this.seek(e.seekTime);
      });
      navigator.mediaSession.setActionHandler("stop", () => this.pause());
      this.mediaSessionInstalled = true;
    } catch {
      /* not all UAs support every action */
    }
  }

  private updateMediaSessionMetadata(track: MusicTrack | null) {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator)) return;
    if (!track) {
      navigator.mediaSession.metadata = null;
      return;
    }
    let artwork: MediaImage[] = [];
    if (track.cover_path) {
      try {
        const url = convertFileSrc(track.cover_path);
        artwork = [
          { src: url, sizes: "512x512", type: "image/jpeg" },
        ];
      } catch {
        /* ignore */
      }
    }
    try {
      navigator.mediaSession.metadata = new MediaMetadata({
        title: track.title ?? track.path,
        artist: track.artist ?? "",
        album: track.album ?? "",
        artwork,
      });
    } catch {
      /* ignore */
    }
  }

  private updateMediaSessionPosition() {
    if (
      typeof navigator === "undefined" ||
      !("mediaSession" in navigator) ||
      !("setPositionState" in navigator.mediaSession)
    ) {
      return;
    }
    if (!this.duration || !Number.isFinite(this.duration)) return;
    try {
      navigator.mediaSession.setPositionState({
        duration: this.duration,
        position: Math.min(this.currentTime, this.duration),
        playbackRate: 1,
      });
    } catch {
      /* ignore */
    }
  }

  private async refreshDominantColor(track: MusicTrack | null) {
    if (!track || !track.cover_path) {
      this.dominantColor = null;
      return;
    }
    const c = await dominantColorFromPath(track.cover_path);
    if (this.currentTrack?.id === track.id) {
      this.dominantColor = c;
    }
  }

  private restoreQueue() {
    if (!this.audio) return;
    try {
      const raw = localStorage.getItem(QUEUE_KEY);
      if (!raw) return;
      const data = JSON.parse(raw) as {
        queue?: MusicTrack[];
        queueIndex?: number;
        currentTime?: number;
      };
      if (!Array.isArray(data.queue) || data.queue.length === 0) return;
      const idx = Math.max(
        0,
        Math.min((data.queueIndex ?? 0) | 0, data.queue.length - 1),
      );
      const track = data.queue[idx];
      if (!track || typeof track.path !== "string") return;
      this.queue = data.queue;
      this.queueIndex = idx;
      this.currentTrack = track;
      this.duration = (track.duration_ms ?? 0) / 1000;
      this.audio.src = convertFileSrc(track.path);
      this.updateMediaSessionMetadata(track);
      void this.refreshDominantColor(track);
      const t = Number(data.currentTime);
      if (Number.isFinite(t) && t > 0) {
        const onLoaded = () => {
          if (this.audio) this.audio.currentTime = t;
          this.audio?.removeEventListener("loadedmetadata", onLoaded);
        };
        this.audio.addEventListener("loadedmetadata", onLoaded);
      }
    } catch {
      /* ignore */
    }
  }

  private persist() {
    try {
      localStorage.setItem(
        PERSIST_KEY,
        JSON.stringify({
          volume: this.volume,
          shuffle: this.shuffle,
          repeat: this.repeat,
          muted: this.muted,
          eqEnabled: this.eqEnabled,
          eqGains: this.eqGains,
          eqPreset: this.eqPreset,
        }),
      );
    } catch {
      /* ignore */
    }
  }

  private scheduleQueueSave() {
    if (this.queueSaveTimer) return;
    this.queueSaveTimer = setTimeout(() => {
      this.queueSaveTimer = null;
      this.saveQueueNow();
    }, 1500);
  }

  private saveQueueNow() {
    if (typeof localStorage === "undefined") return;
    try {
      if (this.queue.length === 0 || !this.currentTrack) {
        localStorage.removeItem(QUEUE_KEY);
        return;
      }
      localStorage.setItem(
        QUEUE_KEY,
        JSON.stringify({
          queue: this.queue,
          queueIndex: this.queueIndex,
          currentTime: this.currentTime,
        }),
      );
    } catch {
      /* ignore */
    }
  }

  private ensureEqGraph() {
    if (!this.audio || this.eqContext) return;
    if (typeof window === "undefined" || !window.AudioContext) return;
    try {
      const ctx = new AudioContext();
      const source = ctx.createMediaElementSource(this.audio);
      const filters = EQ_BAND_FREQUENCIES.map((freq) => {
        const f = ctx.createBiquadFilter();
        f.type = "peaking";
        f.frequency.value = freq;
        f.Q.value = 1;
        f.gain.value = 0;
        return f;
      });
      const out = ctx.createGain();
      out.gain.value = 1;
      let node: AudioNode = source;
      for (const f of filters) {
        node.connect(f);
        node = f;
      }
      node.connect(out);
      out.connect(ctx.destination);
      this.eqContext = ctx;
      this.eqSource = source;
      this.eqFilters = filters;
      this.eqGainNode = out;
      this.applyEq();
    } catch {
      /* ignore — EQ optional */
    }
  }

  private applyEq() {
    if (this.eqFilters.length !== 5) return;
    const gains = this.eqEnabled ? this.eqGains : [0, 0, 0, 0, 0];
    for (let i = 0; i < 5; i++) {
      this.eqFilters[i].gain.value = gains[i] ?? 0;
    }
  }

  setEqEnabled(enabled: boolean) {
    this.eqEnabled = enabled;
    this.ensureEqGraph();
    this.applyEq();
    this.persist();
  }

  setEqGain(band: number, gainDb: number) {
    if (band < 0 || band >= 5) return;
    const next = [...this.eqGains];
    next[band] = Math.max(-12, Math.min(12, gainDb));
    this.eqGains = next;
    this.eqPreset = "custom";
    this.applyEq();
    this.persist();
  }

  setEqPreset(preset: EqPreset) {
    if (!(preset in EQ_PRESETS)) return;
    this.eqPreset = preset;
    if (preset !== "custom") {
      this.eqGains = [...EQ_PRESETS[preset]];
    }
    this.applyEq();
    this.persist();
  }

  async play(track: MusicTrack, queue?: MusicTrack[]) {
    this.init();

    void this.flushPlayReport();

    if (queue && queue.length > 0) {
      this.queue = queue;
      const idx = queue.findIndex(
        (t) =>
          t.id === track.id &&
          (t.source ?? "local") === (track.source ?? "local"),
      );
      this.queueIndex = idx >= 0 ? idx : 0;
    } else {
      const existingIdx = this.queue.findIndex(
        (t) =>
          t.id === track.id &&
          (t.source ?? "local") === (track.source ?? "local"),
      );
      if (existingIdx < 0) {
        this.queue = [track];
        this.queueIndex = 0;
      } else {
        this.queueIndex = existingIdx;
      }
    }

    this.currentTrack = track;
    this.error = null;
    this.ready = false;
    this.currentTime = 0;
    this.duration = (track.duration_ms ?? 0) / 1000;
    this.accumulatedPlayedMs = 0;
    this.playStartedAt = 0;
    this.lastReportTrackId = track.source === "spotify" ? null : track.id;

    if (track.source === "spotify") {
      await this.playSpotify(track);
    } else if (track.source === "youtube") {
      await this.playYoutube(track);
    } else if (track.source === "soundcloud") {
      await this.playSoundcloud(track);
    } else {
      await this.playLocal(track);
    }
  }

  soundcloudResolver:
    | ((track: MusicTrack) => Promise<{ url: string; is_hls: boolean }>)
    | null = null;
  private hlsHandle: HlsHandle | null = null;

  private async playSoundcloud(track: MusicTrack) {
    if (!this.audio) return;
    if (!this.soundcloudResolver) {
      this.error = "SoundCloud resolver não configurado";
      return;
    }
    this.loading = true;
    try {
      try {
        await spotifySdk.pause();
      } catch {
        /* ignore */
      }
      if (this.hlsHandle) {
        try {
          this.hlsHandle.destroy();
        } catch {
          /* ignore */
        }
        this.hlsHandle = null;
      }
      const { url, is_hls } = await this.soundcloudResolver(track);
      if (is_hls || url.includes(".m3u8")) {
        this.hlsHandle = await attachHls(this.audio, url);
      } else {
        this.audio.src = url;
      }
      await this.audio.play();
      this.ensureEqGraph();
      this.saveQueueNow();
      this.updateMediaSessionMetadata(track);
    } catch (e) {
      this.error = e instanceof Error ? e.message : "Erro ao tocar SoundCloud";
      this.loading = false;
      throw e;
    }
  }

  private async playYoutube(track: MusicTrack) {
    if (!this.audio) return;
    const videoId = track.youtube_video_id;
    if (!videoId) {
      this.error = "Video ID ausente";
      this.loading = false;
      return;
    }
    this.loading = true;
    try {
      const res = await pluginInvoke<{ url: string }>(
        "study",
        "study:music:youtube:track_stream_url",
        { video_id: videoId },
      );
      if (!res.url) throw new Error("empty stream url");
      this.audio.src = res.url;
      await this.audio.play();
      this.ensureEqGraph();
      this.saveQueueNow();
      this.updateMediaSessionMetadata(track);
      void this.refreshDominantColor(track);
      void pluginInvoke("study", "study:music:youtube:track_record_play", {
        video_id: videoId,
      }).catch(() => {});
    } catch (e) {
      this.error = e instanceof Error ? e.message : "Erro ao tocar (YouTube)";
      this.loading = false;
    }
  }

  private async playLocal(track: MusicTrack) {
    if (!this.audio) return;
    try {
      try {
        await spotifySdk.pause();
      } catch {
        /* ignore */
      }
      const isHttpUrl =
        track.path.startsWith("http://") || track.path.startsWith("https://");
      this.audio.src = isHttpUrl ? track.path : convertFileSrc(track.path);
      await this.audio.play();
      this.ensureEqGraph();
      this.saveQueueNow();
      this.updateMediaSessionMetadata(track);
      void this.refreshDominantColor(track);
    } catch (e) {
      this.error = e instanceof Error ? e.message : "Erro ao tocar";
      this.loading = false;
    }
  }

  spotifyFreeFallback: ((track: MusicTrack) => Promise<string>) | null = null;

  private async playSpotify(track: MusicTrack) {
    if (!track.spotify_uri) {
      this.error = "Faixa Spotify sem URI";
      return;
    }
    if (this.audio && !this.audio.paused) {
      try {
        this.audio.pause();
      } catch {
        /* ignore */
      }
    }
    this.loading = true;
    let sdkErr: unknown = null;
    try {
      await spotifySdk.ensureLoaded();
      const queueUris = this.queue
        .filter((t) => t.source === "spotify" && t.spotify_uri)
        .map((t) => t.spotify_uri as string);
      const startUri = track.spotify_uri;
      const orderedUris = (() => {
        const idx = queueUris.indexOf(startUri);
        if (idx < 0) return [startUri];
        return [...queueUris.slice(idx), ...queueUris.slice(0, idx)];
      })();
      await spotifySdk.play({ uris: orderedUris });
      this.saveQueueNow();
      this.updateMediaSessionMetadata(track);
      if (track.spotify_cover_url) {
        void this.refreshDominantColorFromUrl(track.spotify_cover_url);
      }
      return;
    } catch (e) {
      sdkErr = e;
      console.warn("[player] Spotify SDK failed, trying YouTube fallback:", e);
    }

    if (!this.spotifyFreeFallback || !this.audio) {
      this.error =
        sdkErr instanceof Error
          ? sdkErr.message
          : "Não foi possível tocar via Spotify SDK e nenhum fallback disponível";
      this.loading = false;
      throw sdkErr ?? new Error(String(this.error));
    }

    try {
      const url = await this.spotifyFreeFallback(track);
      track.youtube_url = url;
      this.audio.src = url;
      await this.audio.play();
      this.ensureEqGraph();
      this.saveQueueNow();
      this.updateMediaSessionMetadata(track);
    } catch (e) {
      this.error =
        e instanceof Error
          ? `Spotify e YouTube falharam: ${e.message}`
          : "Falha ao resolver áudio";
      this.loading = false;
      throw e;
    }
  }

  private async refreshDominantColorFromUrl(_url: string) {
    this.dominantColor = null;
  }

  togglePlay() {
    this.init();
    if (!this.currentTrack) return;
    if (this.isSpotify()) {
      void spotifySdk.togglePlay();
      return;
    }
    if (!this.audio) return;
    if (this.paused) {
      this.audio.play().catch((e) => {
        this.error = e instanceof Error ? e.message : String(e);
      });
      this.ensureEqGraph();
    } else {
      this.audio.pause();
    }
  }

  pause() {
    if (this.isSpotify()) {
      void spotifySdk.pause();
      return;
    }
    if (this.audio && !this.paused) this.audio.pause();
  }

  resume() {
    if (this.isSpotify()) {
      void spotifySdk.resume();
      return;
    }
    if (this.audio && this.paused && this.currentTrack) {
      this.audio.play().catch(() => {});
      this.ensureEqGraph();
    }
  }

  next() {
    if (this.isSpotify()) {
      void spotifySdk.next();
      return;
    }
    if (this.queue.length === 0) return;
    let nextIdx: number;
    if (this.shuffle && this.queue.length > 1) {
      do {
        nextIdx = Math.floor(Math.random() * this.queue.length);
      } while (nextIdx === this.queueIndex);
    } else {
      nextIdx = this.queueIndex + 1;
      if (nextIdx >= this.queue.length) {
        if (this.repeat === "all") {
          nextIdx = 0;
        } else {
          this.audio?.pause();
          return;
        }
      }
    }
    this.queueIndex = nextIdx;
    void this.play(this.queue[nextIdx], this.queue);
  }

  prev() {
    if (this.isSpotify()) {
      void spotifySdk.prev();
      return;
    }
    if (this.queue.length === 0) return;
    if (this.audio && this.audio.currentTime > 3) {
      this.audio.currentTime = 0;
      return;
    }
    let prevIdx = this.queueIndex - 1;
    if (prevIdx < 0) {
      prevIdx = this.repeat === "all" ? this.queue.length - 1 : 0;
    }
    this.queueIndex = prevIdx;
    void this.play(this.queue[prevIdx], this.queue);
  }

  seek(seconds: number) {
    if (this.isSpotify()) {
      const ms = Math.max(0, seconds * 1000);
      void spotifySdk.seek(ms);
      return;
    }
    if (!this.audio) return;
    const dur = this.audio.duration || this.duration;
    if (!Number.isFinite(dur) || dur <= 0) return;
    this.audio.currentTime = Math.max(0, Math.min(dur, seconds));
  }

  seekRatio(ratio: number) {
    const dur = this.duration;
    if (!Number.isFinite(dur) || dur <= 0) return;
    this.seek(dur * Math.max(0, Math.min(1, ratio)));
  }

  setVolume(v: number) {
    const clamped = Math.max(0, Math.min(1, v));
    if (this.isSpotify()) {
      void spotifySdk.setVolume(clamped);
      this.volume = clamped;
      this.persist();
      return;
    }
    if (!this.audio) return;
    this.audio.volume = clamped;
    if (clamped > 0 && this.audio.muted) this.audio.muted = false;
    this.persist();
  }

  toggleMute() {
    if (this.isSpotify()) {
      const next = !this.muted;
      void spotifySdk.setVolume(next ? 0 : this.volume);
      this.muted = next;
      this.persist();
      return;
    }
    if (!this.audio) return;
    this.audio.muted = !this.audio.muted;
    this.persist();
  }

  toggleShuffle() {
    this.shuffle = !this.shuffle;
    this.persist();
  }

  cycleRepeat() {
    const order: RepeatMode[] = ["off", "all", "one"];
    const cur = order.indexOf(this.repeat);
    this.repeat = order[(cur + 1) % order.length];
    this.persist();
  }

  clearQueue() {
    this.audio?.pause();
    this.queue = [];
    this.queueIndex = 0;
    this.currentTrack = null;
    this.currentTime = 0;
    this.duration = 0;
    this.saveQueueNow();
  }

  private handleEnded() {
    void this.flushPlayReport();
    if (this.repeat === "one" && this.audio) {
      this.audio.currentTime = 0;
      this.audio.play().catch(() => {});
      return;
    }
    this.next();
  }

  private async flushPlayReport() {
    const track_id = this.lastReportTrackId;
    if (!track_id) return;
    if (this.playStartedAt > 0) {
      this.accumulatedPlayedMs += Date.now() - this.playStartedAt;
      this.playStartedAt = 0;
    }
    const positionMs = Math.floor((this.currentTime || 0) * 1000);
    const durationPlayedMs = this.accumulatedPlayedMs;
    this.accumulatedPlayedMs = 0;
    this.lastReportTrackId = null;
    if (durationPlayedMs <= 0) return;
    try {
      await pluginInvoke("study", "study:music:play:record", {
        trackId: track_id,
        positionMs,
        durationPlayedMs,
      });
    } catch {
      /* ignore */
    }
  }

  async toggleFavorite(trackId: number): Promise<boolean | null> {
    try {
      const res = await pluginInvoke<{ favorite: boolean }>(
        "study",
        "study:music:favorite:toggle",
        { trackId },
      );
      if (this.currentTrack && this.currentTrack.id === trackId) {
        this.currentTrack = { ...this.currentTrack, favorite: res.favorite };
      }
      return res.favorite;
    } catch {
      return null;
    }
  }

  reportNow() {
    void this.flushPlayReport();
    this.saveQueueNow();
  }
}

export const musicPlayer = new MusicPlayerStore();

if (typeof window !== "undefined") {
  window.addEventListener("beforeunload", () => {
    musicPlayer.reportNow();
  });
}
