import { saveAppSettings, loadAppSettings } from "$lib/api/open_omni";

let outputDir = $state("");
let cookiesFile = $state("");
let loaded = $state(false);

export function getOutputDir(): string {
  return outputDir;
}

export function getCookiesFile(): string {
  return cookiesFile;
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
  } catch (error) {
    console.error("Failed to load settings:", error);
  } finally {
    loaded = true;
  }
}

export async function saveSettings(newOutputDir: string, newCookiesFile: string): Promise<void> {
  const cleanOutputDir = cleanPathLike(newOutputDir);
  const cleanCookiesFile = newCookiesFile.trim() ? cleanPathLike(newCookiesFile) : "";

  await saveAppSettings(cleanOutputDir || null, cleanCookiesFile || null);

  outputDir = cleanOutputDir;
  cookiesFile = cleanCookiesFile;
}


