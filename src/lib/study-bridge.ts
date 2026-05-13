import { pluginInvoke } from "$lib/plugin-invoke";

export type ContinueWatchingItem = {
  course_id: number;
  course_title: string;
  course_thumbnail: string | null;
  last_lesson_id: number | null;
  last_lesson_title: string | null;
  module_title: string | null;
  time_offset: number;
  duration: number;
  progress_pct: number;
  last_watched: string | null;
  next_lesson_id: number | null;
  next_lesson_title: string | null;
  notification_count: number;
};

export type ContinueWatchingPreviewItem = {
  course_id: number;
  title: string;
  thumbnail: string | null;
  progress_pct: number;
  time_offset: number;
  duration: number;
  next_lesson_id: number | null;
};

export type LibraryByTypeItem = {
  id: number;
  title: string;
  thumbnail_path: string | null;
  last_watched: string | null;
  times_watched: number | null;
};

export type LibraryByTypeGroup = {
  type: "platform" | "subject";
  key: string;
  label: string;
  total: number;
  items: LibraryByTypeItem[];
  subject_id?: number;
};

export type LibraryByTypePage = {
  type: "platform" | "subject";
  key: string;
  items: LibraryByTypeItem[];
  offset: number;
  limit: number;
  total: number;
  has_more: boolean;
  subject_id?: number;
};

export type SubjectCourseItem = {
  id: number;
  title: string;
  thumbnail_path: string | null;
  platform: string | null;
};

export type NotificationsCount = {
  unread: number;
  total: number;
};

export type LibraryItemState = {
  last_watched?: string | null;
  time_watched: number;
  time_offset: number;
  overall_time_watched: number;
  times_watched: number;
  flagged_watched: number;
  duration: number;
  video_id?: number | null;
  playback_speed?: number | null;
};

export type WatchedSummary = {
  course_id: number;
  watched_count: number;
  total: number;
  field: string;
};

export type CourseSubject = {
  id: number;
  name: string;
  color: string | null;
};

export type NotificationFull = {
  id: number;
  course_id: number;
  lesson_id: number | null;
  kind: string;
  detected_at: number;
  read_at: number | null;
  dismissed_at: number | null;
  course_title: string;
  lesson_title: string | null;
};

export async function studyContinueWatching(
  limit?: number,
  threshold?: number,
): Promise<ContinueWatchingItem[]> {
  return pluginInvoke<ContinueWatchingItem[]>("study", "study:library:continue_watching", {
    limit,
    threshold,
  });
}

export async function studyContinueWatchingPreview(
  limit?: number,
): Promise<ContinueWatchingPreviewItem[]> {
  return pluginInvoke<ContinueWatchingPreviewItem[]>(
    "study",
    "study:library:continue_watching_preview",
    { limit },
  );
}

export async function studyLibraryByType(args: {
  grouping?: "platform" | "subject";
  groupLimit?: number;
}): Promise<LibraryByTypeGroup[]> {
  return pluginInvoke<LibraryByTypeGroup[]>("study", "study:library:by_type", args);
}

export async function studyLibraryByTypePage(args: {
  grouping: "platform" | "subject";
  key: string;
  offset?: number;
  limit?: number;
}): Promise<LibraryByTypePage> {
  return pluginInvoke<LibraryByTypePage>("study", "study:library:by_type:page", args);
}

export async function studySubjectsCoursesBySubject(args: {
  subjectId: number;
  limit?: number;
}): Promise<SubjectCourseItem[]> {
  return pluginInvoke<SubjectCourseItem[]>(
    "study",
    "study:library:subjects:courses_by_subject",
    args,
  );
}

export async function studyNotificationsCount(): Promise<NotificationsCount> {
  return pluginInvoke<NotificationsCount>("study", "study:library:notifications:count", {});
}

export async function studyLibraryItemStateGet(courseId: number): Promise<LibraryItemState> {
  return pluginInvoke<LibraryItemState>("study", "study:library:state:get", {
    courseId,
  });
}

export async function studyLibraryItemStateUpdate(
  courseId: number,
  patch: Partial<LibraryItemState>,
): Promise<LibraryItemState> {
  return pluginInvoke<LibraryItemState>("study", "study:library:state:update", {
    courseId,
    patch,
  });
}

export async function studyLibraryWatchedGet(courseId: number): Promise<WatchedSummary> {
  return pluginInvoke<WatchedSummary>("study", "study:library:bitfield:get", {
    courseId,
  });
}

