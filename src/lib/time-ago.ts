interface LocaleStrings {
  now: string;
  minAgo: (n: number) => string;
  hourAgo: (n: number) => string;
  yesterday: string;
  days: string[];
  daysAgo: (n: number) => string;
  monthsAgo: (n: number) => string;
  yearsAgo: (n: number) => string;
}

const LOCALES: Record<string, LocaleStrings> = {
  pt: {
    now: "agora",
    minAgo: (n) => `há ${n} min`,
    hourAgo: (n) => `há ${n} h`,
    yesterday: "ontem",
    days: ["domingo", "segunda", "terça", "quarta", "quinta", "sexta", "sábado"],
    daysAgo: (n) => `há ${n} dias`,
    monthsAgo: (n) => `há ${n} ${n === 1 ? "mês" : "meses"}`,
    yearsAgo: (n) => `há ${n} ${n === 1 ? "ano" : "anos"}`,
  },
  en: {
    now: "now",
    minAgo: (n) => `${n} min ago`,
    hourAgo: (n) => `${n} h ago`,
    yesterday: "yesterday",
    days: ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"],
    daysAgo: (n) => `${n} days ago`,
    monthsAgo: (n) => `${n} month${n === 1 ? "" : "s"} ago`,
    yearsAgo: (n) => `${n} year${n === 1 ? "" : "s"} ago`,
  },
};

export default function timeAgo(
  input: Date | string | number | null | undefined,
  locale: string = "pt",
): string {
  if (input === null || input === undefined) return "";
  const d = input instanceof Date ? input : new Date(input);
  if (isNaN(d.getTime())) return "";

  const L = LOCALES[locale] ?? LOCALES.pt;
  const now = Date.now();
  const diffSec = Math.floor((now - d.getTime()) / 1000);

  if (diffSec < 0) return "";
  if (diffSec < 60) return L.now;
  if (diffSec < 3600) return L.minAgo(Math.floor(diffSec / 60));
  if (diffSec < 86400) return L.hourAgo(Math.floor(diffSec / 3600));

  const days = Math.floor(diffSec / 86400);
  if (days === 1) return L.yesterday;
  if (days < 7) return L.days[d.getDay()];
  if (days < 30) return L.daysAgo(days);
  if (days < 365) return L.monthsAgo(Math.max(1, Math.floor(days / 30)));
  return L.yearsAgo(Math.max(1, Math.floor(days / 365)));
}
