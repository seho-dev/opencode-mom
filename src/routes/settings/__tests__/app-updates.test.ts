import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { AppInfo, ProjectPage, UpdateCheck } from '$src/types/app.js';
import AppUpdates from '../AppUpdates.svelte';

const latest: UpdateCheck = { currentVersion: '0.1.0', latestVersion: '0.2.0', updateAvailable: true };
const current: UpdateCheck = { currentVersion: '0.1.0', latestVersion: '0.1.0', updateAvailable: false };
const quarantineCommand = 'xattr -dr com.apple.quarantine "/Applications/opencode-mom.app"';

function mount(overrides: Record<string, unknown> = {}) {
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    getAppInfo: vi.fn<() => Promise<AppInfo>>().mockResolvedValue({ version: '0.1.0', platform: 'macos' }),
    checkForUpdates: vi.fn<() => Promise<UpdateCheck>>().mockResolvedValue(current),
    openProjectPage: vi.fn<(page: ProjectPage) => Promise<void>>().mockResolvedValue(undefined),
    ...overrides,
  };
  const view = render(AppUpdates, {}, { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } });
  return { config, ...view };
}

test('reads installed version locally, checks only on click, guards pending and reports up to date', async () => {
  let finish: (value: UpdateCheck) => void = () => {};
  const checkForUpdates = vi.fn().mockImplementation(
    () =>
      new Promise<UpdateCheck>((resolve) => {
        finish = resolve;
      }),
  );
  const { config } = mount({ checkForUpdates });
  await screen.findByText('Installed version: 0.1.0');
  expect(checkForUpdates).not.toHaveBeenCalled();
  expect(config.openProjectPage).not.toHaveBeenCalled();
  const checkButton = screen.getByRole('button', { name: 'Check for updates' }) as HTMLButtonElement;
  await fireEvent.click(checkButton);
  expect(checkButton.disabled).toBe(true);
  expect(checkButton.getAttribute('aria-busy')).toBe('true');
  expect(checkButton.getAttribute('aria-label')).toBe('Checking for updates…');
  expect(checkButton.getAttribute('title')).toBe('Checking for updates…');
  expect(checkButton.textContent?.trim()).toBe('');
  expect(checkButton.querySelector('svg')?.classList.contains('animate-spin')).toBe(true);
  expect(checkButton.querySelector('svg')?.classList.contains('motion-reduce:animate-none')).toBe(true);
  expect(screen.getByRole('status').textContent).toContain('Checking for updates…');
  await fireEvent.click(checkButton);
  expect(checkForUpdates).toHaveBeenCalledOnce();
  finish(current);
  await screen.findByText('Up to date (0.1.0).');
  expect(checkButton.disabled).toBe(false);
  expect(checkButton.getAttribute('aria-label')).toBe('Check for updates');
  expect(checkButton.getAttribute('title')).toBe('Check for updates');
  expect(checkButton.querySelector('svg')?.classList.contains('animate-spin')).toBe(false);
  expect(screen.queryByRole('button', { name: 'View GitHub downloads' })).toBeNull();
  expect(screen.queryByText('Installing an update on macOS')).toBeNull();
});

test('new macOS release opens a GUI-first security guide with an optional command, downloads and Star', async () => {
  const { config } = mount({ checkForUpdates: vi.fn().mockResolvedValue(latest) });
  await screen.findByText('Installed version: 0.1.0');
  expect(screen.getByRole('button', { name: 'Star on GitHub' })).toBeTruthy();
  expect(config.openProjectPage).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
  await screen.findByText('Version 0.2.0 is available.');
  const summary = screen.getByText('Installing an update on macOS');
  const details = summary.closest('details') as HTMLDetailsElement;
  expect(details.open).toBe(true);
  expect(details.textContent).toContain('ad-hoc signed');
  expect(details.textContent).toContain('notarized by Apple');
  expect(details.textContent).toContain('Never bypass warnings for unknown downloads.');
  expect(details.textContent).toContain('Quit from the tray menu (closing the window only hides it)');
  expect(details.textContent).toContain('replace the app in Applications');
  expect(details.textContent).toContain('If blocked, use System Settings → Privacy & Security → Open Anyway');
  expect(details.textContent).toContain('The Terminal command is usually unnecessary');
  expect(details.textContent).toContain(
    'Optional fallback: if the trusted download is still blocked, run this in Terminal',
  );
  expect(details.textContent).toContain('no sudo, no global Gatekeeper changes.');
  const command = screen.getByText(quarantineCommand);
  expect(command.tagName).toBe('CODE');
  expect(command.textContent).toBe(quarantineCommand);
  expect(command.classList.contains('select-text')).toBe(true);
  expect(details.querySelector('button')).toBeNull();
  expect(screen.queryByRole('dialog')).toBeNull();
  await fireEvent.click(summary);
  expect(details.open).toBe(false);
  await fireEvent.click(summary);
  expect(details.open).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'View GitHub downloads' }));
  expect(config.openProjectPage).toHaveBeenCalledExactlyOnceWith('releases');
  await waitFor(() =>
    expect((screen.getByRole('button', { name: 'Star on GitHub' }) as HTMLButtonElement).disabled).toBe(false),
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Star on GitHub' }));
  expect(config.openProjectPage).toHaveBeenNthCalledWith(2, 'repository');
  expect(screen.getByText(/100 stars/).textContent).toContain('I’ll buy an Apple Developer certificate');
});