export async function studyLibraryWatchedSet(
  courseId: number,
  field: string,
): Promise<{ course_id: number; watched_count: number; total: number }> {
  return pluginInvoke("study", "study:library:bitfield:set", { courseId, field });
}

export async function studySubjectsListForCourse(courseId: number): Promise<CourseSubject[]> {
  return pluginInvoke<CourseSubject[]>("study", "study:library:subjects:list_for_course", {
    courseId,
  });
}

export async function studySubjectsSetForCourse(
  courseId: number,
  subjectIds: number[],
): Promise<{ course_id: number; added: number; removed: number }> {
  return pluginInvoke("study", "study:library:subjects:set_for_course", {
    courseId,
    subjectIds,
  });
}

export async function studyNotificationsList(args: {
  unreadOnly?: boolean;
}): Promise<NotificationFull[]> {
  return pluginInvoke<NotificationFull[]>("study", "study:library:notifications:list", args);
}

export async function studyNotificationsRead(ids: number[]): Promise<{ updated: number }> {
  return pluginInvoke("study", "study:library:notifications:mark_read", { ids });
}

export async function studyNotificationsDismiss(id: number): Promise<{ id: number; dismissed: boolean }> {
  return pluginInvoke("study", "study:library:notifications:dismiss", { id });
}

export type CatalogSort =
  | "last_watched"
  | "times_watched"
  | "name"
  | "watched"
  | "not_watched"
  | "progress"
  | "added"
  | "platform";

export type CatalogSortDirection = "asc" | "desc";

export type CatalogFilters = {
  search?: string;
  sort?: CatalogSort;
  sort_direction?: CatalogSortDirection;
  platforms?: string[];
  status?: string[];
  favorites_only?: boolean;
  with_notes?: boolean;
  with_cards?: boolean;
  tags?: string[];
  kind?: "course" | "media";
  limit?: number;
  offset?: number;
};

export type CatalogCourseItem = {
  id: number;
  title: string;
  source_path: string;
  thumbnail_path: string | null;
  added_at: string;
  last_scan_at: string;
  favorite: boolean;
  kind: string;
  platform: string | null;
  has_gaps: boolean;
  total_lessons: number;
  completed_lessons: number;
  notes_count: number;
  deck_count: number;
  last_watched_at: number;
  tags: string[];
  status: string;
  progress_pct: number;
};

export async function studyCoursesList(filters: CatalogFilters = {}): Promise<CatalogCourseItem[]> {
  return pluginInvoke<CatalogCourseItem[]>("study", "study:courses:list", { filters });
}

export type SearchHit = {
  id: number;
  title: string;
  thumbnail_path?: string | null;
  platform?: string | null;
  course_id?: number;
  course_title?: string;
  module_id?: number | null;
  module_title?: string | null;
  match_start: number | null;
  match_end: number | null;
};

export type SearchResults = {
  courses: SearchHit[];
  lessons: SearchHit[];
  query: string;
};

export async function studyLibrarySearch(q: string, limit?: number): Promise<SearchResults> {
  return pluginInvoke<SearchResults>("study", "study:library:search", { q, limit });
}

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
  position: number;
  duration_ms: number | null;
};

export type PlayerContext = {
  lesson: {
    id: number;
    course_id: number;
    title: string;
    video_path: string;
    duration_ms: number | null;
    subtitle_path: string | null;
    current_seconds: number;
  };
  course_title: string;
  library_state: LibraryItemState;
  next_lesson: NextLesson | null;
  skip_gaps: SkipGaps | null;
  subtitles: SubtitleTrack[];
  thumbnails: ThumbnailSlice[];
  audio_tracks: AudioTrack[];
};

export async function studyPlayerContext(lessonId: number): Promise<PlayerContext> {
  return pluginInvoke<PlayerContext>("study", "study:player:context", { lessonId });
}

export async function studyPlayerSeek(args: {
  lessonId: number;
  fromMs: number;
  toMs: number;
}): Promise<void> {
  await pluginInvoke("study", "study:player:seek", args);
}

export async function studyPlayerTimeChanged(args: {
  lessonId: number;
  timeMs: number;
  durationMs?: number;
}): Promise<void> {
  await pluginInvoke("study", "study:player:time_changed", args);
}

export async function studyDetectSkipGaps(lessonId: number): Promise<SkipGaps | null> {
  return pluginInvoke<SkipGaps | null>("study", "study:player:detect_skip_gaps", { lessonId });
}

