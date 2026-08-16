const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 31536000],
  ["month", 2592000],
  ["day", 86400],
  ["hour", 3600],
  ["minute", 60],
  ["second", 1],
];

export default function timeAgo(
  input: Date | string | number | null | undefined,
  locale: string = "en",
): string {
  if (input == null) return "";
  const d = input instanceof Date ? input : new Date(input);
  const time = d.getTime();
  if (isNaN(time)) return "";

  const diffSec = Math.round((time - Date.now()) / 1000);
  if (Math.abs(diffSec) < 10) return locale.startsWith("pt") ? "agora" : "now";

  try {
    const rtf = new Intl.RelativeTimeFormat(locale || "en", { numeric: "auto" });
    for (const [unit, seconds] of UNITS) {
      if (Math.abs(diffSec) >= seconds || unit === "second") {
        return rtf.format(Math.round(diffSec / seconds), unit);
      }
    }
  } catch {
    // Fallback for unexpected locale errors
  }
  return "";
}
