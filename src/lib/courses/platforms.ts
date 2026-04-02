export interface CoursePlatform {
  id: string;
  name: string;
  route: string;
  icon: string;
  color: string;
  enabled: boolean;
  authCheckCommand?: string;
}

export const COURSE_PLATFORMS: CoursePlatform[] = [
  {
    id: "hotmart",
    name: "Hotmart",
    route: "/courses/hotmart",
    icon: "hotmart",
    color: "#F04E23",
    enabled: true,
    authCheckCommand: "hotmart_check_session",
  },
  {
    id: "udemy",
    name: "Udemy",
    route: "/courses/udemy",
    icon: "udemy",
    color: "#A435F0",
    enabled: true,
    authCheckCommand: "udemy_check_session",
  },
  {
    id: "kiwify",
    name: "Kiwify",
    route: "/courses/kiwify",
    icon: "kiwify",
    color: "#22C55E",
    enabled: true,
    authCheckCommand: "kiwify_check_session",
  },
  {
    id: "gumroad",
    name: "Gumroad",
    route: "/courses/gumroad",
    icon: "gumroad",
    color: "#FF90E8",
    enabled: true,
  },
  {
    id: "teachable",
    name: "Teachable",
    route: "/courses/teachable",
    icon: "teachable",
    color: "#4B5563",
    enabled: true,
  },
  {
    id: "kajabi",
    name: "Kajabi",
    route: "/courses/kajabi",
    icon: "kajabi",
    color: "#2563EB",
    enabled: true,
  },
  {
    id: "skool",
    name: "Skool",
    route: "/courses/skool",
    icon: "skool",
    color: "#5865F2",
    enabled: true,
  },
  {
    id: "greatcourses",
    name: "Wondrium",
    route: "/courses/greatcourses",
    icon: "greatcourses",
    color: "#1E3A5F",
    enabled: true,
  },
  {
    id: "thinkific",
    name: "Thinkific",
    route: "/courses/thinkific",
    icon: "thinkific",
    color: "#4A90D9",
    enabled: true,
    authCheckCommand: "thinkific_check_session",
  },
  {
    id: "rocketseat",
    name: "Rocketseat",
    route: "/courses/rocketseat",
    icon: "rocketseat",
    color: "#8257E5",
    enabled: true,
    authCheckCommand: "rocketseat_check_session",
  },
];
