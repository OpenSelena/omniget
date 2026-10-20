import type { ParsedProfile, SupportedPlatform } from '../utils/open_nami_parser';

export type MediaType = 'photo' | 'video';

export interface MediaItem {
  id: string;
  url: string;
  filename: string;
  extension: string;
  type: MediaType;
  caption?: string;
  date?: number;
  width?: number;
  height?: number;
  thumbnailUrl?: string;
}

export type SubDirChoice = 'Photos' | 'Videos' | 'Stories' | 'Highlights';

export interface ExtractProgress {
  stage: string;
  found: number;
  message?: string;
}

export interface ExtractOptions {
  subDir: SubDirChoice;
  cookiePath?: string | null;
  signal?: AbortSignal;
  onProgress?: (progress: ExtractProgress) => void;
}

export interface Extractor {
  platform: SupportedPlatform;
  extract(profile: ParsedProfile, options: ExtractOptions): AsyncGenerator<MediaItem, void, unknown>;
}
