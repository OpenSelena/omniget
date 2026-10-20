import {HttpClient, sleep} from '../http'
import type {Extractor, ExtractOptions, MediaItem} from '../types'
import type {ParsedProfile} from '../../utils/open_nami_parser'

export function parseTikTokRehydrationJson(html: string): any {
  // Pattern 1: Modern __UNIVERSAL_DATA_FOR_REHYDRATION__
  const universalMatch = html.match(
    /<script id="__UNIVERSAL_DATA_FOR_REHYDRATION__"[^>]*>([\s\S]*?)<\/script>/,
  )
  if (universalMatch) {
    try {
      const raw = JSON.parse(universalMatch[1])
      const scope = raw?.['__DEFAULT_SCOPE__'] || raw
      if (scope) return scope
    } catch {}
  }

  // Pattern 2: SIGI_STATE script tag
  const sigiTagMatch = html.match(
    /<script id="SIGI_STATE"[^>]*>([\s\S]*?)<\/script>/,
  )
  if (sigiTagMatch) {
    try {
      const raw = JSON.parse(sigiTagMatch[1])
      const user = Object.values(raw?.UserModule?.users || {})[0] as any
      const items = Object.values(raw?.ItemModule || {}) as any[]
      return {
        'webapp.user-detail': {
          userInfo: { user },
          itemList: items,
        },
        SIGI_STATE: raw,
      }
    } catch {}
  }

  // Pattern 3: window['SIGI_STATE'] assignment
  const sigiWindowMatch = html.match(
    /(?:window\[['"]SIGI_STATE['"]\]|SIGI_STATE)\s*=\s*(\{[\s\S]*?\})\s*;/i,
  )
  if (sigiWindowMatch) {
    try {
      const raw = JSON.parse(sigiWindowMatch[1])
      const user = Object.values(raw?.UserModule?.users || {})[0] as any
      const items = Object.values(raw?.ItemModule || {}) as any[]
      return {
        'webapp.user-detail': {
          userInfo: { user },
          itemList: items,
        },
        SIGI_STATE: raw,
      }
    } catch {}
  }

  // Pattern 4: __NEXT_DATA__ script tag
  const nextDataMatch = html.match(
    /<script id="__NEXT_DATA__"[^>]*>([\s\S]*?)<\/script>/,
  )
  if (nextDataMatch) {
    try {
      const raw = JSON.parse(nextDataMatch[1])
      const pageProps = raw?.props?.pageProps
      if (pageProps) {
        return {
          'webapp.user-detail': {
            userInfo: pageProps.userInfo,
            itemList: pageProps.items || pageProps.itemList || [],
          },
          __NEXT_DATA__: raw,
        }
      }
    } catch {}
  }

  // Pattern 5: Direct SecUid fallback regex
  const secUidMatch = html.match(/(?:secUid|sec_uid)["':=\s]+(MS4wLjABAAAA[\w-]{32,64})/i)
  if (secUidMatch) {
    return {
      'webapp.user-detail': {
        userInfo: {
          user: {
            secUid: secUidMatch[1],
          },
        },
        itemList: [],
      },
    }
  }

  return null
}


export function parseTikTokItem(item: any, username: string): MediaItem[] {
  const items: MediaItem[] = []
  const id = item.id || item.itemId || String(Date.now())
  const date = Number(item.createTime) || undefined
  const caption = item.desc

  const imageList = item.imagePost?.images
  if (Array.isArray(imageList) && imageList.length > 0) {
    imageList.forEach((img: any, idx: number) => {
      const imgUrl = img.imageURL?.urlList?.[0] || img.displayImage?.urlList?.[0]
      if (imgUrl) {
        items.push({
          id: `${id}_${idx + 1}`,
          url: imgUrl,
          filename: `${username}_${id}_${idx + 1}`,
          extension: 'jpg',
          type: 'photo',
          caption,
          date,
        })
      }
    })
    return items
  }

  const video = item.video
  const videoUrl = video?.playAddr || video?.downloadAddr
  if (videoUrl) {
    items.push({
      id,
      url: videoUrl,
      filename: `${username}_${id}`,
      extension: 'mp4',
      type: 'video',
      caption,
      date,
      width: video?.width,
      height: video?.height,
      thumbnailUrl: video?.cover || video?.originCover,
    })
  }

  return items
}

export class TikTokExtractor implements Extractor {
  platform = 'tiktok' as const
  private httpClient: HttpClient

  constructor(httpClient?: HttpClient) {
    this.httpClient = httpClient ?? new HttpClient({
      'Referer': 'https://www.tiktok.com/',
    })
  }

  async *extract(profile: ParsedProfile, options: ExtractOptions): AsyncGenerator<MediaItem, void, unknown> {
    const cleanUsername = profile.username.replace(/^@/, '')
    const subDir = options.subDir

    options.onProgress?.({
      stage: 'Profile',
      found: 0,
      message: `Fetching TikTok profile @${cleanUsername}...`,
    })

    const profileUrl = `https://www.tiktok.com/@${cleanUsername}`
    const html = await this.httpClient.fetchText(profileUrl, {
      cookiePath: options.cookiePath,
      cookieDomain: 'tiktok.com',
      signal: options.signal,
      retries: 0,
      timeoutMs: 4000,
    })

    const scope = parseTikTokRehydrationJson(html)
    const userDetail = scope?.['webapp.user-detail']
    const userInfo = userDetail?.userInfo?.user
    const secUid = userInfo?.secUid

    let found = 0

    // Initial SSR hydration items
    const initialItems = userDetail?.itemList || []
    for (const rawItem of initialItems) {
      const mediaList = parseTikTokItem(rawItem, cleanUsername)
      for (const item of mediaList) {
        if (subDir === 'Photos' && item.type !== 'photo') continue
        if (subDir === 'Videos' && item.type !== 'video') continue

        found++
        options.onProgress?.({
          stage: subDir,
          found,
          message: `Found item: ${item.filename}`,
        })
        yield item
      }
    }

    if (!secUid) {
      return
    }

    // Paginated posts via item_list API
    let cursor = '0'
    let hasMore = true
    const visitedCursors = new Set<string>()

    while (hasMore) {
      if (options.signal?.aborted) break
      if (visitedCursors.has(cursor)) break
      visitedCursors.add(cursor)

      const deviceId = `${Math.floor(7250000000000000000 + Math.random() * 75000000000000000)}`
      const queryParams = new URLSearchParams({
        aid: '1988',
        app_name: 'tiktok_web',
        device_platform: 'web_pc',
        channel: 'tiktok_web',
        device_id: deviceId,
        secUid,
        count: '30',
        cursor,
        post_item_list_request_type: '0',
      })
      const apiUrl = `https://www.tiktok.com/api/post/item_list/?${queryParams.toString()}`

      let res: any
      try {
        res = await this.httpClient.fetchJson<any>(apiUrl, {
          cookiePath: options.cookiePath,
          cookieDomain: 'tiktok.com',
          signal: options.signal,
          headers: {
            'Referer': profileUrl,
          },
        })
      } catch {
        break
      }

      const items = res?.itemList || []
      if (items.length === 0) break

      for (const rawItem of items) {
        const mediaList = parseTikTokItem(rawItem, cleanUsername)
        for (const item of mediaList) {
          if (subDir === 'Photos' && item.type !== 'photo') continue
          if (subDir === 'Videos' && item.type !== 'video') continue

          found++
          options.onProgress?.({
            stage: subDir,
            found,
            message: `Found item: ${item.filename}`,
          })
          yield item
        }
      }

      hasMore = Boolean(res?.hasMore)
      cursor = String(res?.cursor ?? '0')

      await sleep(1000, 2000, options.signal)
    }
  }
}
