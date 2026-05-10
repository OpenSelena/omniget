declare global {
  interface Window {
    Hls?: any;
  }
}

const HLS_CDN = "https://cdn.jsdelivr.net/npm/hls.js@1.5.13/dist/hls.min.js";

let loadingPromise: Promise<void> | null = null;

export async function ensureHlsLoaded(): Promise<void> {
  if (typeof window === "undefined") return;
  if (window.Hls) return;
  if (loadingPromise) return loadingPromise;

  loadingPromise = new Promise<void>((resolve, reject) => {
    const existing = document.querySelector(`script[src="${HLS_CDN}"]`);
    if (existing) {
      existing.addEventListener("load", () => resolve());
      existing.addEventListener("error", () =>
        reject(new Error("Falha ao carregar hls.js")),
      );
      return;
    }
    const script = document.createElement("script");
    script.src = HLS_CDN;
    script.async = true;
    script.onload = () => resolve();
    script.onerror = () => reject(new Error("Falha ao carregar hls.js"));
    document.head.appendChild(script);
  }).catch((e) => {
    loadingPromise = null;
    throw e;
  });

  return loadingPromise;
}

export type HlsHandle = {
  destroy: () => void;
};

export async function attachHls(audio: HTMLAudioElement, m3u8Url: string): Promise<HlsHandle> {
  if (audio.canPlayType("application/vnd.apple.mpegurl") !== "") {
    audio.src = m3u8Url;
    return {
      destroy: () => {
        try {
          audio.src = "";
        } catch {
          /* ignore */
        }
      },
    };
  }

  await ensureHlsLoaded();
  if (!window.Hls || !window.Hls.isSupported()) {
    throw new Error("HLS não é suportado neste navegador");
  }

  const hls = new window.Hls({
    enableWorker: false,
    lowLatencyMode: false,
  });
  hls.loadSource(m3u8Url);
  hls.attachMedia(audio);

  return {
    destroy: () => {
      try {
        hls.destroy();
      } catch {
        /* ignore */
      }
    },
  };
}
