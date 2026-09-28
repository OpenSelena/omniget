import { describe, it, expect } from 'vitest';
import { parseProfileUrl } from './open_nami_parser';

describe('open_nami_parser', () => {
  it('parses instagram user profile and removes tracking params', () => {
    const raw = 'https://www.instagram.com/opennami/?igsh=MWQ1Z3==&utm_source=qr';
    const parsed = parseProfileUrl(raw);
    expect(parsed.platform).toBe('instagram');
    expect(parsed.username).toBe('opennami');
    expect(parsed.cleanUrl).toBe('https://www.instagram.com/opennami/');
    expect(parsed.isSpecificMedia).toBe(false);
  });

  it('parses instagram reel URL and flags as specific media', () => {
    const raw = 'https://instagram.com/reel/C8XYZ123/?utm_source=ig_web_copy_link';
    const parsed = parseProfileUrl(raw);
    expect(parsed.platform).toBe('instagram');
    expect(parsed.cleanUrl).toBe('https://www.instagram.com/reel/C8XYZ123/');
    expect(parsed.isSpecificMedia).toBe(true);
  });

  it('parses tiktok user handle with or without @', () => {
    const raw = 'https://www.tiktok.com/@creator_zone?is_from_webapp=1';
    const parsed = parseProfileUrl(raw);
    expect(parsed.platform).toBe('tiktok');
    expect(parsed.username).toBe('creator_zone');
    expect(parsed.cleanUrl).toBe('https://www.tiktok.com/@creator_zone');
    expect(parsed.isSpecificMedia).toBe(false);
  });

  it('parses x/twitter user handle', () => {
    const raw = 'https://x.com/OpenSelena?s=20&t=abcdef';
    const parsed = parseProfileUrl(raw);
    expect(parsed.platform).toBe('x');
    expect(parsed.username).toBe('OpenSelena');
    expect(parsed.cleanUrl).toBe('https://x.com/OpenSelena');
  });

  it('parses facebook profile URL', () => {
    const raw = 'https://www.facebook.com/nami.official';
    const parsed = parseProfileUrl(raw);
    expect(parsed.platform).toBe('facebook');
    expect(parsed.username).toBe('nami.official');
    expect(parsed.cleanUrl).toBe('https://www.facebook.com/nami.official');
  });

  it('returns unknown for generic URLs', () => {
    const raw = 'https://example.com/user/page';
    const parsed = parseProfileUrl(raw);
    expect(parsed.platform).toBe('unknown');
    expect(parsed.cleanUrl).toBe('https://example.com/user/page');
  });
});
