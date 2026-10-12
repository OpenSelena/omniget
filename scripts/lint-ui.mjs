#!/usr/bin/env node
// @ts-nocheck
/**
 * OmniGet Targeted UI Linter (powered by lintapple + Playwright)
 *
 * Audits rendered UI against explicit design policies:
 *   - ui-policy.app.json  : Svelte 5 desktop routes (1100x720 & 800x560, 28px controls, baseline diffing)
 *   - ui-policy.docs.json : Landing page & docs suite (1280x800 & 375x812, strict zero-findings)
 *
 * Usage:
 *   pnpm lint:ui /settings                # Lint a single Svelte route (dark + light)
 *   pnpm lint:ui /courses                 # Lint an embedded plugin route
 *   pnpm lint:ui docs/mcp/index.html      # Lint a single documentation page
 *   pnpm lint:ui index.html               # Lint the root landing page
 *   pnpm lint:ui --shell                  # Lint all core desktop routes
 *   pnpm lint:ui --docs                   # Lint landing page + all 7 doc pages
 *   pnpm lint:ui /settings --all-themes   # Lint across all 14 themes in src/app.css
 *   pnpm lint:ui:update [target]          # Record/refresh baseline in scripts/ui-lint-baseline.json
 */

import { spawn } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { checkPage, validatePolicy } from "lintapple";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const ROOT_DIR = path.resolve(__dirname, "..");
export const APP_POLICY_PATH = path.join(ROOT_DIR, "ui-policy.app.json");
export const DOCS_POLICY_PATH = path.join(ROOT_DIR, "ui-policy.docs.json");
export const BASELINE_PATH = path.join(ROOT_DIR, "scripts", "ui-lint-baseline.json");

export const ALL_APP_THEMES = [
  "dark",
  "light",
  "catppuccin-mocha",
  "catppuccin-macchiato",
  "catppuccin-frappe",
  "catppuccin-latte",
  "one-dark-pro",
  "dracula",
  "nyxvamp-veil",
  "nyxvamp-obsidian",
  "nyxvamp-radiance",
  "eink-day",
  "eink-sepia",
  "eink-night",
];

export const LIGHT_SCHEME_THEMES = new Set([
  "light",
  "catppuccin-latte",
  "nyxvamp-radiance",
  "eink-day",
  "eink-sepia",
]);

export const SHELL_ROUTES = [
  "/",
  "/downloads",
  "/omnidisc",
  "/settings",
  "/marketplace",
  "/about",
  "/open-nami",
  "/league",
  "/tools",
];

export const PLUGIN_ROUTES = [
  "/courses",
  "/convert",
  "/telegram",
  "/study",
];

export const CANONICAL_DOCS = [
  "index.html",
  "docs/index.html",
  "docs/installation/index.html",
  "docs/mcp/index.html",
  "docs/courses/index.html",
  "docs/opennami/index.html",
  "docs/troubleshooting/index.html",
  "docs/changelog/index.html",
];

