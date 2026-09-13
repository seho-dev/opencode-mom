import { Bot, Boxes, LayoutDashboard, Network, Server } from '@lucide/svelte';
import type { MessageKey } from '$lib/features/i18n/dictionaries/en.js';

type NavigationItem = { href: string; labelKey: MessageKey; icon: typeof LayoutDashboard };

export const navigationItems: NavigationItem[] = [
  { href: '/', labelKey: 'nav.dashboard', icon: LayoutDashboard },
  { href: '/providers', labelKey: 'nav.providers', icon: Server },
  { href: '/models', labelKey: 'nav.models', icon: Boxes },
  { href: '/agents', labelKey: 'nav.agents', icon: Bot },
  { href: '/groups', labelKey: 'nav.groups', icon: Network },
];

export function isNavigationItemActive(pathname: string, href: string) {
  return href === '/' ? pathname === '/' : pathname.startsWith(href);
}
