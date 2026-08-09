import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

import packageText from '../package.json?raw';
import tauriConfigText from '../src-tauri/tauri.conf.json?raw';

type WindowContract = Readonly<{
  label: string;
  width: number;
  height: number;
  resizable: boolean;
}>;

type TauriConfig = Readonly<{
  app: Readonly<{
    windows: readonly WindowContract[];
    security: Readonly<{
      csp: string;
      capabilities: readonly string[];
    }>;
  }>;
}>;

type PackageManifest = Readonly<{
  dependencies: Readonly<Record<string, string>>;
  devDependencies: Readonly<Record<string, string>>;
}>;

const projectRoot = resolve(import.meta.dirname, '..');

const readProjectFile = (path: string): string => readFileSync(resolve(projectRoot, path), 'utf8');

const expectedCsp = "default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'";

const packageManifest = JSON.parse(packageText) as PackageManifest;
const tauriConfig = JSON.parse(tauriConfigText) as TauriConfig;

describe('UI system native contracts', () => {
  it('Given the Tauri config When Quick Switch is resolved Then its fixed compact window contract is unchanged', () => {
    expect(tauriConfig.app.windows).toContainEqual(expect.objectContaining({
      label: 'quick-switch',
      width: 420,
      height: 360,
      resizable: false
    }));
  });

  it('Given the Tauri config When Settings is resolved Then its resizable desktop window contract is unchanged', () => {
    expect(tauriConfig.app.windows).toContainEqual(expect.objectContaining({
      label: 'settings',
      width: 980,
      height: 620,
      resizable: true
    }));
  });

  it('Given the Tauri config When security is inspected Then CSP and capability labels are unchanged', () => {
    expect(tauriConfig.app.security.csp).toBe(expectedCsp);
    expect(tauriConfig.app.security.capabilities).toEqual(['quick-switch', 'settings']);
  });
});