function normalizeDocTarget(raw) {
  const cleaned = raw.replace(/\\/g, "/").replace(/^\.\//, "");
  if (cleaned.endsWith(".html")) return cleaned;
  return `${cleaned.replace(/\/$/, "")}/index.html`;
}

export function resolveTargets(argv = []) {
  const mode = argv.includes("--update") ? "update" : "check";
  const json = argv.includes("--json");
  const allThemes = argv.includes("--all-themes");
  const isShell = argv.includes("--shell");
  const isDocs = argv.includes("--docs");

  const positional = argv.filter((a) => !a.startsWith("--"));

  if (isDocs) {
    return {
      surface: "docs",
      targets: CANONICAL_DOCS,
      themes: ["dark", "light"],
      mode,
      json,
    };
  }

  if (isShell) {
    return {
      surface: "app",
      targets: SHELL_ROUTES,
      themes: allThemes ? ALL_APP_THEMES : ["dark", "light"],
      mode,
      json,
    };
  }

  if (positional.length === 0) {
    return {
      surface: "app",
      targets: allThemes ? ["/", "/settings"] : ["/"],
      themes: allThemes ? ALL_APP_THEMES : ["dark", "light"],
      mode,
      json,
    };
  }

  const primaryTarget = positional[0].replace(/\\/g, "/");
  const isDocTarget =
    primaryTarget === "index.html" ||
    primaryTarget === "preview.html" ||
    primaryTarget.startsWith("docs/") ||
    primaryTarget === "docs" ||
    primaryTarget.endsWith(".html");

  if (isDocTarget) {
    return {
      surface: "docs",
      targets: positional.map(normalizeDocTarget),
      themes: ["dark", "light"],
      mode,
      json,
    };
  }

  return {
    surface: "app",
    targets: positional.map((t) => (t.startsWith("/") ? t : `/${t}`)),
    themes: allThemes ? ALL_APP_THEMES : ["dark", "light"],
    mode,
    json,
  };
}

/**
 * Computes per-rule counts for a target. When findings carry a `theme` tag,
 * takes the maximum count per rule across audited themes so `--all-themes`
 * (14 themes) compares on the same scale as the 2-theme baseline while still
 * catching any theme-specific regression.
 */
export function tallyFindings(findings = []) {
  const perTheme = new Map();
  for (const f of findings) {
    if (!f || !f.rule) continue;
    const themeKey = f.theme ?? "default";
    let counts = perTheme.get(themeKey);
    if (!counts) {
      counts = {};
      perTheme.set(themeKey, counts);
    }
    counts[f.rule] = (counts[f.rule] ?? 0) + 1;
  }

  const maxByRule = {};
  for (const counts of perTheme.values()) {
    for (const [rule, count] of Object.entries(counts)) {
      maxByRule[rule] = Math.max(maxByRule[rule] ?? 0, count);
    }
  }

  return Object.fromEntries(
    Object.entries(maxByRule).sort(([a], [b]) => a.localeCompare(b))
  );
}

export function compareTargetBaseline(surface, currentCounts = {}, baselineCounts = {}) {
  const worse = [];
  const better = [];

  if (surface === "docs") {
    for (const [rule, now] of Object.entries(currentCounts)) {
      if (now > 0) {
        worse.push({ rule, was: 0, now });
      }
    }
    return { passed: worse.length === 0, worse, better };
  }

  const allRules = new Set([
    ...Object.keys(baselineCounts),
    ...Object.keys(currentCounts),
  ]);
  for (const rule of allRules) {
    const was = baselineCounts[rule] ?? 0;
    const now = currentCounts[rule] ?? 0;
    if (now > was) worse.push({ rule, was, now });
    if (now < was) better.push({ rule, was, now });
  }

  return { passed: worse.length === 0, worse, better };
}

/**
 * Generates the Playwright init script that mocks window.__TAURI_INTERNALS__
 * with populated fixture data for Core and Embedded Plugin routes.
 */
export function buildTauriMockInitScript(theme = "dark") {
  return `
    (() => {
      window.__OMNIGET_LINT_THEME__ = ${JSON.stringify(theme)};
      try {
        localStorage.setItem("omniget_last_seen_version", "0.12.0");
        localStorage.setItem("omniget_docs_theme", ${JSON.stringify(theme)});
        localStorage.setItem("theme", ${JSON.stringify(theme)});
      } catch {}

      const mockSettings = {
        schema_version: 1,
        appearance: { theme: ${JSON.stringify(theme)}, language: "en" },
        download: {
          default_output_dir: "C:\\\\Users\\\\user\\\\Downloads\\\\OmniGet",
          always_ask_path: false,
          video_quality: "1080p",
          skip_existing: true,
          download_attachments: true,
          download_descriptions: false,
          embed_metadata: true,
          embed_thumbnail: true,
          write_nfo_sidecar: false,
          clipboard_detection: false,
          auto_download_on_paste: false,
          filename_template: "%(title)s.%(ext)s",
          organize_by_platform: true,
          download_subtitles: true,
          include_auto_subtitles: false,
          caption_locale: "en",
          keep_vtt: false,
          continuous_lecture_numbers: true,
          translate_metadata: false,
          youtube_sponsorblock: false,
          sponsorblock_mode: "mark",
          sponsorblock_categories: ["sponsor"],
          split_by_chapters: false,
          live_from_start: false,
          speed_limit: "",
          hotkey_enabled: false,
          hotkey_binding: "CommandOrControl+Shift+D",
          music_hotkey_enabled: false,
          music_hotkey_binding: "CommandOrControl+Shift+M",
          music_audio_format: "mp3",
          copy_to_clipboard_on_hotkey: false,
          cookie_file: "",
          always_use_managed_cookies: false,
          bilibili_danmaku_enabled: false,
          bilibili_danmaku_format: "xml",
          bilibili_container: "mp4",
          bilibili_nfo_enabled: false,
          bilibili_cover_sidecar: false,
          bilibili_cover_format: "jpg",
          bilibili_naming_video: "{title}",
          bilibili_naming_multi_part: "{title} - P{part}",
          bilibili_naming_bangumi: "{title}",
          bilibili_naming_cheese: "{title}",
          bilibili_naming_collection: "{title}",
          bilibili_cdn_hosts: "",
          bilibili_cdn_prefer_alternatives: false,
          bilibili_preferred_qn: 80,
          bilibili_preferred_codec: 7,
          bilibili_preferred_audio_qn: 30280
        },
        advanced: {
          max_concurrent_segments: 4,
          max_retries: 3,
          max_concurrent_downloads: 3,
          concurrent_fragments: 4,
          stagger_delay_ms: 200,
          torrent_listen_port: 6881,
          torrent_auto_trackers: true,
          torrent_upnp: true,
          prevent_sleep: false,
          cookies_from_browser: "none",
          twitter_manual_cookie: "",
          user_agent: "Mozilla/5.0"
        },
        telegram: { concurrent_downloads: 2, fix_file_extensions: true },
        rpc: { enabled: false, app_id: "", large_image_key: "" },
        omnidisc: { enabled: true },
        league: { enabled: true, auto_accept: false },
        onboarding_completed: true,
        start_with_system: false,
        start_minimized: false,
        legal_acknowledged: true,
        typography: {
          font_display: "System",
          font_body: "System",
          font_mono: "IBM Plex Mono",
          line_height_base: 1.5,
          spacing_scale: 1.0,
          preset_name: "system"
        }
      };

      const mockPlugins = [
        {
          id: "courses",
          name: "Courses",
          version: "0.12.0",
          enabled: true,
          nav: [{ route: "/courses", label: { en: "Courses" }, icon_svg: null, group: "plugins", order: 10 }]
        },
        {
          id: "telegram",
          name: "Telegram",
          version: "0.12.0",
          enabled: true,
          nav: [{ route: "/telegram", label: { en: "Telegram" }, icon_svg: null, group: "plugins", order: 20 }]
        },
        {
          id: "convert",
          name: "Convert",
          version: "0.12.0",
          enabled: true,
          nav: [{ route: "/convert", label: { en: "Convert" }, icon_svg: null, group: "plugins", order: 30 }]
        },
        {
          id: "study",
          name: "Study",
          version: "0.12.0",
          enabled: true,
          nav: [{ route: "/study", label: { en: "Study" }, icon_svg: null, group: "plugins", order: 40 }]
        }
      ];

      const mockQueue = [
        {
          id: "dl-1",
          url: "https://example.com/lecture-01.mp4",
          title: "01. Introduction to Distributed Systems",
          platform: "YouTube",
          status: "downloading",
          progress: 64.5,
          speed: "8.4 MB/s",
          eta: "00:18",
          total_bytes: 157286400,
          downloaded_bytes: 101449728,
          output_path: "C:\\\\Users\\\\user\\\\Downloads\\\\OmniGet\\\\lecture-01.mp4"
        }
      ];

      let callbackId = 1;
      const callbacks = new Map();

      window.__TAURI_INTERNALS__ = {
        metadata: {
          currentWindow: { label: "main" },
          currentWebview: { windowLabel: "main", label: "main" }
        },
        transformCallback(fn, once) {
          const id = callbackId++;
          callbacks.set(id, { fn, once });
          return id;
        },
        unregisterCallback(id) {
          callbacks.delete(id);
        },
        convertFileSrc(filePath) {
          return filePath;
        },
        async invoke(cmd, args = {}) {
          switch (cmd) {
            case "get_settings":
            case "update_settings":
            case "reset_settings":
              return mockSettings;
            case "list_plugins":
            case "get_marketplace_plugins":
              return mockPlugins;
            case "get_queue":
            case "list_downloads":
              return mockQueue;
            case "check_ytdlp":
            case "is_ytdlp_available":
              return { available: true, version: "2026.10.01", path: "yt-dlp" };
            case "omnidisc_connect":
              return {
                url: args?.url || "https://chat.tonho.wtf",
                instance: { name: "OmniDisc", registration_open: true },
                insecure: false,
              };
            case "omnidisc_has_session":
              return { has_session: false };
            case "omnidisc_voice_status":
              return { connected: false, muted: false, deafened: false, speaking: [] };
            case "omnidisc_voice_devices":
              return { inputs: [], outputs: [] };
            case "omnidisc_voice_ptt_status":
              return { binding: "Control+Shift+M", registered: false };
            case "plugin:courses|courses_list_platforms":
              return [
                { id: "udemy", name: "Udemy", auth_type: "token" },
                { id: "coursera", name: "Coursera", auth_type: "cookie" },
              ];
            case "plugin:telegram|telegram_get_auth_state":
              return { state: "authorized", phone: "+1 555 0100" };
            case "plugin:convert|convert_list_presets":
              return [{ id: "mp4-1080p", label: "MP4 1080p (H.264)" }];
            case "study_list_notebooks":
            case "study_list_decks":
              return [];
            case "plugin:app|version":
              return "0.12.0";
            case "plugin:app|name":
              return "OmniGet";
            case "plugin:event|listen":
              return callbackId++;
            case "plugin:event|unlisten":
              return null;
            case "plugin:clipboard-manager|read_text":
              return "";
            default:
              return null;
          }
        }
      };

      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener() {}
      };
    })();
  `;
}

async function checkTcpPort(host, port) {
  const net = await import("node:net");
  return new Promise((resolve) => {
    const socket = net.createConnection({ host, port });
    socket.setTimeout(800);
    socket.on("connect", () => {
      socket.destroy();
      resolve(true);
    });
    socket.on("timeout", () => {
      socket.destroy();
      resolve(false);
    });
    socket.on("error", () => {
      socket.destroy();
      resolve(false);
    });
  });
}

async function isPortOpen(port) {
  if (await checkTcpPort("127.0.0.1", port)) return "http://127.0.0.1:" + port;
  if (await checkTcpPort("::1", port)) return "http://localhost:" + port;
  return null;
}

async function ensureViteServer(port = 1420) {
  const existingUrl = await isPortOpen(port);
  if (existingUrl) {
    return { url: existingUrl, close: async () => {} };
  }

  const child = spawn(
    process.execPath,
    [
      path.join(ROOT_DIR, "node_modules", "vite", "bin", "vite.js"),
      "dev",
      "--host",
      "127.0.0.1",
      "--port",
      String(port),
    ],
    {
      cwd: ROOT_DIR,
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env, OMNIGET_I18N_STRICT: "0" },
    }
  );

  let logs = "";
  child.stdout.on("data", (d) => {
    logs += d.toString();
  });
  child.stderr.on("data", (d) => {
    logs += d.toString();
  });

  const deadline = Date.now() + 45000;
  while (Date.now() < deadline) {
    const openUrl = await isPortOpen(port);
    if (openUrl) {
      return {
        url: openUrl,
        close: async () => {
          child.kill();
        },
      };
    }
    await new Promise((r) => setTimeout(r, 400));
  }

  child.kill();
  throw new Error(`Timed out waiting for Vite dev server on port ${port}.\nLogs:\n${logs}`);
}

