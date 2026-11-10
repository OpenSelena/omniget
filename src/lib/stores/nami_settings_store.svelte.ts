import { saveAppSettings, loadAppSettings } from "$lib/api/nami";

let outputDir = $state("");
let cookiesFile = $state("");
let browserCookies = $state("auto");
let loaded = $state(false);

export function getOutputDir(): string {
  return outputDir;
}

export function getCookiesFile(): string {
  return cookiesFile;
}

export function getBrowserCookies(): string {
  return browserCookies;
}

export function isLoaded(): boolean {
  return loaded;
}

function cleanPathLike(value: string): string {
  const trimmed = value.trim();
  if (
    trimmed.length >= 2 &&
    ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
      (trimmed.startsWith("'") && trimmed.endsWith("'")))
  ) {
    return trimmed.slice(1, -1).trim();
  }
  return trimmed;
}

export async function loadSettings() {
  try {
    const settings = await loadAppSettings();
    outputDir = settings.output_directory ?? "";
    cookiesFile = settings.cookies_file ?? "";
    browserCookies = settings.browser_cookies ?? "auto";
  } catch (error) {
    console.error("Failed to load settings:", error);
  } finally {
    loaded = true;
  }
}

export async function saveSettings(
  newOutputDir: string,
  newCookiesFile: string,
  newBrowserCookies?: string
): Promise<void> {
  const cleanOutputDir = cleanPathLike(newOutputDir);
  const cleanCookiesFile = newCookiesFile.trim() ? cleanPathLike(newCookiesFile) : "";
  const cleanBrowserCookies = newBrowserCookies !== undefined ? newBrowserCookies.trim() : browserCookies;

  await saveAppSettings(
    cleanOutputDir || null,
    cleanCookiesFile || null,
    cleanBrowserCookies || null
  );

  outputDir = cleanOutputDir;
  cookiesFile = cleanCookiesFile;
  browserCookies = cleanBrowserCookies;
}
