import { Bot, Boxes, LayoutDashboard, Network, Server } from '@lucide/svelte';

export const navigationItems = [
  { href: '/', label: 'Dashboard', icon: LayoutDashboard },
  { href: '/providers', label: 'Providers', icon: Server },
  { href: '/models', label: 'Models', icon: Boxes },
  { href: '/agents', label: 'Agents', icon: Bot },
  { href: '/groups', label: 'Groups', icon: Network },
];

export function isNavigationItemActive(pathname: string, href: string) {
  return href === '/' ? pathname === '/' : pathname.startsWith(href);
}
