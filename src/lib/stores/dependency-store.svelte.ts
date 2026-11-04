import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { YtdlpSetup } from "#lib/ytdlp-banner.ts";

let ytdlpAvailable: boolean = $state(false);
let checked: boolean = $state(false);
let setup: YtdlpSetup = $state({ state: "idle" });
let listening = false;

export function isYtdlpAvailable(): boolean {
  return ytdlpAvailable;
}

export function isDepsChecked(): boolean {
  return checked;
}

export function getYtdlpSetup(): YtdlpSetup {
  return setup;
}

export async function refreshYtdlpStatus(): Promise<void> {
  try {
    ytdlpAvailable = await invoke<boolean>("check_ytdlp_available");
  } catch {
    ytdlpAvailable = false;
  }
  checked = true;
}

function applySetup(next: YtdlpSetup): void {
  setup = next;
  if (next.state === "ready") void refreshYtdlpStatus();
}

/** Follows the automatic yt-dlp install started by the backend. */
export async function initYtdlpSetup(): Promise<void> {
  if (!listening) {
    listening = true;
    try {
      await listen<YtdlpSetup>("ytdlp-setup", (e) => applySetup(e.payload));
    } catch {
      listening = false;
    }
  }
  try {
    applySetup(await invoke<YtdlpSetup>("ytdlp_setup_status"));
  } catch {
    // Older backend without the command: keep the plain availability check.
  }
}

/** The banner's "Retry now". */
export async function retryYtdlpSetup(): Promise<void> {
  setup = { state: "installing", downloaded: 0, total: null };
  try {
    await invoke("ytdlp_setup_retry");
  } catch {
    await initYtdlpSetup();
  }
}
