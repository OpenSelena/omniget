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
