const DISALLOWED_FLAGS = new Set([
  "--exec",
  "--exec-before-download",
  "--external-downloader",
  "--downloader",
  "--external-downloader-args",
  "--downloader-args",
  "--config-locations",
  "--config-location",
  "--alias",
  "--print-to-file",
]);

export function isDisallowedYtdlpFlag(flag: string): boolean {
  const trimmed = flag.trim().toLowerCase();
  const name = trimmed.split("=")[0] ?? trimmed;
  return DISALLOWED_FLAGS.has(name);
}

export function validateYtdlpArgs(args: string[]): { valid: boolean; disallowed?: string } {
  for (const arg of args) {
    if (isDisallowedYtdlpFlag(arg)) {
      return { valid: false, disallowed: arg.trim().toLowerCase().split("=")[0] };
    }
  }
  return { valid: true };
}
