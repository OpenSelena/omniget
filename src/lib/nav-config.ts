export interface NavItem {
  href: string;
  labelKey: string;
  icon: string;
  group: "primary" | "secondary";
  badge?: "downloads";
}

export const NAV_ITEMS: NavItem[] = [
  { href: "/", labelKey: "nav.home", icon: "home", group: "primary" },
  { href: "/downloads", labelKey: "nav.downloads", icon: "downloads", group: "primary", badge: "downloads" },
  { href: "/courses", labelKey: "nav.courses", icon: "courses", group: "primary" },
  { href: "/convert", labelKey: "nav.convert", icon: "convert", group: "secondary" },
  { href: "/telegram", labelKey: "nav.telegram", icon: "telegram", group: "secondary" },
  { href: "/settings", labelKey: "nav.settings", icon: "settings", group: "secondary" },
  { href: "/about", labelKey: "nav.about", icon: "about", group: "secondary" },
];