export async function studySkipGapsGet(lessonId: number): Promise<SkipGaps | null> {
  return pluginInvoke<SkipGaps | null>("study", "study:player:skip_gaps:get", { lessonId });
}

export async function studyLessonThumbnails(lessonId: number): Promise<ThumbnailSlice[]> {
  return pluginInvoke<ThumbnailSlice[]>("study", "study:lesson:thumbnails", { lessonId });
}

export async function studyLessonSubtitles(lessonId: number): Promise<SubtitleTrack[]> {
  return pluginInvoke<SubtitleTrack[]>("study", "study:lesson:subtitles", { lessonId });
}

export async function studyLessonAudioTracks(lessonId: number): Promise<AudioTrack[]> {
  return pluginInvoke<AudioTrack[]>("study", "study:lesson:audio_tracks", { lessonId });
}

export async function studyPlayerNextLesson(args: {
  courseId: number;
  currentLessonId?: number;
}): Promise<NextLesson | null> {
  return pluginInvoke<NextLesson | null>("study", "study:player:next_lesson", args);
}

export async function studyPlayerEvent(kind: string, payload: Record<string, unknown>): Promise<void> {
  pluginInvoke("study", "study:player:event", { kind, payload }).catch(() => {});
}

export type AnalyticsContext = {
  id: string;
  type: string;
  name: string;
  video_id: string;
  video_title: string;
  time_ms: number;
  duration_ms: number;
  platform: string | null;
  platform_key: string;
  times_watched: number;
  progress_pct: number;
  device_type: string;
};

export async function studyPlayerAnalyticsContext(args: {
  lessonId: number;
  timeMs?: number;
}): Promise<AnalyticsContext> {
  return pluginInvoke<AnalyticsContext>("study", "study:player:analytics_context", args);
}

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
    audio_secondary_lang?: string;
    thumbnails_auto_generate?: boolean;
    hero_blur_intensity?: number;
    playback_speed?: number;
  };
  library?: {
    watcher_enabled?: boolean;
    scan_hidden?: boolean;
    auto_vacuum?: boolean;
    auto_vacuum_interval_days?: number;
  };
  [key: string]: unknown;
};

export async function studySettingsGet(): Promise<StudySettings> {
  return pluginInvoke<StudySettings>("study", "study:settings:get", {});
}

export async function studySettingsSave(settings: StudySettings): Promise<StudySettings> {
  return pluginInvoke<StudySettings>("study", "study:settings:save", { settings });
}

export async function studyPlayerSpeedGet(courseId: number): Promise<{ course_id: number; playback_speed: number | null }> {
  return pluginInvoke("study", "study:player:speed:get", { courseId });
}

export async function studyPlayerSpeedSet(courseId: number, speed: number | null): Promise<{ course_id: number; playback_speed: number | null }> {
  return pluginInvoke("study", "study:player:speed:set", { courseId, speed });
}

export type SeekLogEntry = {
  id: number;
  from_ms: number;
  to_ms: number;
  at_secs: number;
};

export async function studyPlayerSeekLog(lessonId: number): Promise<SeekLogEntry[]> {
  return pluginInvoke<SeekLogEntry[]>("study", "study:player:seek_log", { lessonId });
}

export type SeekHeatmap = {
  lesson_id: number;
  bucket_ms: number;
  duration_ms: number;
  buckets: number[];
  total_seeks: number;
};

export async function studyPlayerSeekHeatmap(args: {
  lessonId: number;
  bucketSecs?: number;
}): Promise<SeekHeatmap> {
  return pluginInvoke<SeekHeatmap>("study", "study:player:seek_heatmap", args);
}

export type VacuumResult = {
  seek_logs_deleted: number;
  notifications_deleted: number;
  recents_deleted: number;
  seek_cutoff: number;
  notif_cutoff: number;
};

export async function studyLibraryVacuum(): Promise<VacuumResult> {
  return pluginInvoke<VacuumResult>("study", "study:library:vacuum", {});
}

export type ExportedCourseEntry = {
  course_id: number;
  title: string;
  source_path: string | null;
  library_state: LibraryItemState;
  watched_field: string;
  watched_count: number;
  total_lessons: number;
  recents: { opened_at: number; lesson_id: number | null } | null;
};

export type ExportedState = {
  exported_at: string;
  version: number;
  courses: ExportedCourseEntry[];
};

export async function studyLibraryExportState(args: {
  courseIds?: number[];
} = {}): Promise<ExportedState> {
  return pluginInvoke<ExportedState>("study", "study:library:export_state", args);
}

