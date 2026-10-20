import { runGalleryDlDownload, type DownloadResult } from '$lib/api/open_nami';
import type { ParsedProfile } from '$lib/utils/open_nami_parser';

export interface MultiTierOptions {
  profile: ParsedProfile;
  cleanOutputDir: string;
  cookiesFile: string | null;
  contentType: string;
  downloadId: string;
  skipReplies?: boolean;
  onProgress?: (statusText: string, count: number) => void;
  nativeRunner?: (profile: ParsedProfile) => Promise<{ count: number; success: boolean }>;
  gdlRunner?: () => Promise<DownloadResult>;
}

export async function dispatchMultiTierDownload(options: MultiTierOptions): Promise<DownloadResult> {
  const { profile, cleanOutputDir, cookiesFile, contentType, downloadId, skipReplies, onProgress } = options;

  if (profile.platform && profile.platform !== 'unknown') {
    try {
      if (options.nativeRunner) {
        const nativeRes = await options.nativeRunner(profile);
        if (nativeRes.success && nativeRes.count > 0) {
          return {
            success: true,
            message: `Successfully downloaded ${nativeRes.count} file(s) via native extractor`,
            files_count: nativeRes.count,
            cancelled: false,
          };
        }
      }
    } catch {
      // Graceful fallback to gallery-dl on any native extractor exception
    }
  }

  // Fallback to gallery-dl orchestrator
  onProgress?.('Switching to gallery-dl engine...', 0);
  if (options.gdlRunner) {
    return await options.gdlRunner();
  }

  return await runGalleryDlDownload(
    profile.cleanUrl,
    cleanOutputDir,
    cookiesFile,
    contentType,
    downloadId,
    skipReplies
  );
}