const MIME_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".mjs": "application/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
  ".woff2": "font/woff2",
};

async function detectExistingDocsServer(candidatePorts = [8080, 4173, 3000, 5500]) {
  if (process.env.UI_LINT_DOCS_URL) {
    return process.env.UI_LINT_DOCS_URL.replace(/\/$/, "");
  }
  for (const port of candidatePorts) {
    const openUrl = await isPortOpen(port);
    if (!openUrl) continue;
    try {
      const res = await fetch(`${openUrl}/index.html`, { signal: AbortSignal.timeout(1000) });
      if (res.ok) {
        const text = await res.text();
        if (text.includes("OmniGet") && text.includes("docs/")) {
          return openUrl;
        }
      }
    } catch {
      // Ignore non-docs server on candidate port
    }
  }
  return null;
}

async function ensureStaticDocsServer() {
  const existingUrl = await detectExistingDocsServer();
  if (existingUrl) {
    return { url: existingUrl, close: async () => {} };
  }

  const server = http.createServer((req, res) => {
    const urlPath = decodeURIComponent((req.url || "/").split("?")[0]);
    let relPath = urlPath.replace(/^\/+/, "");
    if (!relPath || relPath.endsWith("/")) {
      relPath = `${relPath}index.html`;
    }
    const absPath = path.resolve(ROOT_DIR, relPath);
    if (!absPath.startsWith(ROOT_DIR) || !fs.existsSync(absPath) || fs.statSync(absPath).isDirectory()) {
      res.writeHead(404);
      res.end("Not found");
      return;
    }
    const ext = path.extname(absPath).toLowerCase();
    res.writeHead(200, { "Content-Type": MIME_TYPES[ext] || "application/octet-stream" });
    fs.createReadStream(absPath).pipe(res);
  });

  return new Promise((resolve, reject) => {
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      resolve({
        url: `http://127.0.0.1:${port}`,
        close: () => new Promise((r) => server.close(() => r())),
      });
    });
  });
}