export type ImportMode = "skip" | "overwrite" | "merge";

export type ImportResult = {
  imported: number;
  skipped: number;
  missing: number;
  mode: ImportMode;
};

export async function studyLibraryImportState(args: {
  payload: ExportedState;
  mode: ImportMode;
}): Promise<ImportResult> {
  return pluginInvoke<ImportResult>("study", "study:library:import_state", args);
}

export type RecommendedCourseItem = {
  id: number;
  title: string;
  thumbnail_path: string | null;
  platform: string | null;
  score: number;
  tag_overlap: number;
  tags: string[];
};

export async function studyLibraryRecommendations(args: {
  courseId: number;
  limit?: number;
}): Promise<RecommendedCourseItem[]> {
  return pluginInvoke<RecommendedCourseItem[]>("study", "study:library:recommendations", args);
}

export type RecentlyAddedCourse = {
  id: number;
  title: string;
  thumbnail_path: string | null;
  added_at: string;
  total_lessons: number;
  completed_lessons: number;
  progress_pct: number;
  tags: string[];
  platform: string | null;
};

export async function studyCoursesRecentlyAdded(args: {
  limit?: number;
  days?: number;
}): Promise<RecentlyAddedCourse[]> {
  return pluginInvoke<RecentlyAddedCourse[]>("study", "study:courses:recently_added", args);
}

export type SubjectActivityRow = {
  subject_id: number | null;
  subject_name: string;
  subject_color: string | null;
  focus_minutes: number;
  player_seconds: number;
  course_count: number;
  lesson_count: number;
};

export async function studySubjectsActivity(args: {
  days?: number;
}): Promise<SubjectActivityRow[]> {
  return pluginInvoke<SubjectActivityRow[]>("study", "study:subjects:activity", args);
}

export type YoutubeStreamFormat = {
  itag: number;
  url: string | null;
  mime_type: string;
  bitrate: number;
  width: number | null;
  height: number | null;
  fps: number | null;
  quality_label: string | null;
  audio_quality: string | null;
  audio_sample_rate: number | null;
  content_length: number | null;
  approx_duration_ms: number | null;
  last_modified: number | null;
  is_adaptive: boolean;
};

export type YoutubeVideoDetails = {
  video_id: string;
  title: string;
  author: string;
  channel_id: string;
  length_seconds: number;
  is_live: boolean;
  view_count: number;
  keywords: string[];
};

export type YoutubeChapter = {
  start_ms: number;
  end_ms: number;
  title: string;
};

export type YoutubePlayerResult = {
  video_id: string;
  status: { state: string; reason: string | null; playable: boolean };
  video_details: YoutubeVideoDetails;
  formats: YoutubeStreamFormat[];
  adaptive_formats: YoutubeStreamFormat[];
  dash_manifest_url: string | null;
  hls_manifest_url: string | null;
  expires_at: number;
  has_potoken: boolean;
  client_used: string;
  cached: boolean;
  chapters?: YoutubeChapter[];
};

export async function studyYoutubePlayer(args: {
  videoId: string;
}): Promise<YoutubePlayerResult> {
  return pluginInvoke<YoutubePlayerResult>("study", "study:youtube:player", {
    video_id: args.videoId,
  });
}

export async function studyYoutubePlayerCacheClear(): Promise<{ ok: boolean }> {
  return pluginInvoke<{ ok: boolean }>("study", "study:youtube:player_cache_clear", {});
}

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

export type SponsorBlockLookupResult = {
  video_id: string;
  segments: SponsorBlockSegment[];
  error?: string;
};

export async function studyYoutubeSponsorblock(args: {
  videoId: string;
  categories?: SponsorBlockCategory[] | string[];
}): Promise<SponsorBlockLookupResult> {
  return pluginInvoke<SponsorBlockLookupResult>("study", "study:youtube:sponsorblock", {
    video_id: args.videoId,
    categories: args.categories,
  });
}

export type YoutubeChaptersResult = {
  video_id_or_url: string;
  chapters: YoutubeChapter[];
};

export async function studyYoutubeChapters(args: {
  videoId: string;
}): Promise<YoutubeChaptersResult> {
  return pluginInvoke<YoutubeChaptersResult>("study", "study:youtube:chapters", {
    video_id: args.videoId,
  });
}

export type YoutubeSearchFilter = "all" | "music" | "videos" | "channels" | "playlists" | "live";

