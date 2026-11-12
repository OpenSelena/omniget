import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const repoRoot = path.resolve(__dirname, "../../..");

describe("Home v1-hero UI and 4K/2K Quality & Mode wiring", () => {
  it("passes {selectedFormatId} (not bare boolean) to QualityPicker and hides it in audio mode", () => {
    const homePage = fs.readFileSync(
      path.join(repoRoot, "src/routes/+page.svelte"),
      "utf8",
    );
    expect(homePage).toContain(
      "<QualityPicker bind:selectedQuality {selectedFormatId} {availableHeights} {hasAudioOnly} />",
    );
    expect(homePage).not.toContain("selectedFormatId {availableHeights}");
    expect(homePage).toContain('{#if downloadMode !== "audio"}');
  });

  it("exposes 4K (2160p) and 2K (1440p) in both QualityPicker and Settings QualitySection", () => {
    const qualityPicker = fs.readFileSync(
      path.join(repoRoot, "src/components/omnibox/QualityPicker.svelte"),
      "utf8",
    );
    expect(qualityPicker).toContain('{ value: "2160p", key: "omnibox.quality_2160p", height: 2160 }');
    expect(qualityPicker).toContain('{ value: "1440p", key: "omnibox.quality_1440p", height: 1440 }');
    expect(qualityPicker).not.toContain("o.height <= 1080");

    const qualitySection = fs.readFileSync(
      path.join(repoRoot, "src/components/settings/downloads/QualitySection.svelte"),
      "utf8",
    );
    expect(qualitySection).toContain('<option value="2160p">{$t(\'omnibox.quality_2160p\')}</option>');
    expect(qualitySection).toContain('<option value="1440p">{$t(\'omnibox.quality_1440p\')}</option>');
  });

  it("uses Video mode label and Quality and more toggle label in en.json", () => {
    const en = JSON.parse(
      fs.readFileSync(path.join(repoRoot, "src/lib/i18n/en.json"), "utf8"),
    );
    expect(en.omnibox.mode_auto).toBe("Video");
    expect(en.omnibox.options).toBe("Quality and more");
    expect(en.omnibox.detecting).toBe("Checking link...");
    expect(en.omnibox.added_to_downloads).toBe("Added to Downloads");
    expect(en.omnibox.open_downloads).toBe("Open Downloads");
    expect(en.downloads.view_downloads).toBe("View downloads");
  });

  it("renders 3-line MediaPreview info (title, author, duration) without thumbnail overlay badge", () => {
    const mediaPreview = fs.readFileSync(
      path.join(repoRoot, "src/components/omnibox/MediaPreview.svelte"),
      "utf8",
    );
    expect(mediaPreview).toContain('class="preview-title"');
    expect(mediaPreview).toContain('class="preview-author"');
    expect(mediaPreview).toContain('class="preview-duration"');
    expect(mediaPreview).not.toContain("preview-duration-badge");
  });
});
