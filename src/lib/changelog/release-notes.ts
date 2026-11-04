// Release notes shipped with the app, so the About > Changelog page can show
// them translated and offline. Each entry is an i18n key under
// `changelog.notes`; the English text lives in en.json like every other
// string. Versions without bundled notes fall back to the GitHub release.

export interface ReleaseNotesSection {
  title: string;
  items: string[];
}

export interface ReleaseNotes {
  version: string;
  sections: ReleaseNotesSection[];
}

const section = (id: string, items: string[]): ReleaseNotesSection => ({
  title: `changelog.notes.${id}.title`,
  items: items.map((item) => `changelog.notes.${id}.${item}`),
});

export const RELEASE_NOTES: ReleaseNotes[] = [
  {
    version: "0.15.0",
    sections: [
      section("features", [
        "ytdlp_auto",
        "drm",
        "reframe",
        "subtitles",
        "phone_qr",
        "snapchat",
        "file_actions",
        "slideshow",
        "h265",
        "publisher",
      ]),
      section("security", [
        "ytdlp_options",
        "extension_pairing",
        "checksums",
        "links",
        "local_network",
        "plugins",
        "phone_share",
        "mcp",
        "ci",
      ]),
      section("fixes", [
        "newest_first",
        "torrent_size",
        "errors_translated",
        "video_ops",
        "windows_build",
        "translations",
      ]),
      section("deps", ["rust", "npm", "toolchain", "katex", "ffmpeg"]),
    ],
  },
];

export function releaseNotesFor(version: string): ReleaseNotes | undefined {
  return RELEASE_NOTES.find((notes) => notes.version === version);
}