export type YoutubeSearchVideoItem = {
  kind: "video";
  video_id: string;
  title: string;
  channel_id: string | null;
  channel_title: string | null;
  thumbnail_url: string | null;
  duration_text: string | null;
  published_time_text: string | null;
  view_count_text: string | null;
  short_description: string | null;
  is_live: boolean;
};

export type YoutubeSearchChannelItem = {
  kind: "channel";
  channel_id: string;
  title: string;
  handle: string | null;
  avatar_url: string | null;
  subscribers_text: string | null;
  video_count_text: string | null;
  description: string | null;
};

export type YoutubeSearchPlaylistItem = {
  kind: "playlist";
  playlist_id: string;
  title: string;
  thumbnail_url: string | null;
  video_count: number | null;
  channel_title: string | null;
};

export type YoutubeSearchMixItem = {
  kind: "mix";
  playlist_id: string;
  title: string;
  thumbnail_url: string | null;
  first_video_id: string | null;
};

export type YoutubeSearchShelfItem = {
  kind: "shelf";
  title: string;
  items: YoutubeSearchItem[];
};

export type YoutubeSearchItem =
  | YoutubeSearchVideoItem
  | YoutubeSearchChannelItem
  | YoutubeSearchPlaylistItem
  | YoutubeSearchMixItem
  | YoutubeSearchShelfItem;

export type YoutubeSearchResultGroup = {
  kind: string;
  title: string | null;
  items: YoutubeSearchItem[];
};

export type YoutubeSearchResult = {
  query: string;
  filter: string;
  estimated_results: number | null;
  groups: YoutubeSearchResultGroup[];
  continuation: string | null;
};

export async function studyYoutubeSearch(args: {
  query: string;
  filter?: YoutubeSearchFilter;
}): Promise<YoutubeSearchResult> {
  return pluginInvoke<YoutubeSearchResult>("study", "study:youtube:search", {
    query: args.query,
    filter: args.filter,
  });
}

export type YoutubeChannelTab = "home" | "videos" | "playlists" | "shorts" | "live" | "about";

export type YoutubeChannelHeader = {
  channel_id: string;
  title: string;
  handle: string | null;
  avatar_url: string | null;
  banner_url: string | null;
  subscribers_text: string | null;
  video_count_text: string | null;
  description: string | null;
};

export type YoutubeChannel = {
  header: YoutubeChannelHeader;
  tab: string;
  items: YoutubeSearchItem[];
  continuation: string | null;
};

export async function studyYoutubeChannel(args: {
  channelId: string;
  tab?: YoutubeChannelTab;
}): Promise<YoutubeChannel> {
  return pluginInvoke<YoutubeChannel>("study", "study:youtube:channel", {
    channel_id: args.channelId,
    tab: args.tab,
  });
}

export type YoutubePlaylistVideo = {
  video_id: string;
  title: string;
  channel_id: string | null;
  channel_title: string | null;
  thumbnail_url: string | null;
  duration_text: string | null;
  duration_seconds: number | null;
  position: number;
};

export type YoutubePlaylistInfo = {
  playlist_id: string;
  title: string;
  description: string | null;
  thumbnail_url: string | null;
  video_count: number | null;
  view_count_text: string | null;
  owner_title: string | null;
  owner_channel_id: string | null;
};

export type YoutubePlaylist = {
  info: YoutubePlaylistInfo;
  videos: YoutubePlaylistVideo[];
  continuation: string | null;
};

export async function studyYoutubePlaylist(args: {
  playlistId: string;
}): Promise<YoutubePlaylist> {
  return pluginInvoke<YoutubePlaylist>("study", "study:youtube:playlist", {
    playlist_id: args.playlistId,
  });
}

export type YoutubeTrendingCategory = "now" | "music" | "gaming" | "movies";

export type YoutubeTrendingResult = {
  category: string;
  items: YoutubeSearchItem[];
};

export async function studyYoutubeTrending(args?: {
  category?: YoutubeTrendingCategory;
}): Promise<YoutubeTrendingResult> {
  return pluginInvoke<YoutubeTrendingResult>("study", "study:youtube:trending", {
    category: args?.category,
  });
}

export type YoutubeMoodCategory = {
  title: string;
  mood_id: string;
  params: string | null;
  color: string | null;
};

export type YoutubeMoodSection = {
  title: string;
  items: YoutubeMoodCategory[];
};

export type YoutubeMoodsResult = {
  sections: YoutubeMoodSection[];
};