test.each(['windows', 'linux', 'other'] as const)('does not show macOS guidance on native %s', async (platform) => {
  mount({
    getAppInfo: vi.fn().mockResolvedValue({ version: '0.1.0', platform }),
    checkForUpdates: vi.fn().mockResolvedValue(latest),
  });
  await screen.findByText('Installed version: 0.1.0');
  await fireEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
  await screen.findByText('Version 0.2.0 is available.');
  expect(screen.queryByText('Installing an update on macOS')).toBeNull();
  expect(screen.queryByText(quarantineCommand)).toBeNull();
});

test('unknown platform stays honest and local app-info retry can enable macOS guidance', async () => {
  const getAppInfo = vi.fn().mockRejectedValueOnce(new Error('Desktop application required')).mockResolvedValueOnce({
    version: '0.1.0',
    platform: 'macos',
  });
  const { config } = mount({ getAppInfo, checkForUpdates: vi.fn().mockResolvedValue(latest) });
  expect((await screen.findByRole('alert')).textContent).toContain('Desktop application required');
  await fireEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
  await screen.findByText('Version 0.2.0 is available.');
  expect(screen.queryByText('Installing an update on macOS')).toBeNull();
  expect(screen.getByText('Installed version: 0.1.0')).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Retry app details' }));
  await screen.findByText('Installing an update on macOS');
  expect(getAppInfo).toHaveBeenCalledTimes(2);
  expect(config.checkForUpdates).toHaveBeenCalledOnce();
});

test('reports no published release without offering a download', async () => {
  mount({ checkForUpdates: vi.fn().mockResolvedValue({ ...current, latestVersion: null }) });
  await fireEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
  expect((await screen.findByRole('status')).textContent).toContain('No published release is available yet.');
  expect(screen.queryByRole('button', { name: 'View GitHub downloads' })).toBeNull();
});

test.each([
  ['en', 'Check for updates', 'Retry update check'],
  ['zh', '检查更新', '重试检查更新'],
] as const)(
  '%s update action is icon-only with localized check and retry names and tooltips',
  async (locale, check, retry) => {
    const checkForUpdates = vi
      .fn()
      .mockRejectedValueOnce(new Error('GitHub rate limit'))
      .mockResolvedValueOnce(current);
    mount({ preferences: { locale, theme: 'dark' }, checkForUpdates });
    const button = screen.getByRole('button', { name: check });
    expect(checkForUpdates).not.toHaveBeenCalled();
    for (const label of [check, retry]) {
      expect(screen.getByRole('button', { name: label })).toBe(button);
      expect(button.getAttribute('aria-label')).toBe(label);
      expect(button.getAttribute('title')).toBe(label);
      expect(button.textContent?.trim()).toBe('');
      expect(button.classList.contains('size-8')).toBe(true);
      expect(button.className).toContain('focus-visible:');
      expect(button.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
      expect(button.querySelector('svg')?.classList.contains('lucide-refresh-cw')).toBe(true);
      await fireEvent.click(button);
      if (label === check) expect((await screen.findByRole('alert')).textContent).toContain('GitHub rate limit');
    }
    await waitFor(() => expect(button.getAttribute('aria-busy')).toBe('false'));
    expect(screen.getByRole('button', { name: check })).toBe(button);
    expect(checkForUpdates).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole('alert')).toBeNull();
  },
);

