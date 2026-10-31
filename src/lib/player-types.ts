export type SubtitleTrack = {
  path: string;
  lang: string;
  label: string;
  format: string;
  default: boolean;
};

export type AudioTrack = {
  path: string;
  lang: string;
  label: string;
  format: string;
  default: boolean;
};

export type SkipGaps = {
  lesson_id: number;
  intro_from_ms: number | null;
  intro_to_ms: number | null;
  outro_from_ms: number | null;
  source: string;
  computed_at?: number;
};

export type ThumbnailSlice = {
  start_ms: number;
  end_ms: number;
  sprite_url: string;
  x: number | null;
  y: number | null;
  w: number | null;
  h: number | null;
};

export type NextLesson = {
  id: number;
  title: string;
  module_id: number | null;
  module_title: string | null;
};

export type StudySettings = {
  player?: {
    default_rate?: number;
    theater_mode?: boolean;
    remember_mute?: boolean;
    seek_seconds?: number;
    seek_step_long_ms?: number;
    seek_step_short_ms?: number;
    auto_resume?: boolean;
    completion_threshold?: number;
    autoplay_next?: boolean;
    autoplay_delay_sec?: number;
    binge_watching?: boolean;
    next_video_notification_ms?: number;
    pause_on_minimize?: boolean;
    esc_exit_fullscreen?: boolean;
    collect_seek_logs?: boolean;
    subtitles_default_lang?: string;
    subtitles_secondary_lang?: string;
    subtitles_size?: number;
    subtitles_offset_ms?: number;
    subtitles_text_color?: string;
    subtitles_background_color?: string;
    subtitles_outline_color?: string;
    subtitles_opacity?: number;
    subtitles_font?: string;
    subtitles_bold?: boolean;
    ass_subtitles_styling?: boolean;
    audio_default_lang?: string;
  };
};

export type YoutubeChapter = {
  start_ms: number;
  end_ms: number;
  title: string;
};

export type SponsorBlockCategory =
  | "sponsor"
  | "selfpromo"
  | "interaction"
  | "intro"
  | "outro"
  | "preview"
  | "music_offtopic"
  | "filler";

export type SponsorBlockSegment = {
  category: SponsorBlockCategory | string;
  action_type: string;
  start_ms: number;
  end_ms: number;
  uuid: string;
  votes: number;
};