async function launchChromiumBrowser() {
  for (const opts of [{}, { channel: "chrome" }, { channel: "msedge" }]) {
    try {
      return await chromium.launch(opts);
    } catch {
      // Try next channel
    }
  }
  throw new Error(
    "No Chromium/Chrome/Edge browser found. Run `npx playwright install chromium --only-shell`."
  );
}

async function auditTarget(browser, baseUrl, surface, target, themes, policy) {
  const findings = [];

  for (const viewport of policy.viewports) {
    for (const theme of themes) {
      const isDarkScheme = !LIGHT_SCHEME_THEMES.has(theme);
      const colorScheme = isDarkScheme ? "dark" : "light";
      const context = await browser.newContext({
        viewport,
        colorScheme,
        reducedMotion: "reduce",
        locale: "en-US",
        timezoneId: "UTC",
        deviceScaleFactor: 1,
      });

      try {
        if (surface === "app") {
          await context.addInitScript(buildTauriMockInitScript(theme));
        } else {
          await context.addInitScript((t) => {
            try {
              localStorage.setItem("omniget_docs_theme", t);
              localStorage.setItem("theme", t);
            } catch {}
          }, theme);
        }

        const page = await context.newPage();
        page.setDefaultTimeout(60000);
        const url =
          surface === "app"
            ? `${baseUrl}${target}`
            : `${baseUrl}/${target.replace(/^\/+/, "")}`;

        await page.goto(url, { waitUntil: "load", timeout: 60000 });
        if (surface === "app") {
          await page
            .locator("#main-content, .stream-popout")
            .first()
            .waitFor({ state: "visible", timeout: 60000 });
        }
        await page.evaluate(
          ({ t, isDark }) => {
            document.documentElement.setAttribute("data-theme", t);
            document.documentElement.classList.toggle("dark", isDark);
          },
          { t: theme, isDark: isDarkScheme }
        );
        await page.waitForTimeout(200);

        const result = await checkPage(page, policy);
        for (const f of result.findings) {
          findings.push({
            ...f,
            target,
            theme,
            viewport: `${viewport.width}x${viewport.height}`,
          });
        }
      } finally {
        await context.close();
      }
    }
  }

  return findings;
}

