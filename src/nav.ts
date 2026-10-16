import type { Component } from "svelte";

import ChevronLeft from "@lucide/svelte/icons/chevron-left";
import ChevronRight from "@lucide/svelte/icons/chevron-right";
import Info from "@lucide/svelte/icons/info";
import Trash2 from "@lucide/svelte/icons/trash-2";
import Zap from "@lucide/svelte/icons/zap";

const PAGE_IDS = {
  cleanup: "cleanup",
  autostart: "autostart",
  about: "about",
} as const;

export type PageId = (typeof PAGE_IDS)[keyof typeof PAGE_IDS];

export const PAGE_LABELS: Record<PageId, string> = {
  cleanup: "Cleanup",
  autostart: "Autostart",
  about: "About",
};

interface NavItem {
  id: PageId;
  label: string;
  icon: Component;
}

interface NavGroup {
  id: string;
  label: string;
  items: NavItem[];
}

export const NAV_GROUPS: NavGroup[] = [
  {
    id: "tools",
    label: "Tools",
    items: [
      { id: PAGE_IDS.cleanup, label: PAGE_LABELS.cleanup, icon: Trash2 },
      { id: PAGE_IDS.autostart, label: PAGE_LABELS.autostart, icon: Zap },
    ],
  },
];

export const SECONDARY_NAV: NavItem[] = [
  { id: PAGE_IDS.about, label: PAGE_LABELS.about, icon: Info },
];

export const SIDEBAR_TOGGLE_LABEL = "Collapse sidebar";

export const SIDEBAR_EXPAND_LABEL = "Expand sidebar";

export function sidebarToggleIcon(expanded: boolean): Component {
  return expanded ? ChevronLeft : ChevronRight;
}