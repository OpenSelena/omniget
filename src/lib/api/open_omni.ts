import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface Profile {
  url: string;
  username?: string;
  platform: string;
  added_at: number;
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
}

export async function checkPythonDependencies(): Promise<string> {
  return await invoke<string>('open_omni_check_python_dependencies');
}

export async function runGalleryDlDownload(
  url: string,
  outputDir: string,
  cookiesFile: string | null,
  contentType: string,
  downloadId: string
): Promise<DownloadResult> {
  return await invoke<DownloadResult>('open_omni_run_gallery_dl_download', {
    url,
    outputDir,
    cookiesFile,
    contentType,
    downloadId
  });
}

export async function cancelDownload(downloadId: string): Promise<string> {
  return await invoke<string>('open_omni_cancel_download', { downloadId });
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
  cookiesFile: string | null
): Promise<string> {
  return await invoke<string>('open_omni_save_app_settings', {
    outputDirectory,
    cookiesFile
  });
}

export async function loadAppSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>('open_omni_load_app_settings');
}

export async function loadProfiles(platform: string): Promise<Profile[]> {
  return await invoke<Profile[]>('open_omni_load_profiles', { platform });
}

export async function saveProfile(platform: string, url: string): Promise<string> {
  return await invoke<string>('open_omni_save_profile', { platform, url });
}

export async function deleteProfile(platform: string, profileUrl: string): Promise<string> {
  return await invoke<string>('open_omni_delete_profile', { platform, profileUrl });
}

export function generateDownloadId(): string {
  return `dl_${Date.now()}_${Math.random().toString(36).slice(2, 11)}`;
}


