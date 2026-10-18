import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { OpenCodeInstallation, OpenCodeServiceStatus, OpenCodeUpdateCheck } from '$src/types/app.js';
import OpenCodeSettings from '../OpenCodeSettings.svelte';

const native = vi.hoisted(() => ({ isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: native.isTauri }));
const installation: OpenCodeInstallation = {
  binaryPath: '/Users/test/.opencode/bin/opencode',
  version: '2.0.0',
  method: 'curl',
  canUpgrade: true,
  upgradeReason: null,
  manualCommand: null,
};
const update: OpenCodeUpdateCheck = { installation, latestVersion: '2.1.0', updateAvailable: true };
const upgraded: OpenCodeInstallation = { ...installation, version: '2.1.0' };
const service: OpenCodeServiceStatus = {
  target: { mode: 'shared', url: null, hasPassword: false },
  connected: true,
  url: 'http://127.0.0.1:4096',
  pid: 4242,
  version: '2.0.7',
  error: null,
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (cause: Error) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}
function mount(overrides: Record<string, unknown> = {}) {
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'macos' }),
    getOpenCodeInstallation: vi.fn().mockResolvedValue(installation),
    getOpenCodeServiceStatus: vi.fn().mockResolvedValue(service),
    setOpenCodeServiceTarget: vi.fn().mockResolvedValue(service),
    checkOpenCodeUpdate: vi.fn().mockResolvedValue(update),
    upgradeOpenCode: vi.fn().mockResolvedValue(upgraded),
    reloadOpencode: vi.fn(),
    checkForUpdates: vi.fn(),
    ...overrides,
  };
  const view = render(
    OpenCodeSettings,
    {},
    {
      wrapper: Harness,
      wrapperProps: { config: config as unknown as ConfigStore },
    },
  );
  return { ...view, config };
}
async function readyCheck(name = 'Check CLI updates') {
  const button = await screen.findByRole('button', { name });
  await waitFor(() => expect((button as HTMLButtonElement).disabled).toBe(false));
  return button;
}
async function openConfirmation() {
  await fireEvent.click(await readyCheck());
  await fireEvent.click(await screen.findByRole('button', { name: 'Upgrade CLI…' }));
  return screen.findByRole('dialog');
}
beforeEach(() => native.isTauri.mockReturnValue(true));

test('native mount reads local details in the background without showing installation metadata', async () => {
  const { config } = mount();
  await readyCheck();
  expect(screen.getByRole('heading', { name: 'CLI updates' })).toBeTruthy();
  expect(screen.queryByRole('region', { name: 'Local CLI installation' })).toBeNull();
  for (const text of ['Local CLI installation', 'CLI version', 'Binary path', 'Installation method', 'curl installer'])
    expect(screen.queryByText(text)).toBeNull();
  expect(screen.queryByText(installation.binaryPath as string)).toBeNull();
  expect(screen.queryByText('2.0.0')).toBeNull();
  expect(config.getAppInfo).toHaveBeenCalledOnce();
  expect(config.getOpenCodeInstallation).toHaveBeenCalledOnce();
  expect(config.getOpenCodeServiceStatus).toHaveBeenCalledOnce();
  expect(screen.getByText('2.0.7')).toBeTruthy();
  expect(screen.getByText('4242')).toBeTruthy();
  expect(screen.getByText(service.url as string)).toBeTruthy();
  expect(config.checkOpenCodeUpdate).not.toHaveBeenCalled();
  expect(config.checkForUpdates).not.toHaveBeenCalled();
  expect(config.upgradeOpenCode).not.toHaveBeenCalled();
  expect(screen.queryByRole('button', { name: 'Upgrade CLI…' })).toBeNull();
});

