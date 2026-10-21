import {HttpClient, sleep} from '../http'
import type {Extractor, ExtractOptions, MediaItem} from '../types'
import type {ParsedProfile} from '../../utils/open_nami_parser'

export function decodeEscapedUrl(raw: string): string {
  const sanitized = raw.replace(/[\r\n\t]/g, '').trim()
  return sanitized
    .replace(/\\u([0-9a-fA-F]{4})/g, (_, hex) => String.fromCharCode(parseInt(hex, 16)))
    .replace(/\\(["\\\/bfnrt])/g, (_, ch) => {
      const escapes: Record<string, string> = {
        '"': '"',
        '\\': '\\',
        '/': '/',
        b: '\b',
        f: '\f',
        n: '\n',
        r: '\r',
        t: '\t',
      }
      return escapes[ch] || ch
    })
}

export function extractFacebookMediaFromHtml(html: string, username: string): MediaItem[] {
  const items: MediaItem[] = []
  const seenUrls = new Set<string>()

  // 1. HD / SD and playable video URLs
  const videoPatterns = [
    /"browser_native_hd_url":"([^"]+)"/g,
    /"browser_native_sd_url":"([^"]+)"/g,
    /"playable_url_quality_hd":"([^"]+)"/g,
    /"playable_url":"([^"]+)"/g,
    /<video[^>]+src=["'](https?:[^"']+)["']/gi,
  ]

  let videoIndex = 1
  for (const regex of videoPatterns) {
    let vMatch: RegExpExecArray | null
    while ((vMatch = regex.exec(html)) !== null) {
      const rawUrl = vMatch[1]
      const videoUrl = decodeEscapedUrl(rawUrl).replace(/&amp;/g, '&')
      if (videoUrl.startsWith('http') && !seenUrls.has(videoUrl)) {
        seenUrls.add(videoUrl)
        items.push({
          id: `fb_video_${videoIndex}`,
          url: videoUrl,
          filename: `${username}_video_${videoIndex}`,
          extension: 'mp4',
          type: 'video',
        })
        videoIndex++
      }
    }
  }

  // 2. High-res photo URI nodes in JSON
  const photoRegex = /,"(?:image|viewer_image|full_size_image|display_image)":\{"uri":"([^"]+)"/g
  let match: RegExpExecArray | null
  let photoIndex = 1

  while ((match = photoRegex.exec(html)) !== null) {
    const rawUrl = match[1]
    const photoUrl = decodeEscapedUrl(rawUrl).replace(/&amp;/g, '&')
    if (!seenUrls.has(photoUrl)) {
      seenUrls.add(photoUrl)
      items.push({
        id: `fb_photo_${photoIndex}`,
        url: photoUrl,
        filename: `${username}_photo_${photoIndex}`,
        extension: 'jpg',
        type: 'photo',
      })
      photoIndex++
    }
  }

  // 3. Mobile / mbasic server-rendered photo image tags
  const mbasicImgRegex = /<img[^>]+src=["'](https:\/\/[^"'\s]*scontent[^"'\s]*\.(?:jpg|png|webp)[^"'\s]*)["']/gi
  let mMatch: RegExpExecArray | null
  while ((mMatch = mbasicImgRegex.exec(html)) !== null) {
    const rawUrl = mMatch[1]
    const photoUrl = decodeEscapedUrl(rawUrl).replace(/&amp;/g, '&')
    if (!seenUrls.has(photoUrl)) {
      seenUrls.add(photoUrl)
      items.push({
        id: `fb_photo_${photoIndex}`,
        url: photoUrl,
        filename: `${username}_photo_${photoIndex}`,
        extension: 'jpg',
        type: 'photo',
      })
      photoIndex++
    }
  }

  // 4. Fallback: scontent photo links in raw HTML
  if (items.length === 0) {
    const scontentRegex = /https:\/\/[^"'\s<>]*scontent[^"'\s<>]*\.(?:jpg|png|webp)[^"'\s<>]*/gi
    let sMatch: RegExpExecArray | null
    while ((sMatch = scontentRegex.exec(html)) !== null) {
      const cleanUrl = sMatch[0].replace(/\\/g, '').replace(/&amp;/g, '&')
      if (!seenUrls.has(cleanUrl)) {
        seenUrls.add(cleanUrl)
        items.push({
          id: `fb_photo_${photoIndex}`,
          url: cleanUrl,
          filename: `${username}_photo_${photoIndex}`,
          extension: 'jpg',
          type: 'photo',
        })
        photoIndex++
      }
    }
  }

  return items
}