export async function studyYoutubeMoodsAndGenres(): Promise<YoutubeMoodsResult> {
  return pluginInvoke<YoutubeMoodsResult>("study", "study:youtube:moods_and_genres", {});
}

export type YoutubeMoodPlaylistItem = {
  playlist_id: string;
  title: string;
  subtitle: string | null;
  thumbnail_url: string | null;
};

export type YoutubeMoodShelf = {
  title: string;
  items: YoutubeMoodPlaylistItem[];
};

export type YoutubeMoodDetail = {
  mood_id: string;
  title: string | null;
  shelves: YoutubeMoodShelf[];
};

export async function studyYoutubeMoodDetail(args: {
  moodId: string;
  params?: string | null;
}): Promise<YoutubeMoodDetail> {
  return pluginInvoke<YoutubeMoodDetail>("study", "study:youtube:mood_detail", {
    mood_id: args.moodId,
    params: args.params ?? undefined,
  });
}

export type YoutubeNewReleaseAlbum = {
  browse_id: string;
  playlist_id: string | null;
  title: string;
  artist: string | null;
  year: string | null;
  thumbnail_url: string | null;
};

export type YoutubeNewReleasesResult = {
  albums: YoutubeNewReleaseAlbum[];
};

export async function studyYoutubeNewReleases(): Promise<YoutubeNewReleasesResult> {
  return pluginInvoke<YoutubeNewReleasesResult>("study", "study:youtube:new_releases", {});
}

export type LoopCookieStatus = {
  available: boolean;
  file_path: string;
  has_youtube: boolean;
};

export async function studyMusicLoopCookieStatus(): Promise<LoopCookieStatus> {
  return pluginInvoke<LoopCookieStatus>("study", "study:music:loop_cookie_status", {});
}

export type YoutubePotokenStatus = {
  engine: string;
  snapshot_version: string;
  minting_available: boolean;
  minting_status: string;
  visitor: {
    has_token: boolean;
    expires_in_seconds: number | null;
    expired: boolean | null;
  };
  content_cached_count: number;
  last_generated_at: number | null;
};

export async function studyYoutubePotokenStatus(): Promise<YoutubePotokenStatus> {
  return pluginInvoke<YoutubePotokenStatus>("study", "study:youtube:potoken_status", {});
}

export type YoutubeSubscription = {
  channel_id: string;
  title: string;
  thumbnail_url: string | null;
  added_at: number;
  last_notified_at: number | null;
};

export async function studyYoutubeSubsList(): Promise<{ subs: YoutubeSubscription[] }> {
  return pluginInvoke<{ subs: YoutubeSubscription[] }>("study", "study:youtube:subs:list", {});
}

export async function studyYoutubeSubsAdd(args: {
  channelId: string;
  title: string;
  thumbnailUrl?: string;
}): Promise<{ ok: boolean; channel_id: string }> {
  return pluginInvoke("study", "study:youtube:subs:add", {
    channel_id: args.channelId,
    title: args.title,
    thumbnail_url: args.thumbnailUrl,
  });
}

export async function studyYoutubeSubsRemove(args: {
  channelId: string;
}): Promise<{ ok: boolean; removed: boolean }> {
  return pluginInvoke("study", "study:youtube:subs:remove", {
    channel_id: args.channelId,
  });
}

export async function studyYoutubeSubsIsSubscribed(args: {
  channelId: string;
}): Promise<{ subscribed: boolean }> {
  return pluginInvoke("study", "study:youtube:subs:is_subscribed", {
    channel_id: args.channelId,
  });
}

export type YoutubeSubsFeedEntry = {
  channel_id: string;
  channel_title: string;
  items: YoutubeSearchItem[];
  error: string | null;
};

export async function studyYoutubeSubsFeed(): Promise<{ entries: YoutubeSubsFeedEntry[] }> {
  return pluginInvoke<{ entries: YoutubeSubsFeedEntry[] }>("study", "study:youtube:subs:feed", {});
}

export type MusicHistorySource = "local" | "spotify" | "youtube" | "soundcloud";

export type MusicHistoryEntry = {
  id: number;
  source: MusicHistorySource;
  external_id: string | null;
  track_id: number | null;
  title: string;
  artist: string | null;
  album: string | null;
  cover_url: string | null;
  duration_ms: number | null;
  position_ms: number;
  played_at: number;
  metadata: unknown | null;
};