test.each([
  ['en', 'CLI updates', 'Refresh CLI details', 'Check CLI updates', 'Refresh connection', 'Local CLI installation'],
  ['zh', 'CLI 更新', '刷新 CLI 信息', '检查 CLI 更新', '刷新连接', '本地 CLI 安装'],
] as const)(
  'CLI and connection icons have %s accessible names and tooltips, not visible labels',
  async (locale, heading, refreshLabel, checkLabel, serviceLabel, installationHeading) => {
    mount({ preferences: { locale, theme: 'dark' } });
    await readyCheck(checkLabel);
    const updates = screen.getByRole('region', { name: heading });
    for (const label of [refreshLabel, checkLabel, serviceLabel]) {
      const button = screen.getByRole('button', { name: label });
      expect(button.getAttribute('aria-label')).toBe(label);
      expect(button.getAttribute('title')).toBe(label);
      expect(button.textContent?.trim()).toBe('');
      expect(button.classList.contains('size-8')).toBe(true);
      expect(button.className).toContain('focus-visible:');
      expect(button.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
      expect(button.querySelector('svg')?.classList.contains('animate-spin')).toBe(false);
    }
    expect(within(updates).getByRole('button', { name: refreshLabel })).toBeTruthy();
    expect(
      within(updates)
        .getByRole('button', { name: checkLabel })
        .querySelector('svg')
        ?.classList.contains('lucide-download'),
    ).toBe(true);
    expect(screen.queryByRole('heading', { name: installationHeading })).toBeNull();
    expect(screen.queryByText(installation.binaryPath as string)).toBeNull();
    expect(screen.queryByText('2.0.0')).toBeNull();
  },
);

test.each(['en', 'zh'] as const)(
  'service target form has no %s manual-disable warning in either mode',
  async (locale) => {
    mount({ preferences: { locale, theme: 'dark' } });
    await readyCheck(locale === 'en' ? 'Check CLI updates' : '检查 CLI 更新');
    const warning = /Before changing the target|disable Mini Island|更换目标前|关闭 Mini Island|合盖保护/;
    expect(screen.queryByText(warning)).toBeNull();
    await fireEvent.click(
      screen.getByRole('radio', { name: locale === 'en' ? /Fixed local endpoint/ : /固定本地端点/ }),
    );
    expect(screen.queryByText(warning)).toBeNull();
    expect(screen.getByLabelText('OpenCode backend URL')).toBeTruthy();
    expect(
      screen.getByRole('button', { name: locale === 'en' ? 'Save connection target' : '保存连接目标' }),
    ).toBeTruthy();
  },
);

test('connection target form keeps a vertical gap and a compact, left-aligned save action in both modes', async () => {
  const { config } = mount();
  await readyCheck();
  for (const mode of [/Default Shared Service/, /Fixed local endpoint/]) {
    await fireEvent.click(screen.getByRole('radio', { name: mode }));
    const save = screen.getByRole('button', { name: 'Save connection target' });
    const form = save.closest('form');
    expect(save.parentElement).toBe(form);
    expect(form?.classList.contains('flex')).toBe(true);
    expect(form?.classList.contains('flex-col')).toBe(true);
    expect(form?.classList.contains('gap-3')).toBe(true);
    expect(form?.classList.contains('space-y-3')).toBe(false);
    expect(save.classList.contains('self-start')).toBe(true);
    expect(save.classList.contains('h-7')).toBe(true);
    expect(save.classList.contains('w-full')).toBe(false);
  }
  expect(config.setOpenCodeServiceTarget).not.toHaveBeenCalled();
});

test('browser fallback makes no native calls and offers no installation or update controls', async () => {
  native.isTauri.mockReturnValue(false);
  const { config } = mount();
  await screen.findByText('Available in the desktop app.');
  for (const call of [
    config.getAppInfo,
    config.getOpenCodeInstallation,
    config.getOpenCodeServiceStatus,
    config.setOpenCodeServiceTarget,
    config.checkOpenCodeUpdate,
    config.upgradeOpenCode,
  ])
    expect(call).not.toHaveBeenCalled();
  expect(screen.queryByRole('button')).toBeNull();
});

test('local app-info failure requires explicit retry before inspecting the installation', async () => {
  const getAppInfo = vi.fn().mockRejectedValueOnce(new Error('Native details unavailable')).mockResolvedValueOnce({
    version: '2.0.0',
    platform: 'windows',
  });
  const { config } = mount({ getAppInfo });
  expect((await screen.findByRole('alert')).textContent).toContain('Could not read CLI details');
  expect(screen.queryByText(/Native details unavailable/)).toBeNull();
  expect(config.getOpenCodeInstallation).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: 'Retry CLI details' }));
  await readyCheck();
  expect(getAppInfo).toHaveBeenCalledTimes(2);
  expect(config.getOpenCodeInstallation).toHaveBeenCalledOnce();
  expect(config.checkOpenCodeUpdate).not.toHaveBeenCalled();
});

