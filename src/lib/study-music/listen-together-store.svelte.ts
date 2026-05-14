import { musicPlayer, type MusicTrack } from "./player-store.svelte";

export type ListenMember = {
  id: string;
  name: string;
  role: "host" | "guest";
};

type WireMessage =
  | { type: "welcome"; id: string; role: "host" | "guest"; members: ListenMember[] }
  | { type: "user_joined"; id: string; name: string; role: "host" | "guest" }
  | { type: "user_left"; id: string }
  | { type: "host_changed"; id: string }
  | { type: "promoted_to_host" }
  | { type: "sync_request"; from: string }
  | { type: "state_sync"; track: MusicTrack; time: number; paused: boolean; from?: string; at?: number }
  | { type: "track_change"; track: MusicTrack; from?: string; at?: number }
  | { type: "play"; from?: string; at?: number }
  | { type: "pause"; time?: number; from?: string; at?: number }
  | { type: "seek"; time: number; from?: string; at?: number }
  | { type: "chat"; from: string; name: string; text: string; at: number }
  | { type: "pong"; at: number }
  | { type: "error"; code: string; message: string };

export type ChatLine = {
  id: string;
  name: string;
  text: string;
  at: number;
};

const STORAGE_KEY = "study-music-listen-relay";

class ListenTogetherStore {
  private ws: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private intentionalDisconnect = false;
  private suppressBroadcast = false;
  private connectionUrl: string | null = null;
  private playerUnsub: (() => void) | null = null;

  status = $state<"idle" | "connecting" | "connected" | "error">("idle");
  errorMessage = $state<string | null>(null);
  selfId = $state<string | null>(null);
  selfRole = $state<"host" | "guest" | null>(null);
  roomCode = $state<string | null>(null);
  displayName = $state("");
  relayUrl = $state("");
  members = $state<ListenMember[]>([]);
  chat = $state<ChatLine[]>([]);

  constructor() {
    if (typeof window !== "undefined") {
      try {
        const raw = localStorage.getItem(STORAGE_KEY);
        if (raw) {
          const data = JSON.parse(raw) as { relayUrl?: string; displayName?: string };
          if (typeof data.relayUrl === "string") this.relayUrl = data.relayUrl;
          if (typeof data.displayName === "string") this.displayName = data.displayName;
        }
      } catch {
        /* ignore */
      }
    }
  }

  get isHost(): boolean {
    return this.selfRole === "host";
  }

  get connected(): boolean {
    return this.status === "connected";
  }

