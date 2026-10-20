import { invoke } from '@tauri-apps/api/core';

export const DEFAULT_USER_AGENT =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36';

export const DEFAULT_CHROME_HEADERS: Record<string, string> = {
  'User-Agent': DEFAULT_USER_AGENT,
  'sec-ch-ua': '"Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24"',
  'sec-ch-ua-mobile': '?0',
  'sec-ch-ua-platform': '"Windows"',
  Accept: '*/*',
  'Accept-Language': 'en-US,en;q=0.9',
};

export function parseNetscapeCookies(content: string, filterDomain?: string): Record<string, string> {
  const cookies: Record<string, string> = {};
  const lines = content.split(/\r?\n/);

  for (const rawLine of lines) {
    let line = rawLine.trim();
    if (!line) continue;

    if (line.startsWith('#HttpOnly_')) {
      line = line.slice('#HttpOnly_'.length);
    } else if (line.startsWith('#')) {
      continue;
    }

    const parts = line.split('\t');
    if (parts.length >= 7) {
      const domain = parts[0].toLowerCase();
      const name = parts[5];
      const value = parts[6];

      if (filterDomain) {
        const cleanDomain = filterDomain.toLowerCase().replace(/^\./, '');
        const lineDomain = domain.replace(/^\./, '');
        if (!lineDomain.endsWith(cleanDomain) && !cleanDomain.endsWith(lineDomain)) {
          continue;
        }
      }

      if (name) {
        cookies[name] = value;
      }
    }
  }

  return cookies;
}

export async function loadCookiesFromPath(
  filePath: string,
  filterDomain?: string
): Promise<Record<string, string>> {
  try {
    const fs = await import('node:fs/promises');
    const content = await fs.readFile(filePath, 'utf-8');
    return parseNetscapeCookies(content, filterDomain);
  } catch {
    return {};
  }
}

export function formatCookieHeader(cookies: Record<string, string>): string {
  return Object.entries(cookies)
    .map(([k, v]) => `${k}=${v}`)
    .join('; ');
}

export async function sleep(minMs: number, maxMs?: number, signal?: AbortSignal): Promise<void> {
  const ms = maxMs !== undefined ? Math.floor(minMs + Math.random() * (maxMs - minMs)) : minMs;
  if (ms <= 0) return;
  if (signal?.aborted) {
    throw signal.reason ?? new Error('Aborted');
  }
  return new Promise((resolve, reject) => {
    const timer = setTimeout(resolve, ms);
    if (signal) {
      signal.addEventListener(
        'abort',
        () => {
          clearTimeout(timer);
          reject(signal.reason ?? new Error('Aborted'));
        },
        { once: true }
      );
    }
  });
}

export interface RequestOptions {
  method?: string;
  headers?: Record<string, string>;
  body?: string;
  cookies?: Record<string, string> | string;
  cookiePath?: string | null;
  cookieDomain?: string;
  retries?: number;
  retryDelayMs?: number;
  timeoutMs?: number;
  signal?: AbortSignal;
}

export interface HttpResponsePayload {
  status: number;
  ok: boolean;
  headers: Record<string, string>;
  body: string;
}

export type InvokerFn = (cmd: string, args: Record<string, unknown>) => Promise<HttpResponsePayload>;

export class HttpResponse {
  readonly status: number;
  readonly statusText: string;
  readonly ok: boolean;
  readonly headers: Headers;
  readonly body: { cancel: () => Promise<void> } | null;
  private readonly rawBody: string;

  constructor(payload: HttpResponsePayload) {
    this.status = payload.status;
    this.statusText = payload.ok ? 'OK' : 'Error';
    this.ok = payload.ok;
    this.headers = new Headers(payload.headers);
    this.rawBody = payload.body;
    this.body = { cancel: async () => {} };
  }

  async text(): Promise<string> {
    return this.rawBody;
  }

  async json<T = any>(): Promise<T> {
    return JSON.parse(this.rawBody);
  }
}

export class HttpClient {
  private defaultCookies: Record<string, string> = {};
  private defaultHeaders: Record<string, string>;
  private cookieCache: Map<string, Record<string, string>> = new Map();
  private invoker: InvokerFn;

