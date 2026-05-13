import Hls from "hls.js";

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
        } catch {}
      },
    };
  }

  if (!Hls.isSupported()) {
    throw new Error("HLS nao e suportado neste navegador");
  }

  const hls = new Hls({
    enableWorker: false,
    lowLatencyMode: false,
  });
  hls.loadSource(m3u8Url);
  hls.attachMedia(audio);

  return {
    destroy: () => {
      try {
        hls.destroy();
      } catch {}
    },
  };
}
