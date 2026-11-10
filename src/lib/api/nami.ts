import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface Profile {
  url: string;
  username?: string;
  platform: string;
  added_at: number;
  last_pulled_at?: number | null;
}

export interface DownloadResult {
  success: boolean;
  message: string;
  files_count: number;
  cancelled: boolean;
}

export interface DownloadProgress {
  message: string;
  files_downloaded: number;
  stage?: string | null;
  stage_index?: number | null;
  stage_total?: number | null;
}

export interface AppSettings {
  output_directory: string | null;
  cookies_file: string | null;
  browser_cookies?: string | null;
}

export interface DetectedBrowser {
  id: string;
  name: string;
  engine: string;
  profile_dir: string;
  available: boolean;
  is_default: boolean;
}

export interface DependencyStatus {
  ok: boolean;
  message: string;
  gallery_dl_version: string | null;
  yt_dlp_version?: string | null;
  auto_provision_available?: boolean;
}

export async function checkPythonDependencies(): Promise<DependencyStatus> {
  return await invoke<DependencyStatus>('nami_check_python_dependencies');
}

export interface HttpRequestOptions {
  url: string;
  method?: string;
  headers?: Record<string, string>;
  body?: string | null;
  cookie_path?: string | null;
  timeout_ms?: number;
}

export interface HttpResponsePayload {
  status: number;
  ok: boolean;
  headers: Record<string, string>;
  body: string;
}

export async function httpFetch(options: HttpRequestOptions): Promise<HttpResponsePayload> {
  return await invoke<HttpResponsePayload>('nami_http_fetch', { options });
}

export async function ensureBinaries(): Promise<DependencyStatus> {
  return await invoke<DependencyStatus>('nami_ensure_binaries');
}

export async function runGalleryDlDownload(
  url: string,
  outputDir: string,
  cookiesFile: string | null,
  contentType: string,
  downloadId: string,
  skipReplies?: boolean
): Promise<DownloadResult> {
  return await invoke<DownloadResult>('nami_run_gallery_dl_download', {
    url,
    outputDir,
    cookiesFile,
    contentType,
    downloadId,
    skipReplies: skipReplies ?? null
  });
}

export async function cancelDownload(downloadId: string): Promise<string> {
  return await invoke<string>('nami_cancel_download', { downloadId });
}

export async function listenToDownloadProgress(
  downloadId: string,
  callback: (progress: DownloadProgress) => void
): Promise<UnlistenFn> {
  return await listen<DownloadProgress>(`download_${downloadId}`, (event) => {
    callback(event.payload);
  });
}

export async function saveAppSettings(
  outputDirectory: string | null,
  cookiesFile: string | null,
  browserCookies?: string | null
): Promise<string> {
  return await invoke<string>('nami_save_app_settings', {
    outputDirectory,
    cookiesFile,
    browserCookies: browserCookies ?? null
  });
}

export async function detectBrowsers(): Promise<DetectedBrowser[]> {
  return await invoke<DetectedBrowser[]>('nami_detect_browsers');
}

export async function extractBrowserCookies(
  browserId: string,
  platform: string
): Promise<string | null> {
  return await invoke<string | null>('nami_extract_browser_cookies', {
    browserId,
    platform
  });
}

export async function loadAppSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>('nami_load_app_settings');
}

export async function loadProfiles(platform: string): Promise<Profile[]> {
  return await invoke<Profile[]>('nami_load_profiles', { platform });
}

export async function saveProfile(platform: string, url: string): Promise<string> {
  return await invoke<string>('nami_save_profile', { platform, url });
}

export async function deleteProfile(platform: string, profileUrl: string): Promise<string> {
  return await invoke<string>('nami_delete_profile', { platform, profileUrl });
}

export function generateDownloadId(): string {
  return `dl_${Date.now()}_${Math.random().toString(36).slice(2, 11)}`;
}

export interface NamiMediaRecord {
  id: string;
  platform: string;
  post_id: string;
  author?: string | null;
  url: string;
  file_path: string;
  file_size?: number | null;
  sha256?: string | null;
  downloaded_at: number;
}

export interface ReconcileResult {
  checked: number;
  removed: number;
}

export async function loadNamiHistory(
  platform?: string | null,
  limit = 50,
  offset = 0
): Promise<NamiMediaRecord[]> {
  return await invoke<NamiMediaRecord[]>('nami_get_history', {
    platform: platform ?? null,
    limit,
    offset,
  });
}

export async function reconcileNamiDisk(): Promise<ReconcileResult> {
  return await invoke<ReconcileResult>('nami_reconcile_disk');
}

export async function deleteNamiHistoryRecord(id: string): Promise<boolean> {
  return await invoke<boolean>('nami_delete_history_record', { id });
}

export interface PullResult {
  success: boolean;
  profile_url: string;
  platform: string;
  username?: string | null;
  files_downloaded: number;
  message: string;
  early_exited: boolean;
}

export interface PullBatchProgress {
  current_index: number;
  total: number;
  profile_url: string;
  platform: string;
  username?: string | null;
  files_downloaded: number;
  status: string;
}

export interface PullBatchSummary {
  total: number;
  succeeded: number;
  failed: number;
  total_downloaded: number;
  results: PullResult[];
}

export async function pullCreator(
  profileUrl: string,
  platform: string,
  contentType = 'all',
  earlyExitThreshold = 5,
  downloadId?: string
): Promise<PullResult> {
  return await invoke<PullResult>('nami_pull_creator', {
    profileUrl,
    platform,
    contentType,
    earlyExitThreshold,
    downloadId: downloadId ?? null,
  });
}

export async function pullAllCreators(
  platform?: string | null,
  contentType = 'all',
  earlyExitThreshold = 5
): Promise<PullBatchSummary> {
  return await invoke<PullBatchSummary>('nami_pull_all_creators', {
    platform: platform ?? null,
    contentType,
    earlyExitThreshold,
  });
}
