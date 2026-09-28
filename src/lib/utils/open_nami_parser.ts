export type Platform = 'instagram' | 'tiktok' | 'facebook' | 'x' | 'unknown';
export type SupportedPlatform = 'instagram' | 'tiktok' | 'facebook' | 'x';

export interface ParsedProfile {
  platform: Platform;
  username: string;
  cleanUrl: string;
  rawInput: string;
  isSpecificMedia?: boolean;
}

const INVALID_CHARS_REGEX = /[<>:"/\\|?*\x00-\x1f]/g;
const WINDOWS_RESERVED_NAMES = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i;

export function sanitizeUsername(username: string): string {
  let cleaned = username.replace(INVALID_CHARS_REGEX, '_').trim().replace(/^[.\s]+|[.\s]+$/g, '');
  if (WINDOWS_RESERVED_NAMES.test(cleaned)) {
    cleaned = `_${cleaned}`;
  }
  return cleaned.slice(0, 120) || 'unknown';
}

export function buildProfileUrl(platform: SupportedPlatform, username: string): string {
  const cleanUser = username.trim().replace(/^@/, '').replace(/^[.\s]+|[.\s]+$/g, '');
  switch (platform) {
    case 'instagram':
      return `https://www.instagram.com/${cleanUser}/`;
    case 'tiktok':
      return `https://www.tiktok.com/@${cleanUser}`;
    case 'facebook':
      return `https://www.facebook.com/${cleanUser}`;
    case 'x':
      return `https://x.com/${cleanUser}`;
  }
}

export function parseProfileUrl(raw: string): ParsedProfile {
  const trimmed = raw.trim();
  if (!trimmed) {
    return { platform: 'unknown', username: '', cleanUrl: '', rawInput: raw, isSpecificMedia: false };
  }

  // Handle prefix shortcuts (ig:user, tt:user, etc.)
  const prefixMatch = trimmed.match(/^(ig|instagram|tt|tiktok|fb|facebook|x|twitter):([^\s/]+)$/i);
  if (prefixMatch) {
    const rawPrefix = prefixMatch[1].toLowerCase();
    const rawUser = prefixMatch[2].replace(/^@/, '');
    const platform: SupportedPlatform =
      rawPrefix === 'ig' || rawPrefix === 'instagram'
        ? 'instagram'
        : rawPrefix === 'tt' || rawPrefix === 'tiktok'
          ? 'tiktok'
          : rawPrefix === 'fb' || rawPrefix === 'facebook'
            ? 'facebook'
            : 'x';
    return {
      platform,
      username: sanitizeUsername(rawUser),
      cleanUrl: buildProfileUrl(platform, rawUser),
      rawInput: raw,
      isSpecificMedia: false,
    };
  }

  let candidate = trimmed;
  if (!/^https?:\/\//i.test(candidate)) {
    if (
      candidate.includes('instagram.com') ||
      candidate.includes('tiktok.com') ||
      candidate.includes('facebook.com') ||
      candidate.includes('x.com') ||
      candidate.includes('twitter.com')
    ) {
      candidate = 'https://' + candidate;
    }
  }

  try {
    const url = new URL(candidate);
    const host = url.hostname.toLowerCase().replace(/^www\./, '').replace(/^m\./, '');
    const pathname = url.pathname.replace(/\/+$/, '');
    const pathParts = pathname.split('/').filter(Boolean);

    // 1. Instagram
    if (host.includes('instagram.com')) {
      const firstPart = pathParts[0]?.toLowerCase() || '';
      const isMedia = firstPart === 'p' || firstPart === 'reel' || firstPart === 'reels' || firstPart === 'tv';
      if (isMedia) {
        return {
          platform: 'instagram',
          username: '',
          cleanUrl: `https://www.instagram.com/${pathParts[0]}/${pathParts[1] || ''}/`,
          rawInput: raw,
          isSpecificMedia: true,
        };
      }

      const reserved = ['explore', 'direct', 'accounts', 'stories'];
      if (firstPart === 'stories' && pathParts[1]) {
        return {
          platform: 'instagram',
          username: sanitizeUsername(pathParts[1]),
          cleanUrl: `https://www.instagram.com/stories/${pathParts[1]}/`,
          rawInput: raw,
          isSpecificMedia: true,
        };
      }

      if (firstPart && !reserved.includes(firstPart)) {
        const username = pathParts[0].replace(/^@/, '');
        return {
          platform: 'instagram',
          username: sanitizeUsername(username),
          cleanUrl: `https://www.instagram.com/${username}/`,
          rawInput: raw,
          isSpecificMedia: false,
        };
      }
    }

    // 2. TikTok
    if (host.includes('tiktok.com')) {
      const userPart = pathParts[0] || '';
      const username = userPart.startsWith('@') ? userPart.slice(1) : userPart;
      const isVideo = pathParts[1] === 'video';
      if (username) {
        return {
          platform: 'tiktok',
          username: sanitizeUsername(username),
          cleanUrl: isVideo
            ? `https://www.tiktok.com/@${username}/video/${pathParts[2] || ''}`
            : `https://www.tiktok.com/@${username}`,
          rawInput: raw,
          isSpecificMedia: isVideo,
        };
      }
    }

    // 3. Facebook
    if (host.includes('facebook.com') || host === 'fb.com') {
      if (pathParts[0]?.toLowerCase() === 'profile.php') {
        const id = url.searchParams.get('id');
        if (id) {
          return {
            platform: 'facebook',
            username: sanitizeUsername(id),
            cleanUrl: `https://www.facebook.com/profile.php?id=${id}`,
            rawInput: raw,
            isSpecificMedia: false,
          };
        }
      } else if (pathParts[0]?.toLowerCase() === 'people' && pathParts[1]) {
        return {
          platform: 'facebook',
          username: sanitizeUsername(pathParts[1]),
          cleanUrl: `https://www.facebook.com/people/${pathParts[1]}`,
          rawInput: raw,
          isSpecificMedia: false,
        };
      } else if (pathParts[0]) {
        const reserved = ['watch', 'videos', 'reel', 'groups', 'pages', 'events', 'photo'];
        const isMedia = reserved.includes(pathParts[0].toLowerCase());
        const username = isMedia ? '' : sanitizeUsername(pathParts[0]);
        return {
          platform: 'facebook',
          username,
          cleanUrl: `https://www.facebook.com/${pathParts.join('/')}`,
          rawInput: raw,
          isSpecificMedia: isMedia,
        };
      }
    }

    // 4. X / Twitter
    if (host.includes('x.com') || host.includes('twitter.com')) {
      const firstPart = pathParts[0]?.toLowerCase() || '';
      const reserved = ['home', 'explore', 'notifications', 'messages', 'i', 'settings'];
      if (firstPart && !reserved.includes(firstPart)) {
        const username = pathParts[0].replace(/^@/, '');
        const isStatus = pathParts[1] === 'status';
        return {
          platform: 'x',
          username: sanitizeUsername(username),
          cleanUrl: isStatus
            ? `https://x.com/${username}/status/${pathParts[2] || ''}`
            : `https://x.com/${username}`,
          rawInput: raw,
          isSpecificMedia: isStatus,
        };
      }
    }

    // Generic URL fallback
    return {
      platform: 'unknown',
      username: '',
      cleanUrl: `${url.origin}${url.pathname}`,
      rawInput: raw,
      isSpecificMedia: false,
    };
  } catch {
    return {
      platform: 'unknown',
      username: '',
      cleanUrl: trimmed,
      rawInput: raw,
      isSpecificMedia: false,
    };
  }
}
