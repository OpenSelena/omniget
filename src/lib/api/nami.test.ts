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
    return null;
  })
}));

import { loadNamiHistory, reconcileNamiDisk, deleteNamiHistoryRecord } from './nami';

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
});
