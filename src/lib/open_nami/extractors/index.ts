import { InstagramExtractor } from './instagram';
import { XExtractor } from './x';
import { TikTokExtractor } from './tiktok';
import { FacebookExtractor } from './facebook';
import type { Extractor } from '../types';
import type { SupportedPlatform } from '../../utils/open_nami_parser';

export * from '../types';
export * from '../http';
export * from './instagram';
export * from './x';
export * from './tiktok';
export * from './facebook';

export function getExtractor(platform: SupportedPlatform): Extractor {
  switch (platform) {
    case 'instagram':
      return new InstagramExtractor();
    case 'x':
      return new XExtractor();
    case 'tiktok':
      return new TikTokExtractor();
    case 'facebook':
      return new FacebookExtractor();
    default:
      throw new Error(`Unsupported extractor platform: ${platform}`);
  }
}