describe('UI system foundation contracts', () => {
  it('Given the package manifest When foundation dependencies are inspected Then exact direct versions are pinned without legacy theme packages', () => {
    expect(packageManifest.dependencies).toMatchObject({
      '@lucide/svelte': '1.24.0',
      clsx: '2.1.1',
      'tailwind-merge': '3.6.0',
      'tailwind-variants': '3.2.2',
      'tw-animate-css': '1.4.0'
    });
    expect(packageManifest.devDependencies).toMatchObject({
      '@internationalized/date': '3.12.2',
      '@tailwindcss/vite': '4.3.3',
      'bits-ui': '2.18.1',
      tailwindcss: '4.3.3'
    });
    expect(packageManifest.dependencies).not.toHaveProperty('lucide-svelte');
    expect(packageManifest.dependencies).not.toHaveProperty('mode-watcher');
    expect(packageManifest.devDependencies).not.toHaveProperty('mode-watcher');
    expect(packageManifest.dependencies).not.toHaveProperty('shadcn-svelte');
    expect(packageManifest.devDependencies).not.toHaveProperty('shadcn-svelte');
  });

  it('Given the manual shadcn config When parsed Then it matches the generator-free Vega contract exactly', () => {
    const componentsPath = resolve(projectRoot, 'components.json');

    expect(existsSync(componentsPath)).toBe(true);
    expect(JSON.parse(readFileSync(componentsPath, 'utf8'))).toEqual({
      $schema: 'https://shadcn-svelte.com/schema.json',
      tailwind: {
        css: 'src/app.css',
        baseColor: 'neutral'
      },
      aliases: {
        lib: '$lib',
        utils: '$lib/utils',
        components: '$lib/components',
        ui: '$lib/components/ui',
        hooks: '$lib/hooks'
      },
      typescript: true,
      registry: 'https://shadcn-svelte.com/registry/styles/vega'
    });
  });

  it('Given the Vite config When plugins are read Then Tailwind runs before SvelteKit', () => {
    const viteConfig = readProjectFile('vite.config.ts');

    expect(viteConfig).toContain("import tailwindcss from '@tailwindcss/vite';");
    expect(viteConfig.indexOf('tailwindcss()')).toBeGreaterThan(-1);
    expect(viteConfig.indexOf('tailwindcss()')).toBeLessThan(viteConfig.indexOf('sveltekit()'));
  });

  it('Given the shared class helpers When their source is inspected Then the registry-compatible strict types are exported', () => {
    const utilsPath = resolve(projectRoot, 'src/lib/utils.ts');

    expect(existsSync(utilsPath)).toBe(true);
    const utils = readFileSync(utilsPath, 'utf8');
    expect(utils).toContain("import { clsx, type ClassValue } from 'clsx';");
    expect(utils).toContain("import { twMerge } from 'tailwind-merge';");
    expect(utils).toContain('export function cn(...inputs: ClassValue[])');
    expect(utils).toContain('return twMerge(clsx(inputs));');
    expect(utils).toContain('export type WithoutChild<T> = T extends { child?: unknown } ? Omit<T, \'child\'> : T;');
    expect(utils).toContain('export type WithoutChildren<T> = T extends { children?: unknown } ? Omit<T, \'children\'> : T;');
    expect(utils).toContain('export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;');
    expect(utils).toContain('export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };');
    expect(utils).not.toMatch(/\bany\b|@ts-ignore|@ts-expect-error/);
  });

  it('Given the root stylesheet When foundation CSS is inspected Then imports, tokens, aliases, and OS theming remain authoritative', () => {
    const appCssPath = resolve(projectRoot, 'src/app.css');

    expect(existsSync(appCssPath)).toBe(true);
    const appCss = readFileSync(appCssPath, 'utf8');
    expect(appCss).toContain('@import "tailwindcss";');
    expect(appCss).toContain('@import "tw-animate-css";');
    expect(appCss).toContain('--radius: 8px;');
    expect(appCss).toContain('--text-on-accent: #ffffff;');
    expect(appCss).toContain('--text-on-accent: #111315;');
    expect(appCss).toContain('@media (prefers-color-scheme: dark)');
    expect(appCss).toContain('--background: var(--surface-app);');
    expect(appCss).toContain('--foreground: var(--text-primary);');
    expect(appCss).toContain('--card: var(--surface-panel);');
    expect(appCss).toContain('--card-foreground: var(--text-primary);');
    expect(appCss).toContain('--popover: var(--surface-panel);');
    expect(appCss).toContain('--popover-foreground: var(--text-primary);');
    expect(appCss).toContain('--primary: var(--accent-primary);');
    expect(appCss).toContain('--primary-foreground: var(--text-on-accent);');
    expect(appCss).toContain('--secondary: var(--surface-muted);');
    expect(appCss).toContain('--secondary-foreground: var(--text-primary);');
    expect(appCss).toContain('--muted: var(--surface-muted);');
    expect(appCss).toContain('--muted-foreground: var(--text-secondary);');
    expect(appCss).toContain('--accent: var(--surface-hover);');
    expect(appCss).toContain('--accent-foreground: var(--text-primary);');
    expect(appCss).toContain('--destructive: var(--status-error);');
    expect(appCss).toContain('--border: var(--border-default);');
    expect(appCss).toContain('--input: var(--border-default);');
    expect(appCss).toContain('--ring: var(--focus-ring);');
    expect(appCss).toContain('@theme inline');
    expect(appCss).toContain('--color-success: var(--status-success);');
    expect(appCss).toContain('--color-warning: var(--status-warning);');
    expect(appCss).toContain('--color-info: var(--status-info);');
    expect(appCss).toContain('--radius-sm: calc(var(--radius) - 4px);');
    expect(appCss).toContain('--radius-md: calc(var(--radius) - 2px);');
    expect(appCss).toContain('--radius-lg: var(--radius);');
    expect(appCss).toContain('--radius-xl: calc(var(--radius) + 4px);');
    expect(appCss).toContain('@media (prefers-reduced-motion: reduce)');
    expect(appCss).toContain('animation-duration: 0.01ms !important;');
    expect(appCss).toContain('animation-iteration-count: 1 !important;');
    expect(appCss).toContain('transition-duration: 0.01ms !important;');
    expect(appCss).toContain('scroll-behavior: auto !important;');
    expect(appCss).not.toMatch(/@custom-variant\s+dark|\.dark\s*\{/);
    expect(appCss).not.toMatch(/outline\s*:\s*(?:0|none)/);
  });

  it('Given the route styles When global CSS ownership is inspected Then app.css is the single root stylesheet', () => {
    const layout = readProjectFile('src/routes/+layout.svelte');
    const page = readProjectFile('src/routes/+page.svelte');

    expect(layout).toContain("import '../app.css';");
    expect(layout).not.toContain(':global(');
    expect(page).not.toContain(':global(body)');
    expect(existsSync(resolve(projectRoot, 'src/app.css'))).toBe(true);
  });
});
