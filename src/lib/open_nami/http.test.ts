import { describe, it, expect, vi } from 'vitest';
import { HttpClient, parseNetscapeCookies, formatCookieHeader } from './http';

describe('open_nami http client', () => {
  it('parses netscape cookie format correctly', () => {
    const raw = `# Netscape HTTP Cookie File
.instagram.com\tTRUE\t/\tTRUE\t1799999999\tsessionid\t12345%3Aabc
#HttpOnly_.instagram.com\tTRUE\t/\tTRUE\t1799999999\tds_user_id\t98765
.facebook.com\tTRUE\t/\tTRUE\t1799999999\tc_user\t111222
`;
    const igCookies = parseNetscapeCookies(raw, 'instagram.com');
    expect(igCookies['sessionid']).toBe('12345%3Aabc');
    expect(igCookies['ds_user_id']).toBe('98765');
    expect(igCookies['c_user']).toBeUndefined();
  });

  it('formats cookie headers properly', () => {
    const cookies = { sessionid: 'abc', ds_user_id: '123' };
    const header = formatCookieHeader(cookies);
    expect(header).toBe('sessionid=abc; ds_user_id=123');
  });

  it('dispatches requests through custom invoker', async () => {
    const mockInvoker = vi.fn().mockResolvedValue({
      status: 200,
      ok: true,
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ data: { user: { id: '42' } } }),
    });

    const client = new HttpClient({ invoker: mockInvoker });
    const res = await client.request('https://www.instagram.com/graphql/query');

    expect(mockInvoker).toHaveBeenCalledWith(
      'open_nami_http_fetch',
      {
        options: expect.objectContaining({
          url: 'https://www.instagram.com/graphql/query',
        }),
      }
    );
    expect(res.status).toBe(200);
    expect(res.ok).toBe(true);

    const json = await res.json();
    expect(json.data.user.id).toBe('42');
  });
});