test('inspection failure disables checks and retry reads locally without an automatic update check', async () => {
  const getOpenCodeInstallation = vi
    .fn()
    .mockRejectedValueOnce(new Error('Read failed'))
    .mockResolvedValueOnce(installation);
  const { config } = mount({ getOpenCodeInstallation });
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('Could not read CLI details');
  expect(alert.textContent).not.toContain('Read failed');
  expect((screen.getByRole('button', { name: 'Check CLI updates' }) as HTMLButtonElement).disabled).toBe(true);
  const retry = within(alert).getByRole('button', { name: 'Retry CLI details' });
  expect(retry.textContent?.trim()).toBe('');
  expect(retry.getAttribute('aria-label')).toBe('Retry CLI details');
  expect(retry.getAttribute('title')).toBe('Retry CLI details');
  expect(retry.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
  await waitFor(() => expect((retry as HTMLButtonElement).disabled).toBe(false));
  expect((screen.getByRole('button', { name: 'Refresh CLI details' }) as HTMLButtonElement).disabled).toBe(false);
  await fireEvent.click(retry);
  await readyCheck();
  expect(getOpenCodeInstallation).toHaveBeenCalledTimes(2);
  expect(config.getAppInfo).toHaveBeenCalledOnce();
  expect(config.checkOpenCodeUpdate).not.toHaveBeenCalled();
});

test('local refresh spins and guards CLI and service controls without checking releases', async () => {
  const operation = deferred<OpenCodeInstallation>();
  const getOpenCodeInstallation = vi.fn().mockResolvedValueOnce(installation).mockReturnValueOnce(operation.promise);
  const { config } = mount({ getOpenCodeInstallation });
  await readyCheck();
  const refresh = screen.getByRole('button', { name: 'Refresh CLI details' });
  await fireEvent.click(refresh);
  expect(refresh.getAttribute('aria-busy')).toBe('true');
  expect(refresh.querySelector('svg')?.classList.contains('animate-spin')).toBe(true);
  expect(refresh.querySelector('svg')?.classList.contains('motion-reduce:animate-none')).toBe(true);
  expect(refresh.textContent?.trim()).toBe('');
  const panel = screen.getByRole('region', { name: 'OpenCode V2' });
  for (const button of within(panel).getAllByRole('button')) expect((button as HTMLButtonElement).disabled).toBe(true);
  for (const input of panel.querySelectorAll('input')) expect(input.matches(':disabled')).toBe(true);
  await fireEvent.click(refresh);
  await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh connection' }));
  expect(getOpenCodeInstallation).toHaveBeenCalledTimes(2);
  expect(config.getOpenCodeServiceStatus).toHaveBeenCalledOnce();
  expect(config.checkOpenCodeUpdate).not.toHaveBeenCalled();
  operation.resolve(installation);
  await readyCheck();
  expect(refresh.getAttribute('aria-busy')).toBe('false');
  expect(refresh.querySelector('svg')?.classList.contains('animate-spin')).toBe(false);
  expect(screen.queryByText(installation.binaryPath as string)).toBeNull();
});

test.each([
  ['not_found', 'not-found', 'OpenCode not found.'],
  ['unsupported', 'manual', 'Cannot upgrade this installation here;'],
  ['bundled', 'bundled', 'Bundled with an app;'],
  ['not_v2', 'npm', 'Not a confirmed OpenCode V2 install.'],
  ['unstable', 'bun', 'Not a stable release;'],
  ['manager_unavailable', 'pnpm', 'Package manager unavailable;'],
  ['ambiguous', 'unknown', 'Install method could not be confirmed;'],
] as const)(
  'unsupported %s hides installation details and adds only a manual-update hint after an explicit check',
  async (reason, method, copy) => {
    const info: OpenCodeInstallation = {
      ...installation,
      method,
      upgradeReason: reason,
      canUpgrade: false,
      manualCommand: 'npm install -g @opencode/cli@2',
      ...(reason === 'not_found' ? { binaryPath: null, version: null } : {}),
    };
    const { config } = mount({
      getOpenCodeInstallation: vi.fn().mockResolvedValue(info),
      checkOpenCodeUpdate: vi.fn().mockResolvedValue({ ...update, installation: info }),
    });
    await readyCheck();
    expect(screen.queryByText((content) => content.startsWith(copy))).toBeNull();
    expect(screen.queryByText(info.manualCommand as string)).toBeNull();
    expect(screen.queryByRole('region', { name: 'Local CLI installation' })).toBeNull();
    expect(config.checkOpenCodeUpdate).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
    await screen.findByText('OpenCode 2.1.0 is available.');
    expect(screen.getByText('Cannot upgrade this installation here; use its original install method.')).toBeTruthy();
    expect(screen.queryByText(info.manualCommand as string)).toBeNull();
    expect(screen.queryByText(installation.binaryPath as string)).toBeNull();
    expect(screen.queryByText('2.0.0')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Upgrade CLI…' })).toBeNull();
    expect(config.upgradeOpenCode).not.toHaveBeenCalled();
  },
);

test('check owns authoritative installation, guards duplicate checks and refresh, and shows no-update feedback', async () => {
  const check = deferred<OpenCodeUpdateCheck>();
  const checkOpenCodeUpdate = vi.fn().mockReturnValue(check.promise);
  const { config } = mount({ checkOpenCodeUpdate });
  await readyCheck();
  const button = screen.getByRole('button', { name: 'Check CLI updates' }) as HTMLButtonElement;
  const refresh = screen.getByRole('button', { name: 'Refresh CLI details' }) as HTMLButtonElement;
  await fireEvent.click(button);
  expect(button.disabled).toBe(true);
  expect(button.getAttribute('aria-busy')).toBe('true');
  expect(button.textContent?.trim()).toBe('');
  expect(button.querySelector('svg')?.classList.contains('animate-spin')).toBe(true);
  expect(button.querySelector('svg')?.classList.contains('motion-reduce:animate-none')).toBe(true);
  expect(refresh.disabled).toBe(true);
  await fireEvent.click(button);
  await fireEvent.click(refresh);
  expect(checkOpenCodeUpdate).toHaveBeenCalledOnce();
  expect(config.getOpenCodeInstallation).toHaveBeenCalledOnce();
  check.resolve({ installation: upgraded, latestVersion: '2.1.0', updateAvailable: false });
  await screen.findByText('No CLI update. Latest stable: 2.1.0.');
  expect(screen.queryByText('2.1.0')).toBeNull();
  expect(button.querySelector('svg')?.classList.contains('lucide-download')).toBe(true);
  expect(screen.getByText('2.0.7')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Upgrade CLI…' })).toBeNull();
});

test('check failure preserves known details and only retries on click', async () => {
  const checkOpenCodeUpdate = vi
    .fn()
    .mockRejectedValueOnce(new Error('Release unavailable'))
    .mockResolvedValueOnce(update);
  mount({ checkOpenCodeUpdate });
  await readyCheck();
  await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
  expect((await screen.findByRole('alert')).textContent).toContain('Could not check CLI updates');
  expect(screen.queryByText(/Release unavailable/)).toBeNull();
  expect(screen.queryByText('2.0.0')).toBeNull();
  expect(checkOpenCodeUpdate).toHaveBeenCalledOnce();
  const retry = screen.getByRole('button', { name: 'Retry CLI update check' });
  expect(retry.textContent?.trim()).toBe('');
  expect(retry.getAttribute('title')).toBe('Retry CLI update check');
  await fireEvent.click(retry);
  await screen.findByText('OpenCode 2.1.0 is available.');
  expect(checkOpenCodeUpdate).toHaveBeenCalledTimes(2);
  expect(screen.queryByRole('alert')).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Upgrade CLI…' }));
  const dialog = await screen.findByRole('dialog');
  expect(within(dialog).getByText(installation.binaryPath as string)).toBeTruthy();
  expect(within(dialog).getByText('2.0.0 → 2.1.0')).toBeTruthy();
});

test('confirmation shows exact checked path and versions; cancellation never upgrades', async () => {
  const checkedInstallation = {
    ...installation,
    binaryPath: '/opt/homebrew/bin/opencode',
    version: '2.0.1',
    method: 'curl',
  };
  const { config } = mount({
    checkOpenCodeUpdate: vi.fn().mockResolvedValue({ ...update, installation: checkedInstallation }),
  });
  await readyCheck();
  await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
  await screen.findByRole('button', { name: 'Upgrade CLI…' });
  expect(screen.queryByText('/opt/homebrew/bin/opencode')).toBeNull();
  expect(screen.queryByText('2.0.1')).toBeNull();
  await fireEvent.click(await screen.findByRole('button', { name: 'Upgrade CLI…' }));
  const dialog = await screen.findByRole('dialog');
  expect(within(dialog).getByText('/opt/homebrew/bin/opencode').classList.contains('select-text')).toBe(true);
  expect(within(dialog).getByText('2.0.1 → 2.1.0')).toBeTruthy();
  expect(dialog.textContent).toContain('Does not update MOM');
  expect(dialog.textContent).toContain('or restart services');
  expect(dialog.textContent).toContain('No sudo');
  expect(config.upgradeOpenCode).not.toHaveBeenCalled();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(config.upgradeOpenCode).not.toHaveBeenCalled();
});

test('upgrade is confirmed once, blocks dismissal and refresh, and reads CLI back without restarting', async () => {
  const operation = deferred<OpenCodeInstallation>();
  const upgradeOpenCode = vi.fn().mockReturnValue(operation.promise);
  const getOpenCodeInstallation = vi.fn().mockResolvedValueOnce(installation).mockResolvedValueOnce(upgraded);
  const { config } = mount({ upgradeOpenCode, getOpenCodeInstallation });
  const dialog = await openConfirmation();
  const confirm = within(dialog).getByRole('button', { name: 'Confirm CLI upgrade' }) as HTMLButtonElement;
  await fireEvent.click(confirm);
  await fireEvent.click(confirm);
  expect(upgradeOpenCode).toHaveBeenCalledExactlyOnceWith(installation.binaryPath, '2.0.0', '2.1.0');
  expect(confirm.disabled).toBe(true);
  expect((within(dialog).getByRole('button', { name: 'Cancel' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.keyDown(dialog, { key: 'Escape' });
  expect(screen.getByRole('dialog')).toBeTruthy();
  expect((screen.getByRole('button', { name: 'Refresh CLI details' }) as HTMLButtonElement).disabled).toBe(true);
  operation.resolve(upgraded);
  await screen.findByText('Upgrade completed. CLI version: 2.1.0.');
  expect(screen.queryByText('2.1.0')).toBeNull();
  expect(screen.getByText('2.0.7')).toBeTruthy();
  expect(getOpenCodeInstallation).toHaveBeenCalledTimes(2);
  expect(config.getOpenCodeServiceStatus).toHaveBeenCalledTimes(2);
  expect(config.checkOpenCodeUpdate).toHaveBeenCalledOnce();
  expect(config.reloadOpencode).not.toHaveBeenCalled();
  expect(screen.getByText(/A running service may still use the old version/)).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Upgrade CLI…' })).toBeNull();
});

test('upgrade failure retains known details and requires refresh and a new check before retry', async () => {
  const upgradeOpenCode = vi.fn().mockRejectedValue(new Error('Binary changed; refresh before retrying'));
  const { config } = mount({ upgradeOpenCode });
  const dialog = await openConfirmation();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Confirm CLI upgrade' }));
  expect((await screen.findByRole('alert')).textContent).toContain('CLI upgrade failed');
  expect(screen.queryByText(/Binary changed/)).toBeNull();
  expect(screen.queryByText('2.0.0')).toBeNull();
  expect(config.getOpenCodeInstallation).toHaveBeenCalledOnce();
  expect(upgradeOpenCode).toHaveBeenCalledOnce();
  expect((screen.getByRole('button', { name: 'Check CLI updates' }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.queryByRole('button', { name: 'Upgrade CLI…' })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Retry CLI details' }));
  await waitFor(() =>
    expect((screen.getByRole('button', { name: 'Check CLI updates' }) as HTMLButtonElement).disabled).toBe(false),
  );
  expect(config.getOpenCodeInstallation).toHaveBeenCalledTimes(2);
  expect(config.checkOpenCodeUpdate).toHaveBeenCalledOnce();
  expect(upgradeOpenCode).toHaveBeenCalledOnce();
  await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Upgrade CLI…' }));
  const retriedDialog = await screen.findByRole('dialog');
  expect(within(retriedDialog).getByText(installation.binaryPath as string)).toBeTruthy();
  expect(within(retriedDialog).getByText('2.0.0 → 2.1.0')).toBeTruthy();
  expect(upgradeOpenCode).toHaveBeenCalledOnce();
});

test('failed upgrade readback is not presented as failed command or success and refresh never repeats the upgrade', async () => {
  const getOpenCodeInstallation = vi
    .fn()
    .mockResolvedValueOnce(installation)
    .mockRejectedValueOnce(new Error('Readback unavailable'))
    .mockResolvedValueOnce(upgraded);
  const { config } = mount({ getOpenCodeInstallation });
  const dialog = await openConfirmation();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Confirm CLI upgrade' }));
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('Upgrade command finished');
  expect(alert.textContent).not.toContain('Readback unavailable');
  expect(screen.queryByText('2.0.0')).toBeNull();
  expect(screen.queryByText(/Upgrade completed. CLI version/)).toBeNull();
  expect(config.upgradeOpenCode).toHaveBeenCalledOnce();
  await fireEvent.click(screen.getByRole('button', { name: 'Retry CLI details' }));
  await readyCheck();
  expect(screen.queryByText('2.1.0')).toBeNull();
  expect(config.upgradeOpenCode).toHaveBeenCalledOnce();
  expect(getOpenCodeInstallation).toHaveBeenCalledTimes(3);
});

test('teardown before app-info completion does not inspect or bleed into a new view', async () => {
  const info = deferred<{ version: string; platform: string }>();
  const first = mount({ getAppInfo: vi.fn().mockReturnValue(info.promise) });
  first.unmount();
  mount();
  info.resolve({ version: '2.0.0', platform: 'macos' });
  await readyCheck();
  expect(first.config.getOpenCodeInstallation).not.toHaveBeenCalled();
  expect(first.config.getOpenCodeServiceStatus).not.toHaveBeenCalled();
});

test.each(['inspect', 'check', 'upgrade'] as const)(
  'late %s completion after teardown has no readback or visible effect',
  async (kind) => {
    const operation = deferred<OpenCodeInstallation | OpenCodeUpdateCheck>();
    const call = vi.fn().mockReturnValue(operation.promise);
    const first = mount({
      ...(kind === 'inspect' ? { getOpenCodeInstallation: call } : {}),
      ...(kind === 'check' ? { checkOpenCodeUpdate: call } : {}),
      ...(kind === 'upgrade' ? { upgradeOpenCode: call } : {}),
    });
    if (kind === 'check') {
      await readyCheck();
      await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
    } else if (kind === 'upgrade') {
      const dialog = await openConfirmation();
      await fireEvent.click(within(dialog).getByRole('button', { name: 'Confirm CLI upgrade' }));
    }
    await waitFor(() => expect(call).toHaveBeenCalledOnce());
    first.unmount();
    const second = mount();
    operation.resolve(kind === 'check' ? update : upgraded);
    await tick();
    await readyCheck();
    expect(first.config.getOpenCodeInstallation).toHaveBeenCalledOnce();
    expect(second.config.checkOpenCodeUpdate).not.toHaveBeenCalled();
    expect(screen.queryByText('OpenCode 2.1.0 is available.')).toBeNull();
    expect(screen.queryByText('2.1.0')).toBeNull();
  },
);

test('Chinese confirmation includes the exact path and local-only safety copy', async () => {
  const { config } = mount({ preferences: { locale: 'zh', theme: 'dark' } });
  await readyCheck('检查 CLI 更新');
  await fireEvent.click(screen.getByRole('button', { name: '检查 CLI 更新' }));
  await fireEvent.click(await screen.findByRole('button', { name: '升级 CLI…' }));
  const dialog = await screen.findByRole('dialog');
  expect(within(dialog).getByText(installation.binaryPath as string)).toBeTruthy();
  expect(within(dialog).getByText('2.0.0 → 2.1.0')).toBeTruthy();
  expect(dialog.textContent).toContain('或重启服务');
  expect(dialog.textContent).toContain('不使用 sudo');
  await fireEvent.click(within(dialog).getByRole('button', { name: '取消' }));
  expect(config.upgradeOpenCode).not.toHaveBeenCalled();
});

async function explicitDraft(url = 'http://localhost:5096', password = '') {
  await readyCheck();
  await fireEvent.click(screen.getByRole('radio', { name: /Fixed local endpoint/ }));
  await fireEvent.input(screen.getByLabelText('OpenCode backend URL'), { target: { value: url } });
  if (password)
    await fireEvent.input(screen.getByLabelText('Service password (optional)'), { target: { value: password } });
}

test('installation and service reads fail independently; a missing shared target still allows an explicit save', async () => {
  const getOpenCodeInstallation = vi.fn().mockRejectedValue(new Error('CLI missing'));
  const getOpenCodeServiceStatus = vi.fn().mockRejectedValue(new Error('secret backend error'));
  const { config } = mount({ getOpenCodeInstallation, getOpenCodeServiceStatus });
  await screen.findByText('Unknown', { selector: 'span' });
  expect(getOpenCodeServiceStatus).toHaveBeenCalledOnce();
  expect(screen.queryByText(/secret backend error/)).toBeNull();
  expect(screen.queryByText('Idle')).toBeNull();
  await fireEvent.click(screen.getByRole('radio', { name: /Fixed local endpoint/ }));
  await fireEvent.input(screen.getByLabelText('OpenCode backend URL'), { target: { value: 'http://127.0.0.1:5096' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save connection target' }));
  expect(config.setOpenCodeServiceTarget).toHaveBeenCalledExactlyOnceWith('http://127.0.0.1:5096');
  expect((screen.getByRole('button', { name: 'Check CLI updates' }) as HTMLButtonElement).disabled).toBe(true);
});

test('service failure leaves installation checks usable and shows no stale PID or version', async () => {
  const getOpenCodeServiceStatus = vi
    .fn()
    .mockResolvedValueOnce(service)
    .mockRejectedValueOnce(new Error('password=do-not-render'));
  const { config } = mount({ getOpenCodeServiceStatus });
  await screen.findByText('4242');
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh connection' }));
  await screen.findByText('Unknown', { selector: 'span' });
  expect(screen.queryByText('4242')).toBeNull();
  expect(screen.queryByText('2.0.7')).toBeNull();
  expect(screen.queryByText(/do-not-render/)).toBeNull();
  expect((screen.getByRole('button', { name: 'Check CLI updates' }) as HTMLButtonElement).disabled).toBe(false);
  expect(config.checkOpenCodeUpdate).not.toHaveBeenCalled();
});

test('read-only refresh keeps URL, password and clear-password drafts and tests only the saved target', async () => {
  const getOpenCodeServiceStatus = vi.fn().mockResolvedValue({
    ...service,
    target: { mode: 'explicit', url: service.url, hasPassword: true },
  });
  const { config } = mount({ getOpenCodeServiceStatus });
  await screen.findByLabelText('OpenCode backend URL');
  await explicitDraft('http://127.0.0.1:6096', 'local-secret');
  const password = screen.getByLabelText('Service password (optional)') as HTMLInputElement;
  expect(password.type).toBe('password');
  expect(password.autocomplete).toBe('new-password');
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh connection' }));
  await waitFor(() => expect(getOpenCodeServiceStatus).toHaveBeenCalledTimes(2));
  expect((screen.getByLabelText('OpenCode backend URL') as HTMLInputElement).value).toBe('http://127.0.0.1:6096');
  expect(password.value).toBe('local-secret');
  expect(config.setOpenCodeServiceTarget).not.toHaveBeenCalled();
  expect(screen.queryByText('local-secret')).toBeNull();
  await fireEvent.click(screen.getByLabelText('Clear the saved password when saving'));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh connection' }));
  await waitFor(() => expect(getOpenCodeServiceStatus).toHaveBeenCalledTimes(3));
  expect((screen.getByLabelText('Clear the saved password when saving') as HTMLInputElement).checked).toBe(true);
});

test('failed save preserves confirmed target and local secret draft without displaying backend payloads', async () => {
  const setOpenCodeServiceTarget = vi.fn().mockRejectedValue({
    code: 'configuration_failed',
    message: 'local-secret failed',
    detail: { password: 'local-secret' },
  });
  const { config } = mount({ setOpenCodeServiceTarget });
  await explicitDraft('http://127.0.0.1:5096', 'local-secret');
  await fireEvent.click(screen.getByRole('button', { name: 'Save connection target' }));
  await screen.findByText(/Could not save; the saved target is unchanged and your draft is kept/);
  expect(setOpenCodeServiceTarget).toHaveBeenCalledExactlyOnceWith('http://127.0.0.1:5096', 'local-secret');
  expect((screen.getByLabelText('Service password (optional)') as HTMLInputElement).value).toBe('local-secret');
  expect((screen.getByLabelText('OpenCode backend URL') as HTMLInputElement).value).toBe('http://127.0.0.1:5096');
  expect(screen.queryByText(/local-secret/)).toBeNull();
  expect(screen.getByText('4242')).toBeTruthy();
  expect(screen.getByText(service.url as string)).toBeTruthy();
  expect(config.getOpenCodeServiceStatus).toHaveBeenCalledOnce();
});

test.each([
  ['en', 'configuration_failed', 'Could not save;', 'the saved target is unchanged and your draft is kept'],
  ['en', 'busy', 'local-service operation is running', 'wait, then retry'],
  ['en', 'validation_failed', 'direct V2 loopback URL, port and password', 'then retry'],
  ['zh', 'configuration_failed', '无法保存', '已保存目标和草稿均未改变'],
  ['zh', 'busy', '本地服务操作进行中', '请稍后重试'],
  ['zh', 'validation_failed', '直连 V2 loopback URL、端口和密码', '后重试'],
] as const)(
  'save %s/%s gives only a safe action hint and allows an explicit retry',
  async (locale, code, hint, action) => {
    const setOpenCodeServiceTarget = vi
      .fn()
      .mockRejectedValueOnce({ code, message: 'private-password', detail: { password: 'private-password' } })
      .mockResolvedValueOnce({
        ...service,
        target: { mode: 'explicit', url: 'http://[::1]:5096', hasPassword: true },
      });
    mount({ preferences: { locale, theme: 'dark' }, setOpenCodeServiceTarget });
    await readyCheck(locale === 'en' ? 'Check CLI updates' : '检查 CLI 更新');
    await fireEvent.click(
      screen.getByRole('radio', { name: locale === 'en' ? /Fixed local endpoint/ : /固定本地端点/ }),
    );
    const url = screen.getByLabelText('OpenCode backend URL');
    const password = screen.getByLabelText(locale === 'en' ? 'Service password (optional)' : '服务密码（可选）');
    await fireEvent.input(url, { target: { value: 'http://[::1]:5096' } });
    await fireEvent.input(password, { target: { value: 'private-password' } });
    const save = screen.getByRole('button', { name: locale === 'en' ? 'Save connection target' : '保存连接目标' });
    await fireEvent.click(save);
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain(hint);
    expect(alert.textContent).toContain(action);
    expect(alert.textContent).not.toContain('private-password');
    expect(alert.textContent).not.toMatch(/Mini Island|lid protection|power|合盖保护|电源|关闭/);
    expect((url as HTMLInputElement).value).toBe('http://[::1]:5096');
    expect((password as HTMLInputElement).value).toBe('private-password');
    expect(screen.getByText('4242')).toBeTruthy();
    expect(setOpenCodeServiceTarget).toHaveBeenCalledOnce();
    await fireEvent.click(save);
    await waitFor(() => expect((password as HTMLInputElement).value).toBe(''));
    expect(setOpenCodeServiceTarget).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole('alert')).toBeNull();
  },
);

test('a status error cannot advertise a stale connected backend and never renders its private text', async () => {
  mount({ getOpenCodeServiceStatus: vi.fn().mockResolvedValue({ ...service, error: 'private-password' }) });
  await screen.findByText('Disconnected');
  expect(screen.queryByText('Connected')).toBeNull();
  expect(screen.queryByText('4242')).toBeNull();
  expect(screen.queryByText('2.0.7')).toBeNull();
  expect(screen.queryByText('private-password')).toBeNull();
  expect((await screen.findByRole('alert')).textContent).toContain('Could not confirm the connection');
});

test('offline save commits the returned normalized target, clears password and never starts or falls back', async () => {
  const offline: OpenCodeServiceStatus = {
    target: { mode: 'explicit', url: 'http://127.0.0.1:5096', hasPassword: true },
    connected: false,
    url: null,
    pid: null,
    version: null,
    error: 'secret inaccessible service',
  };
  const { config } = mount({ setOpenCodeServiceTarget: vi.fn().mockResolvedValue(offline) });
  await explicitDraft('http://localhost:5096', 'new-secret');
  await fireEvent.click(screen.getByRole('button', { name: 'Save connection target' }));
  await screen.findByText(/Target saved, but the backend is offline/);
  expect(screen.getByText('Disconnected')).toBeTruthy();
  expect(screen.queryByText('4242')).toBeNull();
  expect(screen.queryByText(/secret inaccessible/)).toBeNull();
  expect((screen.getByLabelText('OpenCode backend URL') as HTMLInputElement).value).toBe(offline.target.url);
  expect((screen.getByLabelText('Service password (optional)') as HTMLInputElement).value).toBe('');
  expect(screen.getByText(/A password is saved/)).toBeTruthy();
  expect(config.getOpenCodeServiceStatus).toHaveBeenCalledOnce();
  expect(config.reloadOpencode).not.toHaveBeenCalled();
  expect(config.checkOpenCodeUpdate).not.toHaveBeenCalled();
});

test('saved credential can be explicitly cleared; blank password omits the argument and Shared saves null', async () => {
  const explicit: OpenCodeServiceStatus = {
    ...service,
    target: { mode: 'explicit', url: service.url, hasPassword: true },
  };
  const setOpenCodeServiceTarget = vi
    .fn()
    .mockResolvedValueOnce(explicit)
    .mockResolvedValueOnce({ ...explicit, target: { ...explicit.target, hasPassword: false } })
    .mockResolvedValueOnce(service);
  mount({ getOpenCodeServiceStatus: vi.fn().mockResolvedValue(explicit), setOpenCodeServiceTarget });
  await screen.findByLabelText('Service password (optional)');
  await fireEvent.click(screen.getByRole('button', { name: 'Save connection target' }));
  await screen.findByText('Target saved; connected to the backend.');
  expect(setOpenCodeServiceTarget.mock.calls[0]).toEqual([service.url]);
  await fireEvent.click(screen.getByLabelText('Clear the saved password when saving'));
  await fireEvent.click(screen.getByRole('button', { name: 'Save connection target' }));
  await waitFor(() => expect(screen.queryByLabelText('Clear the saved password when saving')).toBeNull());
  expect(setOpenCodeServiceTarget.mock.calls[1]).toEqual([service.url, '']);
  await fireEvent.click(screen.getByRole('radio', { name: /Default Shared Service/ }));
  await fireEvent.click(screen.getByRole('button', { name: 'Save connection target' }));
  await waitFor(() => expect(setOpenCodeServiceTarget.mock.calls[2]).toEqual([null]));
});

test('connection save and refresh pending guards prevent duplicate calls and premature readback', async () => {
  const operation = deferred<OpenCodeServiceStatus>();
  const setOpenCodeServiceTarget = vi.fn().mockReturnValue(operation.promise);
  const { config } = mount({ setOpenCodeServiceTarget });
  await explicitDraft();
  const save = screen.getByRole('button', { name: 'Save connection target' });
  await fireEvent.click(save);
  const refresh = screen.getByRole('button', { name: 'Refresh connection' });
  expect(refresh.textContent?.trim()).toBe('');
  expect(refresh.getAttribute('aria-busy')).toBe('false');
  expect(refresh.querySelector('svg')?.classList.contains('animate-spin')).toBe(false);
  await fireEvent.click(save);
  await fireEvent.click(refresh);
  expect(setOpenCodeServiceTarget).toHaveBeenCalledOnce();
  expect(config.getOpenCodeServiceStatus).toHaveBeenCalledOnce();
  expect(screen.getByText('4242')).toBeTruthy();
  expect((screen.getByLabelText('OpenCode backend URL') as HTMLInputElement).disabled).toBe(true);
  operation.resolve({
    ...service,
    pid: 5151,
    target: { mode: 'explicit', url: 'http://127.0.0.1:5096', hasPassword: false },
  });
  await screen.findByText('5151');
  expect(screen.queryByText('4242')).toBeNull();
});

test('all CLI and connection controls stay guarded while saving; no upgrade confirmation can open', async () => {
  const operation = deferred<OpenCodeServiceStatus>();
  const { config } = mount({ setOpenCodeServiceTarget: vi.fn().mockReturnValue(operation.promise) });
  await explicitDraft('http://localhost:5096', 'local-secret');
  await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
  const upgrade = await screen.findByRole('button', { name: 'Upgrade CLI…' });
  const refresh = screen.getByRole('button', { name: 'Refresh CLI details' });
  const check = screen.getByRole('button', { name: 'Check CLI updates' });
  const form = screen.getByLabelText('OpenCode backend URL').closest('form') as HTMLFormElement;
  await fireEvent.click(screen.getByRole('button', { name: 'Save connection target' }));
  const panel = screen.getByRole('region', { name: 'OpenCode V2' });
  for (const button of within(panel).getAllByRole('button')) expect((button as HTMLButtonElement).disabled).toBe(true);
  for (const input of panel.querySelectorAll('input')) expect(input.matches(':disabled')).toBe(true);
  expect(panel.getAttribute('aria-busy')).toBe('true');
  await fireEvent.click(upgrade);
  await fireEvent.click(refresh);
  await fireEvent.click(check);
  await fireEvent.submit(form);
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(config.upgradeOpenCode).not.toHaveBeenCalled();
  expect(config.getOpenCodeInstallation).toHaveBeenCalledOnce();
  expect(config.checkOpenCodeUpdate).toHaveBeenCalledOnce();
  expect(config.setOpenCodeServiceTarget).toHaveBeenCalledOnce();
  operation.resolve(service);
  await screen.findByRole('button', { name: 'Upgrade CLI…' });
  expect((refresh as HTMLButtonElement).disabled).toBe(false);
  expect((check as HTMLButtonElement).disabled).toBe(false);
  await fireEvent.click(screen.getByRole('button', { name: 'Upgrade CLI…' }));
  const dialog = await screen.findByRole('dialog');
  expect(within(dialog).getByText('2.0.0 → 2.1.0')).toBeTruthy();
  for (const button of within(panel).getAllByRole('button')) expect((button as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.submit(form);
  expect(config.setOpenCodeServiceTarget).toHaveBeenCalledOnce();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
});

test('a CLI check disables connection edits and save, without automatically reading or changing the target', async () => {
  const operation = deferred<OpenCodeUpdateCheck>();
  const { config } = mount({ checkOpenCodeUpdate: vi.fn().mockReturnValue(operation.promise) });
  await explicitDraft();
  const form = screen.getByLabelText('OpenCode backend URL').closest('form') as HTMLFormElement;
  await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
  const panel = screen.getByRole('region', { name: 'OpenCode V2' });
  for (const button of within(panel).getAllByRole('button')) expect((button as HTMLButtonElement).disabled).toBe(true);
  for (const input of panel.querySelectorAll('input')) expect(input.matches(':disabled')).toBe(true);
  await fireEvent.submit(form);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh connection' }));
  expect(config.setOpenCodeServiceTarget).not.toHaveBeenCalled();
  expect(config.getOpenCodeServiceStatus).toHaveBeenCalledOnce();
  operation.resolve(update);
  await screen.findByRole('button', { name: 'Upgrade CLI…' });
  expect((screen.getByRole('button', { name: 'Save connection target' }) as HTMLButtonElement).disabled).toBe(false);
});

test('connection refresh blocks CLI actions until readback completes, while preserving the draft', async () => {
  const operation = deferred<OpenCodeServiceStatus>();
  const getOpenCodeServiceStatus = vi.fn().mockResolvedValueOnce(service).mockReturnValueOnce(operation.promise);
  const { config } = mount({ getOpenCodeServiceStatus });
  await explicitDraft('http://[::1]:5096', 'local-secret');
  await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
  const upgrade = await screen.findByRole('button', { name: 'Upgrade CLI…' });
  const refresh = screen.getByRole('button', { name: 'Refresh connection' });
  await fireEvent.click(refresh);
  expect(refresh.getAttribute('aria-busy')).toBe('true');
  expect(refresh.querySelector('svg')?.classList.contains('animate-spin')).toBe(true);
  expect(refresh.querySelector('svg')?.classList.contains('motion-reduce:animate-none')).toBe(true);
  expect(refresh.textContent?.trim()).toBe('');
  const panel = screen.getByRole('region', { name: 'OpenCode V2' });
  for (const button of within(panel).getAllByRole('button')) expect((button as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.click(upgrade);
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(config.upgradeOpenCode).not.toHaveBeenCalled();
  operation.resolve(service);
  await screen.findByRole('button', { name: 'Upgrade CLI…' });
  expect(refresh.getAttribute('aria-busy')).toBe('false');
  expect(refresh.querySelector('svg')?.classList.contains('animate-spin')).toBe(false);
  expect((screen.getByLabelText('OpenCode backend URL') as HTMLInputElement).value).toBe('http://[::1]:5096');
  expect((screen.getByLabelText('Service password (optional)') as HTMLInputElement).value).toBe('local-secret');
});

test.each(['read', 'save'] as const)(
  'connection %s completion after teardown has no visible or readback effect',
  async (kind) => {
    const operation = deferred<OpenCodeServiceStatus>();
    const call = vi.fn().mockReturnValue(operation.promise);
    const first = mount(kind === 'read' ? { getOpenCodeServiceStatus: call } : { setOpenCodeServiceTarget: call });
    if (kind === 'save') {
      await explicitDraft();
      await fireEvent.click(screen.getByRole('button', { name: 'Save connection target' }));
    }
    await waitFor(() => expect(call).toHaveBeenCalledOnce());
    first.unmount();
    const second = mount();
    operation.resolve({ ...service, pid: 9999 });
    await tick();
    await screen.findByText('4242');
    expect(screen.queryByText('9999')).toBeNull();
    expect(second.config.setOpenCodeServiceTarget).not.toHaveBeenCalled();
    expect(first.config.getOpenCodeServiceStatus).toHaveBeenCalledOnce();
  },
);
