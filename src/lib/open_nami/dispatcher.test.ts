import { describe, it, expect, vi } from 'vitest';
import { dispatchMultiTierDownload } from './dispatcher';
import type { ParsedProfile } from '../utils/open_nami_parser';

describe('open_nami multi-tier dispatcher', () => {
  const mockProfile: ParsedProfile = {
    platform: 'instagram',
    username: 'test_creator',
    cleanUrl: 'https://www.instagram.com/test_creator/',
    rawInput: 'test_creator',
    isSpecificMedia: false,
  };

  it('runs native extractor when available and returns result on success', async () => {
    const mockNativeRunner = vi.fn().mockResolvedValue({ success: true, count: 5 });
    const mockGdlRunner = vi.fn();

    const result = await dispatchMultiTierDownload({
      profile: mockProfile,
      cleanOutputDir: 'C:/downloads',
      cookiesFile: null,
      contentType: 'photos',
      downloadId: 'dl_123',
      nativeRunner: mockNativeRunner,
      gdlRunner: mockGdlRunner,
    });

    expect(mockNativeRunner).toHaveBeenCalledTimes(1);
    expect(mockGdlRunner).not.toHaveBeenCalled();
    expect(result.success).toBe(true);
    expect(result.files_count).toBe(5);
  });

  it('falls back to gallery-dl runner if native extractor yields 0 items', async () => {
    const mockNativeRunner = vi.fn().mockResolvedValue({ success: false, count: 0 });
    const mockGdlRunner = vi.fn().mockResolvedValue({
      success: true,
      message: 'Downloaded via gallery-dl',
      files_count: 3,
      cancelled: false,
    });
    const progressSpy = vi.fn();

    const result = await dispatchMultiTierDownload({
      profile: mockProfile,
      cleanOutputDir: 'C:/downloads',
      cookiesFile: null,
      contentType: 'photos',
      downloadId: 'dl_123',
      onProgress: progressSpy,
      nativeRunner: mockNativeRunner,
      gdlRunner: mockGdlRunner,
    });

    expect(mockNativeRunner).toHaveBeenCalledTimes(1);
    expect(progressSpy).toHaveBeenCalledWith('Switching to gallery-dl engine...', 0);
    expect(mockGdlRunner).toHaveBeenCalledTimes(1);
    expect(result.success).toBe(true);
    expect(result.files_count).toBe(3);
  });

  it('bypasses native extractor and uses gallery-dl directly for unknown platforms', async () => {
    const unknownProfile: ParsedProfile = {
      platform: 'unknown',
      username: '',
      cleanUrl: 'https://example.com/gallery',
      rawInput: 'https://example.com/gallery',
      isSpecificMedia: false,
    };
    const mockNativeRunner = vi.fn();
    const mockGdlRunner = vi.fn().mockResolvedValue({
      success: true,
      message: 'Downloaded via gallery-dl',
      files_count: 10,
      cancelled: false,
    });

    const result = await dispatchMultiTierDownload({
      profile: unknownProfile,
      cleanOutputDir: 'C:/downloads',
      cookiesFile: null,
      contentType: 'all',
      downloadId: 'dl_456',
      nativeRunner: mockNativeRunner,
      gdlRunner: mockGdlRunner,
    });

    expect(mockNativeRunner).not.toHaveBeenCalled();
    expect(mockGdlRunner).toHaveBeenCalledTimes(1);
    expect(result.files_count).toBe(10);
  });
});
