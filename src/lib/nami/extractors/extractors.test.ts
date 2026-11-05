import { describe, it, expect } from 'vitest';
import { getExtractor } from './index';
import { parseIgNodeToMediaItems } from './instagram';
import { parseTikTokRehydrationJson } from './tiktok';
import { extractFacebookMediaFromHtml } from './facebook';
import { shouldSkipTweetReply } from './x';

describe('open_nami extractors suite', () => {
  it('instantiates all 4 platform extractors cleanly', () => {
    expect(getExtractor('instagram').platform).toBe('instagram');
    expect(getExtractor('tiktok').platform).toBe('tiktok');
    expect(getExtractor('facebook').platform).toBe('facebook');
    expect(getExtractor('x').platform).toBe('x');
  });

  it('parses instagram media nodes and carousels', () => {
    const mockNode = {
      id: '123456789',
      shortcode: 'C_xyz123',
      is_video: false,
      display_url: 'https://instagram.com/pic.jpg',
      taken_at_timestamp: 1700000000,
    };
    const items = parseIgNodeToMediaItems(mockNode, 'creator');
    expect(items.length).toBe(1);
    expect(items[0].id).toBe('C_xyz123');
    expect(items[0].type).toBe('photo');
    expect(items[0].url).toBe('https://instagram.com/pic.jpg');
    expect(items[0].filename).toContain('creator_1700000000');
  });

  it('parses tiktok rehydration json structures', () => {
    const mockHtml = `<script id="__UNIVERSAL_DATA_FOR_REHYDRATION__">{"__DEFAULT_SCOPE__":{"webapp.user-detail":{"userInfo":{"user":{"uniqueId":"nami_zone"}}}}}</script>`;
    const data = parseTikTokRehydrationJson(mockHtml);
    expect(data['webapp.user-detail'].userInfo.user.uniqueId).toBe('nami_zone');
  });

  it('extracts facebook video items from HTML fragments', () => {
    const mockHtml = `<div><video src="https://video.xx.fbcdn.net/v/t42/test.mp4"></video></div>`;
    const items = extractFacebookMediaFromHtml(mockHtml, 'fb_creator');
    expect(items.length).toBe(1);
    expect(items[0].type).toBe('video');
    expect(items[0].url).toBe('https://video.xx.fbcdn.net/v/t42/test.mp4');
  });

  it('preserves author self-reply threads while skipping conversational replies on x', () => {
    const selfReplyTweet = {
      legacy: {
        id_str: '100',
        user_id_str: '999',
        in_reply_to_user_id_str: '999',
      },
    };
    const conversationalReplyTweet = {
      legacy: {
        id_str: '101',
        user_id_str: '999',
        in_reply_to_user_id_str: '888',
      },
    };
    const originalPostTweet = {
      legacy: {
        id_str: '102',
        user_id_str: '999',
      },
    };

    expect(shouldSkipTweetReply(originalPostTweet, '999')).toBe(false);
    expect(shouldSkipTweetReply(selfReplyTweet, '999')).toBe(false);
    expect(shouldSkipTweetReply(conversationalReplyTweet, '999')).toBe(true);
  });
});