export function extractFacebookPaginationCursor(html: string): string | null {
  // 1. JSON page_info end_cursor or cursor
  const jsonCursorMatch = html.match(/"end_cursor":"([^"]+)"/) || html.match(/"cursor":"([^"]+)"/)
  if (jsonCursorMatch && jsonCursorMatch[1]) {
    return jsonCursorMatch[1]
  }

  // 2. Mobile / mbasic pagination links with cursor or after parameter
  const linkCursorMatch = html.match(/[?&](?:cursor|after)=([^"'\s&<>]+)/i)
  if (linkCursorMatch && linkCursorMatch[1]) {
    try {
      return decodeURIComponent(linkCursorMatch[1])
    } catch {
      return linkCursorMatch[1]
    }
  }

  return null
}

export function resolveFacebookUrl(href: string, currentUrl: string): string {
  const cleanHref = href.replace(/\\/g, '').replace(/&amp;/g, '&')
  try {
    return new URL(cleanHref, currentUrl).toString()
  } catch {
    return cleanHref
  }
}

export function extractFacebookNextPageUrl(html: string, currentUrl: string): string | null {
  // 1. Check for dedicated more item containers in mbasic / mobile HTML
  const moreItemMatch = html.match(/<div[^>]*id=["'](?:m_more_item|m_more_posts)["'][^>]*>\s*<a[^>]+href=["']([^"']+)["']/i)
  if (moreItemMatch && moreItemMatch[1]) {
    return resolveFacebookUrl(moreItemMatch[1], currentUrl)
  }

  // 2. Check for anchor tag with cursor/after and pagination label
  const cursorLabelMatch = html.match(/<a[^>]+href=["']([^"']*(?:cursor=|after=)[^"']+)["'][^>]*>[\s\S]*?(?:See [Mm]ore|More [Pp]hotos|More|Next|Older [Pp]osts)[\s\S]*?<\/a>/i)
  if (cursorLabelMatch && cursorLabelMatch[1]) {
    return resolveFacebookUrl(cursorLabelMatch[1], currentUrl)
  }

  // 3. Check for any link containing cursor= or after=
  const genericCursorMatch = html.match(/<a[^>]+href=["']([^"']*(?:[?&]|&amp;)(?:cursor|after)=[^"']+)["']/i)
  if (genericCursorMatch && genericCursorMatch[1]) {
    return resolveFacebookUrl(genericCursorMatch[1], currentUrl)
  }

  // 4. Fallback: if cursor is present in JSON or HTML, append it to currentUrl
  const cursor = extractFacebookPaginationCursor(html)
  if (cursor) {
    try {
      const u = new URL(currentUrl)
      if (u.searchParams.get('cursor') !== cursor) {
        u.searchParams.set('cursor', cursor)
        return u.toString()
      }
    } catch {
      if (!currentUrl.includes(cursor)) {
        const sep = currentUrl.includes('?') ? '&' : '?'
        return `${currentUrl}${sep}cursor=${encodeURIComponent(cursor)}`
      }
    }
  }

  return null
}

export class FacebookExtractor implements Extractor {
  platform = 'facebook' as const
  private httpClient: HttpClient

  constructor(httpClient?: HttpClient) {
    this.httpClient = httpClient ?? new HttpClient({
      'Referer': 'https://www.facebook.com/',
    })
  }

  async *extract(profile: ParsedProfile, options: ExtractOptions): AsyncGenerator<MediaItem, void, unknown> {
    const username = profile.username
    const subDir = options.subDir

    options.onProgress?.({
      stage: 'Profile',
      found: 0,
      message: `Fetching Facebook profile @${username}...`,
    })

    const targetPath = subDir === 'Photos' ? 'photos' : 'videos'
    const desktopUrl = `https://www.facebook.com/${encodeURIComponent(username)}/${targetPath}`
    const mbasicUrl = `https://mbasic.facebook.com/${encodeURIComponent(username)}/${targetPath}`
    const mobileUrl = `https://m.facebook.com/${encodeURIComponent(username)}/${targetPath}`

    let currentUrl = desktopUrl
    let html = ''

    try {
      html = await this.httpClient.fetchText(desktopUrl, {
        cookiePath: options.cookiePath,
        cookieDomain: 'facebook.com',
        signal: options.signal,
        retries: 0,
        timeoutMs: 4000,
      })
    } catch {
      html = ''
    }

    let initialItems = extractFacebookMediaFromHtml(html, username)

    // Fallback to mobile endpoints if desktop returns blank shell
    if (initialItems.length === 0) {
      options.onProgress?.({
        stage: 'Profile',
        found: 0,
        message: `Desktop yielded 0 items. Falling back to mbasic.facebook.com for @${username}...`,
      })
      try {
        const mbasicHtml = await this.httpClient.fetchText(mbasicUrl, {
          cookiePath: options.cookiePath,
          cookieDomain: 'facebook.com',
          signal: options.signal,
          retries: 1,
          timeoutMs: 5000,
          headers: {
            'Referer': 'https://mbasic.facebook.com/',
          },
        })
        const mbasicItems = extractFacebookMediaFromHtml(mbasicHtml, username)
        if (mbasicItems.length > 0 || mbasicHtml.length > 200) {
          html = mbasicHtml
          currentUrl = mbasicUrl
          initialItems = mbasicItems
        }
      } catch {
        // Fall through to m.facebook.com
      }

      if (initialItems.length === 0) {
        try {
          const mobileHtml = await this.httpClient.fetchText(mobileUrl, {
            cookiePath: options.cookiePath,
            cookieDomain: 'facebook.com',
            signal: options.signal,
            retries: 0,
            timeoutMs: 5000,
            headers: {
              'User-Agent':
                'Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36',
              'Referer': 'https://m.facebook.com/',
            },
          })
          const mobileItems = extractFacebookMediaFromHtml(mobileHtml, username)
          if (mobileItems.length > 0) {
            html = mobileHtml
            currentUrl = mobileUrl
            initialItems = mobileItems
          }
        } catch {
          // Fallbacks exhausted
        }
      }
    }

    const seenUrls = new Set<string>()
    let found = 0
    let page = 1
    const maxPages = 5

    while (true) {
      const items = extractFacebookMediaFromHtml(html, username)
      let pageNewCount = 0

      for (const item of items) {
        if (subDir === 'Photos' && item.type !== 'photo') continue
        if (subDir === 'Videos' && item.type !== 'video') continue
        if (seenUrls.has(item.url)) continue

        seenUrls.add(item.url)
        found++
        pageNewCount++
        options.onProgress?.({
          stage: subDir,
          found,
          message: `Found item: ${item.filename}`,
        })
        yield item
      }

      if (page >= maxPages || options.signal?.aborted) {
        break
      }

      const nextUrl = extractFacebookNextPageUrl(html, currentUrl)
      if (!nextUrl || nextUrl === currentUrl) {
        break
      }

      if (pageNewCount === 0 && page > 1) {
        break
      }

      await sleep(1000, 2000, options.signal)

      try {
        currentUrl = nextUrl
        html = await this.httpClient.fetchText(currentUrl, {
          cookiePath: options.cookiePath,
          cookieDomain: 'facebook.com',
          signal: options.signal,
          retries: 1,
          timeoutMs: 5000,
          headers: {
            'Referer': currentUrl.includes('mbasic') ? 'https://mbasic.facebook.com/' : 'https://www.facebook.com/',
          },
        })
        page++
      } catch {
        break
      }
    }
  }
}

