import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === 'nami_get_history') {
      return [
        {
          id: 'tiktok_123',
          platform: 'tiktok',
          post_id: '123',
          author: 'creator',
          url: 'https://tiktok.com/@creator/video/123',
          file_path: '/path/to/video.mp4',
          file_size: 1024,
          sha256: null,
          downloaded_at: 1731280000
        }
      ];
    }
    if (cmd === 'nami_reconcile_disk') {
      return { checked: 10, removed: 2 };
    }
    if (cmd === 'nami_delete_history_record') {
      return true;
    }
    if (cmd === 'nami_pull_creator') {
      return {
        success: true,
        profile_url: args?.profileUrl,
        platform: args?.platform,
        username: 'creator',
        files_downloaded: 2,
        message: 'Successfully downloaded 2 files',
        early_exited: true,
      };
    }
    if (cmd === 'nami_pull_all_creators') {
      return {
        total: 1,
        succeeded: 1,
        failed: 0,
        total_downloaded: 2,
        results: [
          {
            success: true,
            profile_url: 'https://instagram.com/creator',
            platform: 'instagram',
            username: 'creator',
            files_downloaded: 2,
            message: 'OK',
            early_exited: true,
          },
        ],
      };
    }
    return null;
  })
}));

import { loadNamiHistory, reconcileNamiDisk, deleteNamiHistoryRecord, pullCreator, pullAllCreators } from './nami';

describe('nami api history and ledger', () => {
  it('calls nami_get_history and returns records', async () => {
    const history = await loadNamiHistory('tiktok', 10, 0);
    expect(history.length).toBe(1);
    expect(history[0].id).toBe('tiktok_123');
    expect(history[0].platform).toBe('tiktok');
  });

  it('calls nami_reconcile_disk and returns count', async () => {
    const res = await reconcileNamiDisk();
    expect(res.checked).toBe(10);
    expect(res.removed).toBe(2);
  });

  it('calls nami_delete_history_record and returns status', async () => {
    const ok = await deleteNamiHistoryRecord('tiktok_123');
    expect(ok).toBe(true);
  });

  it('calls nami_pull_creator and returns pull result', async () => {
    const res = await pullCreator('https://instagram.com/creator', 'instagram');
    expect(res.success).toBe(true);
    expect(res.files_downloaded).toBe(2);
    expect(res.early_exited).toBe(true);
  });

  it('calls nami_pull_all_creators and returns batch summary', async () => {
    const summary = await pullAllCreators('instagram');
    expect(summary.total).toBe(1);
    expect(summary.succeeded).toBe(1);
    expect(summary.total_downloaded).toBe(2);
  });
});
