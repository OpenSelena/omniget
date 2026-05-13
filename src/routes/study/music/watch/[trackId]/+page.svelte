<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { musicPlayer } from "$lib/study-music/player-store.svelte";
  import { t } from "$lib/i18n";
  import PlayerShell from "$lib/study-components/player/PlayerShell.svelte";
  import { studySettingsGet } from "$lib/study-bridge";

  const trackId = $derived.by(() => {
    const raw = $page.params.trackId ?? "";
    const v = Number.parseInt(raw, 10);
    return Number.isFinite(v) ? v : null;
  });

  const track = $derived.by(() => {
    if (trackId == null) return null;
    if (musicPlayer.currentTrack?.id === trackId) return musicPlayer.currentTrack;
    return musicPlayer.queue.find((q) => q.id === trackId) ?? null;
  });

  const videoUrl = $derived(musicPlayer.youtubeVideoUrl);
  const chapters = $derived(musicPlayer.youtubeChapters);
  const sponsorBlockSegments = $derived(musicPlayer.youtubeSponsorBlockSegments);

  let snapshot = $state<{ currentTime: number; wasPlaying: boolean } | null>(null);
  let videoEl: HTMLVideoElement | null = null;
  let lastVideoTime = $state(0);
  let lastVideoPaused = $state(true);
  let sponsorBlockAutoSkip = $state(false);

  onMount(() => {
    const snap = musicPlayer.suspendAudioForWatch();
    snapshot = snap;
    lastVideoTime = snap.currentTime;
    lastVideoPaused = !snap.wasPlaying;
    void studySettingsGet()
      .then((s) => {
        const music = (s as unknown as { music?: { sponsorblock_auto_skip?: boolean } }).music;
        sponsorBlockAutoSkip = Boolean(music?.sponsorblock_auto_skip ?? false);
      })
      .catch(() => {});
  });

  onDestroy(() => {
    if (!snapshot) return;
    const resumeAt = videoEl?.currentTime ?? lastVideoTime;
    const shouldPlay = videoEl ? !videoEl.paused : snapshot.wasPlaying;
    musicPlayer.resumeAudioFromWatch(resumeAt, shouldPlay);
  });

  function handleVideoEl(el: HTMLVideoElement | null) {
    videoEl = el;
  }

  function handleLoadedMetadata() {
    if (!videoEl) return;
    if (snapshot?.wasPlaying) {
      videoEl.play().catch(() => {});
    }
  }

  function close() {
    goto("/study/music");
  }

  function onTimeUpdate(cur: number, _dur: number) {
    lastVideoTime = cur;
    musicPlayer.currentTime = cur;
  }
  function onPlay() {
    lastVideoPaused = false;
    musicPlayer.paused = false;
  }
  function onPause() {
    lastVideoPaused = true;
    musicPlayer.paused = true;
  }
  function onSeek(_fromMs: number, toMs: number) {
    const cur = toMs / 1000;
    lastVideoTime = cur;
    musicPlayer.currentTime = cur;
  }
  function onEnded() {
    musicPlayer.next();
  }
</script>

<section class="watch-overlay">
  {#if !track}
    <div class="watch-error">
      <h2>{$t("study.music.watch_video_unavailable")}</h2>
      <button type="button" class="back-btn" onclick={close}>
        {$t("study.music.watch_back")}
      </button>
    </div>
  {:else if !videoUrl}
    <div class="watch-error">
      <h2>{$t("study.music.watch_video_unavailable")}</h2>
      <p class="muted">{track.title ?? track.path}</p>
      <button type="button" class="back-btn" onclick={close}>
        {$t("study.music.watch_back")}
      </button>
    </div>
  {:else}
    <div class="player-frame">
      <PlayerShell
        videoSrc={videoUrl}
        title={track.title ?? track.path}
        courseTitle={track.artist ?? ""}
        backHref="/study/music"
        durationMs={track.duration_ms ?? null}
        initialSeconds={snapshot?.currentTime ?? 0}
        initialPlaybackSpeed={1.0}
        subtitles={[]}
        audioTracks={[]}
        skipGaps={null}
        thumbnails={[]}
        sponsorBlockSegments={sponsorBlockSegments}
        {sponsorBlockAutoSkip}
        {chapters}
        settings={null}
        selectedSubtitleLang={null}
        selectedAudioLang={null}
        theaterMode={false}
        {onTimeUpdate}
        {onSeek}
        {onPlay}
        {onPause}
        {onEnded}
        onLoadedMetadata={handleLoadedMetadata}
        onSubtitleChange={() => {}}
        onAudioChange={() => {}}
        onSpeedChange={() => {}}
        onSkipGap={() => {}}
        onTheaterToggle={() => {}}
        onClose={close}
        onVideoEl={handleVideoEl}
      />
    </div>
  {/if}
</section>

<style>
  .watch-overlay {
    position: fixed;
    inset: 0;
    z-index: 95;
    background: #000;
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .player-frame {
    width: min(100%, 1600px);
    aspect-ratio: 16 / 9;
    max-height: calc(100vh - 48px);
  }
  .watch-error {
    color: rgba(255, 255, 255, 0.92);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    text-align: center;
  }
  .watch-error h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
  }
  .muted {
    margin: 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 14px;
  }
  .back-btn {
    margin-top: 8px;
    padding: 10px 24px;
    background: var(--accent, white);
    color: var(--on-accent, black);
    border: 0;
    border-radius: 999px;
    font-family: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }
</style>