async function main() {
  const plan = resolveTargets(process.argv.slice(2));
  const policyPath = plan.surface === "app" ? APP_POLICY_PATH : DOCS_POLICY_PATH;
  const policy = validatePolicy(JSON.parse(fs.readFileSync(policyPath, "utf-8")));

  const existingBaseline = fs.existsSync(BASELINE_PATH)
    ? JSON.parse(fs.readFileSync(BASELINE_PATH, "utf-8"))
    : { targets: {} };

  const server =
    plan.surface === "app"
      ? await ensureViteServer(1420)
      : await ensureStaticDocsServer();

  const browser = await launchChromiumBrowser();
  const targetFindings = {};
  const targetCounts = {};

  try {
    for (const target of plan.targets) {
      const findings = await auditTarget(
        browser,
        server.url,
        plan.surface,
        target,
        plan.themes,
        policy
      );
      targetFindings[target] = findings;
      targetCounts[target] = tallyFindings(findings);
    }
  } finally {
    await browser.close();
    await server.close();
  }

  if (plan.mode === "update") {
    const mergedTargets = { ...(existingBaseline.targets ?? {}), ...targetCounts };
    const updated = {
      updated_at: new Date().toISOString().split("T")[0],
      targets: Object.fromEntries(
        Object.entries(mergedTargets).sort(([a], [b]) => a.localeCompare(b))
      ),
    };
    fs.writeFileSync(BASELINE_PATH, JSON.stringify(updated, null, 2) + "\n");
    console.log(
      `UI lint baseline updated for ${plan.targets.join(", ")} -> ${path.relative(ROOT_DIR, BASELINE_PATH)}`
    );
    process.exit(0);
  }

  if (plan.json) {
    console.log(JSON.stringify({ plan, counts: targetCounts, findings: targetFindings }, null, 2));
  }

  let hasRegression = false;
  for (const target of plan.targets) {
    const counts = targetCounts[target];
    const baselineForTarget = existingBaseline.targets?.[target] ?? {};
    const comparison = compareTargetBaseline(plan.surface, counts, baselineForTarget);

    if (!comparison.passed) {
      hasRegression = true;
      console.error(`\n[FAIL] UI lint regression on ${target}:`);
      for (const { rule, was, now } of comparison.worse) {
        console.error(`  ${rule}: ${was} -> ${now} (+${now - was})`);
        const examples = targetFindings[target]
          .filter((f) => f.rule === rule)
          .slice(0, 3);
        for (const ex of examples) {
          console.error(
            `    - [${ex.theme} @ ${ex.viewport}] ${ex.selector}: ${ex.message} (Fix: ${ex.fix})`
          );
        }
      }
    } else {
      const total = Object.values(counts).reduce((a, b) => a + b, 0);
      console.log(`[PASS] ${target} (${total} baselined findings, 0 new)`);
      if (comparison.better.length > 0) {
        console.log(
          `  (${comparison.better.length} rule(s) improved on ${target} — run \`pnpm lint:ui:update ${target}\` to ratchet baseline down)`
        );
      }
    }
  }

  process.exit(hasRegression ? 1 : 0);
}

const isDirectRun =
  process.argv[1] &&
  path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url));

if (isDirectRun) {
  main().catch((err) => {
    console.error(err);
    process.exit(2);
  });
}