export async function studyMusicHistoryAdd(args: {
  source: MusicHistorySource;
  title: string;
  externalId?: string;
  trackId?: number;
  artist?: string;
  album?: string;
  coverUrl?: string;
  durationMs?: number;
  positionMs?: number;
  metadata?: Record<string, unknown>;
}): Promise<{ ok: boolean; id: number; played_at: number }> {
  return pluginInvoke("study", "study:music:history:add", {
    source: args.source,
    title: args.title,
    external_id: args.externalId,
    track_id: args.trackId,
    artist: args.artist,
    album: args.album,
    cover_url: args.coverUrl,
    duration_ms: args.durationMs,
    position_ms: args.positionMs ?? 0,
    metadata: args.metadata,
  });
}

export async function studyMusicHistoryList(args?: {
  limit?: number;
  source?: MusicHistorySource;
}): Promise<{ entries: MusicHistoryEntry[] }> {
  return pluginInvoke<{ entries: MusicHistoryEntry[] }>("study", "study:music:history:list", {
    limit: args?.limit,
    source: args?.source,
  });
}

export async function studyMusicHistoryClear(args?: {
  source?: MusicHistorySource;
}): Promise<{ ok: boolean; removed: number }> {
  return pluginInvoke("study", "study:music:history:clear", {
    source: args?.source,
  });
}

export async function studyMusicHistoryRemove(args: {
  id: number;
}): Promise<{ ok: boolean; removed: number }> {
  return pluginInvoke("study", "study:music:history:remove", {
    id: args.id,
  });
}

export type MusicQuickPickEntry = {
  source: MusicHistorySource;
  external_id: string | null;
  track_id: number | null;
  title: string;
  artist: string | null;
  cover_url: string | null;
  origin: "favorite" | "history" | "trending";
};

export async function studyMusicQuickPicks(): Promise<{
  entries: MusicQuickPickEntry[];
  seed: number;
}> {
  return pluginInvoke<{ entries: MusicQuickPickEntry[]; seed: number }>(
    "study",
    "study:music:quick_picks",
    {},
  );
}

export type MusicContinueEntry = {
  source: MusicHistorySource;
  external_id: string | null;
  track_id: number | null;
  title: string;
  artist: string | null;
  album: string | null;
  cover_url: string | null;
  duration_ms: number | null;
  position_ms: number;
  played_at: number;
  progress: number;
};

export async function studyMusicContinueListening(args?: {
  limit?: number;
}): Promise<{ entries: MusicContinueEntry[] }> {
  return pluginInvoke<{ entries: MusicContinueEntry[] }>(
    "study",
    "study:music:continue_listening",
    { limit: args?.limit },
  );
}

export type MusicDiscoverEntry = {
  external_id: string;
  title: string;
  artist: string | null;
  album: string | null;
  cover_url: string | null;
  duration_ms: number | null;
};

export async function studyMusicDailyDiscover(args?: {
  refresh?: boolean;
}): Promise<{
  entries: MusicDiscoverEntry[];
  seed_video_id: string | null;
  seed_title: string | null;
  from_cache: boolean;
}> {
  return pluginInvoke<{
    entries: MusicDiscoverEntry[];
    seed_video_id: string | null;
    seed_title: string | null;
    from_cache: boolean;
  }>("study", "study:music:daily_discover", { refresh: args?.refresh ?? false });
}

export type YoutubeUserPlaylistSummary = {
  id: number;
  title: string;
  description: string | null;
  created_at: number;
  updated_at: number;
  item_count: number;
};

export type YoutubeUserPlaylistItem = {
  video_id: string;
  position: number;
  added_at: number;
  title: string | null;
  channel_title: string | null;
  thumbnail_url: string | null;
  duration_ms: number | null;
};

export type YoutubeUserPlaylistDetail = {
  summary: YoutubeUserPlaylistSummary;
  items: YoutubeUserPlaylistItem[];
};

export async function studyYoutubeUserPlaylistCreate(args: {
  title: string;
  description?: string;
}): Promise<{ id: number }> {
  return pluginInvoke<{ id: number }>("study", "study:youtube:user_playlist:create", args);
}

export async function studyYoutubeUserPlaylistRename(args: {
  playlistId: number;
  title: string;
  description?: string;
}): Promise<{ ok: boolean; updated: boolean }> {
  return pluginInvoke("study", "study:youtube:user_playlist:rename", {
    playlist_id: args.playlistId,
    title: args.title,
    description: args.description,
  });
}