test.each([
  ['View GitHub downloads', 'releases'],
  ['Star on GitHub', 'repository'],
] as const)('%s guards opening, displays opener errors and allows retry', async (name, page) => {
  let reject: (cause: Error) => void = () => {};
  const openProjectPage = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((_resolve, fail) => {
          reject = fail;
        }),
    )
    .mockResolvedValueOnce(undefined);
  mount({ openProjectPage, checkForUpdates: vi.fn().mockResolvedValue(latest) });
  await fireEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
  const button = (await screen.findByRole('button', { name })) as HTMLButtonElement;
  await fireEvent.click(button);
  expect(button.disabled).toBe(true);
  expect(button.getAttribute('aria-busy')).toBe('true');
  await fireEvent.click(button);
  expect(openProjectPage).toHaveBeenCalledExactlyOnceWith(page);
  reject(new Error('Browser unavailable'));
  expect((await screen.findByRole('alert')).textContent).toContain('Could not open GitHub in your default browser');
  expect(screen.getByRole('alert').textContent).toContain('Browser unavailable');
  expect(button.disabled).toBe(false);
  await fireEvent.click(button);
  await waitFor(() => expect(button.disabled).toBe(false));
  expect(openProjectPage).toHaveBeenCalledTimes(2);
  expect(screen.queryByRole('alert')).toBeNull();
});

test('late update results after teardown do not bleed into a new settings view', async () => {
  let finish: (value: UpdateCheck) => void = () => {};
  const checkForUpdates = vi.fn().mockImplementation(
    () =>
      new Promise<UpdateCheck>((resolve) => {
        finish = resolve;
      }),
  );
  const first = mount({ checkForUpdates });
  await fireEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
  first.unmount();
  const second = mount();
  finish(latest);
  await tick();
  expect(screen.queryByText('Version 0.2.0 is available.')).toBeNull();
  expect(second.config.checkForUpdates).not.toHaveBeenCalled();
  expect((screen.getByRole('button', { name: 'Check for updates' }) as HTMLButtonElement).disabled).toBe(false);
});

test('late local app details and opener errors after teardown do not bleed into a new view', async () => {
  let finishInfo: (value: AppInfo) => void = () => {};
  let rejectOpen: (cause: Error) => void = () => {};
  const getAppInfo = vi.fn().mockImplementation(
    () =>
      new Promise<AppInfo>((resolve) => {
        finishInfo = resolve;
      }),
  );
  const openProjectPage = vi.fn().mockImplementation(
    () =>
      new Promise<void>((_resolve, reject) => {
        rejectOpen = reject;
      }),
  );
  const first = mount({ getAppInfo, openProjectPage });
  await fireEvent.click(screen.getByRole('button', { name: 'Star on GitHub' }));
  first.unmount();
  mount();
  finishInfo({ version: '9.9.9', platform: 'macos' });
  rejectOpen(new Error('Late opener error'));
  await tick();
  await screen.findByText('Installed version: 0.1.0');
  expect(screen.queryByText('Installed version: 9.9.9')).toBeNull();
  expect(screen.queryByRole('alert')).toBeNull();
});

test('Chinese update feedback, GUI-first guidance and courteous Star copy match the English experience', async () => {
  mount({ preferences: { locale: 'zh', theme: 'dark' }, checkForUpdates: vi.fn().mockResolvedValue(latest) });
  await screen.findByText('已安装版本：0.1.0');
  await fireEvent.click(screen.getByRole('button', { name: '检查更新' }));
  await screen.findByText('发现新版本 0.2.0。');
  const details = screen.getByText('在 macOS 上安装更新').closest('details') as HTMLDetailsElement;
  expect(details.open).toBe(true);
  expect(details.textContent).toContain('如被拦截，先用 系统设置 → 隐私与安全性 → 仍要打开');
  expect(details.textContent).toContain('通常无需下面的终端命令');
  expect(details.textContent).toContain('可选后备方案：若信任的官方下载仍被拦截，请在终端自行运行下方命令');
  expect(details.textContent).toContain('不要用 sudo 或全局关闭 Gatekeeper');
  const command = screen.getByText(quarantineCommand);
  expect(command.tagName).toBe('CODE');
  expect(command.textContent).toBe(quarantineCommand);
  expect(command.classList.contains('select-text')).toBe(true);
  expect(details.querySelector('button')).toBeNull();
  expect(screen.getByText(/如果 MOM 帮到了你/).textContent).toContain('达到 100 星后，我会自费购买');
  expect(screen.getByRole('button', { name: '查看 GitHub 下载' })).toBeTruthy();
  expect(screen.getByRole('button', { name: '在 GitHub 点个 Star' })).toBeTruthy();
});