  constructor(
    headersOrOptions?: Record<string, string> | { headers?: Record<string, string>; cookies?: Record<string, string>; invoker?: InvokerFn },
    cookies?: Record<string, string>,
    invoker?: InvokerFn
  ) {
    if (headersOrOptions && ('headers' in headersOrOptions || 'cookies' in headersOrOptions || 'invoker' in headersOrOptions)) {
      const opts = headersOrOptions as { headers?: Record<string, string>; cookies?: Record<string, string>; invoker?: InvokerFn };
      this.defaultHeaders = { ...DEFAULT_CHROME_HEADERS, ...(opts.headers || {}) };
      if (opts.cookies) {
        this.defaultCookies = { ...opts.cookies };
      }
      this.invoker =
        opts.invoker ||
        (typeof window !== 'undefined'
          ? ((cmd, args) => invoke<HttpResponsePayload>(cmd, args))
          : async () => ({ status: 200, ok: true, headers: {}, body: '{}' }));
    } else {
      this.defaultHeaders = { ...DEFAULT_CHROME_HEADERS, ...(headersOrOptions as Record<string, string> || {}) };
      if (cookies) {
        this.defaultCookies = { ...cookies };
      }
      this.invoker =
        invoker ||
        (typeof window !== 'undefined'
          ? ((cmd, args) => invoke<HttpResponsePayload>(cmd, args))
          : async () => ({ status: 200, ok: true, headers: {}, body: '{}' }));
    }
  }

  setCookie(name: string, value: string): void {
    this.defaultCookies[name] = value;
  }

  setHeader(name: string, value: string): void {
    this.defaultHeaders[name] = value;
  }

  clearCookieCache(): void {
    this.cookieCache.clear();
  }

  private async getCachedCookies(filePath: string, domain?: string): Promise<Record<string, string>> {
    const cacheKey = `${filePath}::${domain || ''}`;
    const cached = this.cookieCache.get(cacheKey);
    if (cached) {
      return cached;
    }
    const loaded = await loadCookiesFromPath(filePath, domain);
    this.cookieCache.set(cacheKey, loaded);
    return loaded;
  }

  async loadCookies(filePath: string, domain?: string): Promise<void> {
    const loaded = await this.getCachedCookies(filePath, domain);
    Object.assign(this.defaultCookies, loaded);
  }

  async request(url: string, options: RequestOptions = {}): Promise<HttpResponse> {
    let cookieHeader = '';
    if (options.cookiePath) {
      const fromPath = await this.getCachedCookies(options.cookiePath, options.cookieDomain);
      const merged = { ...this.defaultCookies, ...fromPath };
      cookieHeader = formatCookieHeader(merged);
    } else if (typeof options.cookies === 'string') {
      cookieHeader = options.cookies;
    } else if (options.cookies) {
      const merged = { ...this.defaultCookies, ...options.cookies };
      cookieHeader = formatCookieHeader(merged);
    } else if (Object.keys(this.defaultCookies).length > 0) {
      cookieHeader = formatCookieHeader(this.defaultCookies);
    }

    const headers: Record<string, string> = {
      ...this.defaultHeaders,
      ...(options.headers || {}),
    };

    if (cookieHeader) {
      headers['Cookie'] = cookieHeader;
    }

    const payload = await this.invoker('open_nami_http_fetch', {
      options: {
        url,
        method: options.method || 'GET',
        headers,
        body: options.body || null,
        cookie_path: options.cookiePath || null,
        timeout_ms: options.timeoutMs || 15000,
      },
    });

    return new HttpResponse(payload);
  }

  async get(url: string, options: RequestOptions = {}): Promise<HttpResponse> {
    return this.request(url, { ...options, method: 'GET' });
  }

  async post(url: string, options: RequestOptions = {}): Promise<HttpResponse> {
    return this.request(url, { ...options, method: 'POST' });
  }

  async getText(url: string, options: RequestOptions = {}): Promise<string> {
    const res = await this.get(url, options);
    return res.text();
  }

  async getJson<T = any>(url: string, options: RequestOptions = {}): Promise<T> {
    const res = await this.get(url, options);
    return res.json<T>();
  }

  async fetchJson<T = unknown>(url: string, options: RequestOptions = {}): Promise<T> {
    const res = await this.request(url, {
      ...options,
      headers: {
        Accept: 'application/json, text/plain, */*',
        ...(options.headers as Record<string, string>),
      },
    });
    if (!res.ok) {
      try {
        await res.body?.cancel();
      } catch {}
      throw new Error(`HTTP ${res.status} (${res.statusText}) for ${url}`);
    }
    const text = await res.text();
    const cleanText = text.replace(/^\s*(?:for\s*\(\s*;\s*;\s*\);|\)\]\}',)/, '').trim();
    return JSON.parse(cleanText) as T;
  }

  async fetchText(url: string, options: RequestOptions = {}): Promise<string> {
    const res = await this.request(url, options);
    if (!res.ok) {
      try {
        await res.body?.cancel();
      } catch {}
      throw new Error(`HTTP ${res.status} (${res.statusText}) for ${url}`);
    }
    return await res.text();
  }
}
