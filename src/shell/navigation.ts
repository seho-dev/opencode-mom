import { BookOpen, Bot, Boxes, Cable, LayoutDashboard, Network, Server, Settings } from '@lucide/svelte';
import type { MessageKey } from '$src/i18n/dictionaries/en.js';

type NavigationItem = { href: string; labelKey: MessageKey; icon: typeof LayoutDashboard };

export const navigationItems: NavigationItem[] = [
  { href: '/', labelKey: 'nav.dashboard', icon: LayoutDashboard },
  { href: '/providers', labelKey: 'nav.providers', icon: Server },
  { href: '/models', labelKey: 'nav.models', icon: Boxes },
  { href: '/agents', labelKey: 'nav.agents', icon: Bot },
  { href: '/groups', labelKey: 'nav.groups', icon: Network },
  { href: '/mcp', labelKey: 'nav.mcp', icon: Cable },
  { href: '/skills', labelKey: 'nav.skills', icon: BookOpen },
  { href: '/settings', labelKey: 'nav.settings', icon: Settings },
];

export function isNavigationItemActive(pathname: string, href: string) {
  return href === '/' ? pathname === '/' : pathname === href || pathname.startsWith(`${href}/`);
}