export async function studyYoutubeUserPlaylistDelete(args: {
  playlistId: number;
}): Promise<{ ok: boolean; removed: boolean }> {
  return pluginInvoke("study", "study:youtube:user_playlist:delete", {
    playlist_id: args.playlistId,
  });
}

export async function studyYoutubeUserPlaylistList(): Promise<{
  playlists: YoutubeUserPlaylistSummary[];
}> {
  return pluginInvoke<{ playlists: YoutubeUserPlaylistSummary[] }>(
    "study",
    "study:youtube:user_playlist:list",
    {},
  );
}

export async function studyYoutubeUserPlaylistGet(args: {
  playlistId: number;
}): Promise<YoutubeUserPlaylistDetail> {
  return pluginInvoke<YoutubeUserPlaylistDetail>("study", "study:youtube:user_playlist:get", {
    playlist_id: args.playlistId,
  });
}

export async function studyYoutubeUserPlaylistAddItem(args: {
  playlistId: number;
  item: {
    video_id: string;
    title?: string;
    channel_title?: string;
    thumbnail_url?: string;
    duration_ms?: number;
  };
  position?: number;
}): Promise<{ ok: boolean; position: number }> {
  return pluginInvoke("study", "study:youtube:user_playlist:add_item", {
    playlist_id: args.playlistId,
    item: args.item,
    position: args.position,
  });
}

export async function studyYoutubeUserPlaylistRemoveItem(args: {
  playlistId: number;
  videoId: string;
}): Promise<{ ok: boolean; removed: boolean }> {
  return pluginInvoke("study", "study:youtube:user_playlist:remove_item", {
    playlist_id: args.playlistId,
    video_id: args.videoId,
  });
}

export async function studyYoutubeUserPlaylistReorder(args: {
  playlistId: number;
  videoIds: string[];
}): Promise<{ ok: boolean }> {
  return pluginInvoke("study", "study:youtube:user_playlist:reorder", {
    playlist_id: args.playlistId,
    video_ids: args.videoIds,
  });
}

export type YoutubeEndpointMetrics = {
  endpoint: string;
  success_count: number;
  fail_count: number;
  last_success_ts: number | null;
  last_fail_ts: number | null;
  last_error: string | null;
};

export async function studyYoutubeMetricsGet(): Promise<{
  metrics: YoutubeEndpointMetrics[];
}> {
  return pluginInvoke<{ metrics: YoutubeEndpointMetrics[] }>(
    "study",
    "study:youtube:metrics:get",
    {},
  );
}

export async function studyYoutubeMetricsReset(): Promise<{ removed: number }> {
  return pluginInvoke<{ removed: number }>("study", "study:youtube:metrics:reset", {});
}

export async function studyYoutubeHumanizeError(args: {
  error: string;
}): Promise<{ humanized: string; original: string }> {
  return pluginInvoke<{ humanized: string; original: string }>(
    "study",
    "study:youtube:humanize_error",
    { error: args.error },
  );
}

export async function studySoundcloudHumanizeError(args: {
  error: string;
}): Promise<{ humanized: string; original: string }> {
  return pluginInvoke<{ humanized: string; original: string }>(
    "study",
    "study:soundcloud:humanize_error",
    { error: args.error },
  );
}

export type LocalArtistTrack = {
  id: number;
  path: string;
  title: string | null;
  artist: string | null;
  album: string | null;
  album_artist: string | null;
  year: number | null;
  duration_ms: number | null;
  cover_path: string | null;
  play_count: number;
  favorite: boolean;
};

export async function studyMusicArtistTracksLocal(args: {
  artist: string;
  limit?: number;
}): Promise<{ artist: string; normalized: string; tracks: LocalArtistTrack[] }> {
  return pluginInvoke<{ artist: string; normalized: string; tracks: LocalArtistTrack[] }>(
    "study",
    "study:music:artist:tracks_local",
    { artist: args.artist, limit: args.limit ?? 100 },
  );
}

export type AlbumLookupHit = {
  playlist_id: string;
  title: string;
  thumbnail_url: string | null;
  video_count: number | null;
  channel_title: string | null;
  score: number;
};

export async function studyMusicAlbumOnlineLookup(args: {
  album: string;
  artist?: string;
  year?: number;
}): Promise<{ query: string; best: AlbumLookupHit | null; candidates: AlbumLookupHit[] }> {
  return pluginInvoke<{ query: string; best: AlbumLookupHit | null; candidates: AlbumLookupHit[] }>(
    "study",
    "study:music:album:online_lookup",
    { album: args.album, artist: args.artist, year: args.year },
  );
}