  private persist() {
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ relayUrl: this.relayUrl, displayName: this.displayName }),
      );
    } catch {
      /* ignore */
    }
  }

  setRelayUrl(url: string) {
    this.relayUrl = url.trim();
    this.persist();
  }

  setDisplayName(name: string) {
    this.displayName = name.trim().slice(0, 40);
    this.persist();
  }

  static randomRoomCode(): string {
    const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let out = "";
    const buf = new Uint8Array(6);
    crypto.getRandomValues(buf);
    for (let i = 0; i < 6; i++) out += alphabet[buf[i] % alphabet.length];
    return out;
  }

  createRoom(code?: string) {
    const room = (code ?? ListenTogetherStore.randomRoomCode()).toUpperCase();
    return this.connect(room, "host");
  }

  joinRoom(code: string) {
    return this.connect(code.trim().toUpperCase(), "guest");
  }

  private connect(room: string, role: "host" | "guest") {
    this.disconnect(true);
    this.intentionalDisconnect = false;
    if (!this.relayUrl) {
      this.status = "error";
      this.errorMessage = "Relay URL not set";
      return;
    }
    if (!this.displayName) {
      this.status = "error";
      this.errorMessage = "Display name required";
      return;
    }
    this.status = "connecting";
    this.errorMessage = null;
    const base = this.relayUrl.replace(/\/+$/, "");
    const sep = base.includes("?") ? "&" : "?";
    const url = `${base}${sep}room=${encodeURIComponent(room)}&name=${encodeURIComponent(this.displayName)}&role=${role}`;
    this.connectionUrl = url;
    this.openSocket(url, room);
  }

  private openSocket(url: string, room: string) {
    try {
      const ws = new WebSocket(url);
      this.ws = ws;
      this.roomCode = room;
      ws.onopen = () => {
        this.startPing();
        this.attachPlayerListener();
      };
      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data) as WireMessage;
          this.handleMessage(msg);
        } catch {
          /* ignore malformed */
        }
      };
      ws.onerror = () => {
        if (this.status !== "connected") {
          this.status = "error";
          this.errorMessage = "Failed to reach relay";
        }
      };
      ws.onclose = () => {
        this.stopPing();
        if (this.intentionalDisconnect) {
          this.resetSessionState();
          this.status = "idle";
          return;
        }
        this.status = "connecting";
        this.scheduleReconnect();
      };
    } catch (e) {
      this.status = "error";
      this.errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  private scheduleReconnect() {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    if (!this.connectionUrl || !this.roomCode) return;
    this.reconnectTimer = setTimeout(() => {
      this.openSocket(this.connectionUrl!, this.roomCode!);
    }, 3000);
  }

  private startPing() {
    this.stopPing();
    this.pingTimer = setInterval(() => {
      if (this.ws?.readyState === 1) {
        try {
          this.ws.send(JSON.stringify({ type: "ping" }));
        } catch {
          /* ignore */
        }
      }
    }, 25_000);
  }

  private stopPing() {
    if (this.pingTimer !== null) {
      clearInterval(this.pingTimer);
      this.pingTimer = null;
    }
  }

  private resetSessionState() {
    this.selfId = null;
    this.selfRole = null;
    this.roomCode = null;
    this.members = [];
    this.chat = [];
    this.detachPlayerListener();
  }

  private attachPlayerListener() {
    if (this.playerUnsub) return;
    this.playerUnsub = musicPlayer.addBroadcastListener((ev) => {
      if (this.suppressBroadcast || !this.isHost) return;
      switch (ev.type) {
        case "play":
          this.send({ type: "play" });
          break;
        case "pause":
          this.send({ type: "pause", time: ev.time });
          break;
        case "seek":
          this.send({ type: "seek", time: ev.time });
          break;
        case "track-change":
          this.send({ type: "track_change", track: ev.track });
          break;
      }
    });
  }

  private detachPlayerListener() {
    if (this.playerUnsub) {
      this.playerUnsub();
      this.playerUnsub = null;
    }
  }

  disconnect(keepStatus = false) {
    this.intentionalDisconnect = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.stopPing();
    if (this.ws) {
      try {
        this.ws.close();
      } catch {
        /* ignore */
      }
      this.ws = null;
    }
    this.connectionUrl = null;
    if (!keepStatus) {
      this.resetSessionState();
      this.status = "idle";
    }
  }

  private handleMessage(msg: WireMessage) {
    switch (msg.type) {
      case "welcome": {
        this.selfId = msg.id;
        this.selfRole = msg.role;
        this.members = msg.members;
        this.status = "connected";
        this.errorMessage = null;
        break;
      }
      case "user_joined": {
        this.members = [...this.members, { id: msg.id, name: msg.name, role: msg.role }];
        break;
      }
      case "user_left": {
        this.members = this.members.filter((m) => m.id !== msg.id);
        break;
      }
      case "host_changed": {
        this.members = this.members.map((m) => ({
          ...m,
          role: m.id === msg.id ? "host" : "guest",
        }));
        if (this.selfId === msg.id) this.selfRole = "host";
        break;
      }
      case "promoted_to_host": {
        this.selfRole = "host";
        break;
      }
      case "sync_request": {
        if (this.isHost) this.broadcastFullState();
        break;
      }
      case "state_sync": {
        if (!this.isHost) this.applyState(msg);
        break;
      }
      case "track_change": {
        if (!this.isHost && msg.track) this.applyTrack(msg.track);
        break;
      }
      case "play": {
        if (!this.isHost) {
          this.suppressBroadcast = true;
          try {
            if (musicPlayer.currentTrack) {
              musicPlayer.resume();
            }
          } finally {
            queueMicrotask(() => {
              this.suppressBroadcast = false;
            });
          }
        }
        break;
      }
      case "pause": {
        if (!this.isHost) {
          this.suppressBroadcast = true;
          try {
            musicPlayer.pause();
            if (typeof msg.time === "number") {
              musicPlayer.seek(msg.time);
            }
          } finally {
            queueMicrotask(() => {
              this.suppressBroadcast = false;
            });
          }
        }
        break;
      }
      case "seek": {
        if (!this.isHost && typeof msg.time === "number") {
          this.suppressBroadcast = true;
          try {
            musicPlayer.seek(msg.time);
          } finally {
            queueMicrotask(() => {
              this.suppressBroadcast = false;
            });
          }
        }
        break;
      }
      case "chat": {
        this.chat = [
          ...this.chat,
          { id: msg.from, name: msg.name, text: msg.text, at: msg.at },
        ].slice(-50);
        break;
      }
      case "error": {
        this.status = "error";
        this.errorMessage = `${msg.code}: ${msg.message}`;
        this.intentionalDisconnect = true;
        break;
      }
      case "pong":
        break;
    }
  }

  private applyState(msg: { track: MusicTrack; time: number; paused: boolean; at?: number }) {
    const drift = msg.at ? (Date.now() - msg.at) / 1000 : 0;
    const targetTime = msg.time + (msg.paused ? 0 : drift);
    this.suppressBroadcast = true;
    try {
      if (
        !musicPlayer.currentTrack ||
        musicPlayer.currentTrack.id !== msg.track.id
      ) {
        void musicPlayer.play(msg.track);
      }
      musicPlayer.seek(Math.max(0, targetTime));
      if (msg.paused) musicPlayer.pause();
      else musicPlayer.resume();
    } finally {
      queueMicrotask(() => {
        this.suppressBroadcast = false;
      });
    }
  }

  private applyTrack(track: MusicTrack) {
    this.suppressBroadcast = true;
    try {
      void musicPlayer.play(track);
    } finally {
      queueMicrotask(() => {
        this.suppressBroadcast = false;
      });
    }
  }

  private send(msg: Record<string, unknown>) {
    if (!this.ws || this.ws.readyState !== 1) return;
    try {
      this.ws.send(JSON.stringify(msg));
    } catch {
      /* ignore */
    }
  }

  broadcastFullState() {
    if (!this.isHost || !musicPlayer.currentTrack) return;
    this.send({
      type: "state_sync",
      track: musicPlayer.currentTrack,
      time: musicPlayer.currentTime,
      paused: musicPlayer.paused,
    });
  }

  sendChat(text: string) {
    const trimmed = text.trim();
    if (!trimmed) return;
    this.send({ type: "chat", text: trimmed.slice(0, 500) });
  }
}

export const listenTogether = new ListenTogetherStore();
