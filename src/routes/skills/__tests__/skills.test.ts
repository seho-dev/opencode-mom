import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { SkillEntry } from '$src/types/skills.js';
import SkillList from '../+page.svelte';
import SkillDetail from '../[id]/+page.svelte';
import SkillNew from '../new/+page.svelte';
import SkillForm from '../SkillForm.svelte';

vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }));

vi.mock('$app/state', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  let params = { id: 'docs/a b' };
  let url = new URL('http://localhost/skills');
  return {
    page: {
      get url() {
        track();
        return url;
      },
      set url(value) {
        url = value;
        notify();
      },
      get params() {
        track();
        return params;
      },
      set params(value) {
        params = value;
        notify();
      },
    },
  };
});
const skill: SkillEntry = {
  id: 'docs/a b',
  kind: 'local',
  name: 'Read docs',
  description: 'A full description',
  autoinvoke: true,
  path: '/global/skills/docs/SKILL.md',
  source: 'global',
  content:
    '# Heading\n\n<script>window.unsafe = true</script>\n<img src=x onerror="alert(1)">\n```sh\necho read-only\n```\n' +
    'Long body. '.repeat(500),
};
function makeConfig(overrides: Record<string, unknown> = {}) {
  return {
    preferences: { locale: 'en', theme: 'light' },
    listSkills: vi.fn().mockResolvedValue({ data: [skill], diagnostics: ['Skipped unreadable skill'] }),
    getSkill: vi.fn().mockResolvedValue(skill),
    reloadOpencode: vi.fn(),
    ...overrides,
  } as unknown as ConfigStore;
}
beforeEach(() => {
  page.params = { id: skill.id };
  page.url = new URL('http://localhost/skills');
  vi.mocked(goto).mockClear();
});

test('list omits the scope hint and shows diagnostics, encoded detail link, search, and retry', async () => {
  const listSkills = vi
    .fn()
    .mockRejectedValueOnce(new Error('read denied'))
    .mockResolvedValueOnce({ data: [skill], diagnostics: ['Skipped unreadable skill'] });
  render(SkillList, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ listSkills }) } });
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('read denied');
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  const link = await screen.findByRole('link', { name: skill.name });
  expect(link.getAttribute('href')).toBe('/skills/docs%2Fa%20b');
  expect(screen.getByText('Skipped unreadable skill')).toBeTruthy();
  expect(
    screen.queryByText(/Skills from user-global directories|Project, built-in and plugin skills are not included/),
  ).toBeNull();
  expect(screen.getByRole('link', { name: 'New skill' }).getAttribute('href')).toBe('/skills/new');
  expect(screen.getByRole('link', { name: `Edit ${skill.name}` }).getAttribute('href')).toBe(
    '/skills/docs%2Fa%20b?edit=1',
  );
  await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'not-here' } });
  expect(screen.getByText('No matches')).toBeTruthy();
  expect(screen.queryByRole('button', { name: /delete|edit|new/i })).toBeNull();
});

test('detail displays full Markdown safely as plain text with metadata', async () => {
  const { container } = render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig() } });
  await screen.findByRole('heading', { name: skill.name });
  expect(container.querySelector('pre')?.textContent).toBe(skill.content);
  expect(container.querySelector('script')).toBeNull();
  expect(container.querySelector('img')).toBeNull();
  expect(screen.getByText(skill.path)).toBeTruthy();
  expect(screen.getByText('Enabled')).toBeTruthy();
  expect(
    screen.queryByText(/Skills from user-global directories|Project, built-in and plugin skills are not included/),
  ).toBeNull();
});

test('detail explains empty content and errors; direct ID change cannot show old content', async () => {
  let resolve: (value: SkillEntry) => void = () => {};
  const getSkill = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<SkillEntry>((done) => {
          resolve = done;
        }),
    )
    .mockRejectedValueOnce({ code: 'not_found', message: 'missing entry' })
    .mockRejectedValueOnce(new Error('read failed'))
    .mockResolvedValueOnce({ ...skill, id: 'new', name: 'New skill', content: '' });
  render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ getSkill }) } });
  await waitFor(() => expect(getSkill).toHaveBeenCalledOnce());
  page.params = { id: 'new' };
  let alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('not found in user-global directories');
  resolve(skill);
  await Promise.resolve();
  expect(screen.queryByRole('heading', { name: skill.name })).toBeNull();
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  alert = await screen.findByRole('alert');
  await waitFor(() => expect(alert.textContent).toContain('read failed'));
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await screen.findByText('This skill has no Markdown body.');
  expect(screen.getByRole('heading', { name: 'New skill' })).toBeTruthy();
  expect(getSkill).toHaveBeenLastCalledWith('new');
});

test('list explains empty user-global skills', async () => {
  render(
    SkillList,
    {},
    {
      wrapper: Harness,
      wrapperProps: { config: makeConfig({ listSkills: vi.fn().mockResolvedValue({ data: [], diagnostics: [] }) }) },
    },
  );
  await screen.findByText('No skills in user-global directories.');
});

test('list uses backend kind badges and only Local rows offer Edit', async () => {
  const remote = { ...skill, id: 'remote', name: 'Remote skill', kind: 'remote', path: '/cached/SKILL.md' };
  render(
    SkillList,
    {},
    {
      wrapper: Harness,
      wrapperProps: {
        config: makeConfig({ listSkills: vi.fn().mockResolvedValue({ data: [skill, remote], diagnostics: [] }) }),
      },
    },
  );
  await screen.findByRole('link', { name: remote.name });
  expect(screen.getByText('Local')).toBeTruthy();
  expect(screen.getByText('Remote')).toBeTruthy();
  expect(screen.getByRole('link', { name: `View ${remote.name}` }).getAttribute('href')).toBe('/skills/remote');
  expect(screen.queryByRole('link', { name: `Edit ${remote.name}` })).toBeNull();
});

test('creates Local skill with exact full Markdown document and ID payload; errors keep draft', async () => {
  const createSkill = vi
    .fn()
    .mockRejectedValueOnce({ code: 'validation_failed', message: 'Frontmatter is invalid' })
    .mockResolvedValueOnce({ ...skill, id: 'new-skill' });
  const config = makeConfig({ createSkill });
  render(SkillNew, {}, { wrapper: Harness, wrapperProps: { config } });
  const input = screen.getByLabelText('Full Markdown document') as HTMLTextAreaElement;
  expect(input.value).toBe('---\nname: ""\ndescription: ""\n---\n\n');
  expect(input.value).not.toMatch(/metadata|autoinvoke|disable-model-invocation/);
  const content =
    '---\nname: Custom display name\ndescription: Full description\nmetadata:\n  opencode/autoinvoke: true\nextra: retained\n---\n\n# Full body\n';
  await fireEvent.input(screen.getByLabelText('ID'), { target: { value: 'new-skill' } });
  await fireEvent.input(input, { target: { value: content } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  await screen.findByText(/Frontmatter is invalid/);
  expect(input.value).toBe(content);
  expect((screen.getByLabelText('ID') as HTMLInputElement).value).toBe('new-skill');
  await fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/skills/new-skill?saved=1'));
  expect(createSkill).toHaveBeenNthCalledWith(2, { id: 'new-skill', content });
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});

test('Local edit keeps expected snapshot and draft after conflict; cancel restores saved document', async () => {
  const updateSkill = vi.fn().mockRejectedValue({ code: 'configuration_failed', message: 'File changed externally' });
  render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ updateSkill }) } });
  await screen.findByRole('heading', { name: skill.name });
  await fireEvent.click(screen.getByRole('button', { name: 'Edit', exact: true }));
  const input = screen.getByLabelText('Full Markdown document') as HTMLTextAreaElement;
  expect(input.value).toBe(skill.content);
  expect((screen.getByLabelText('ID') as HTMLInputElement).disabled).toBe(true);
  const content = '# Edited\n\n<script>not executed</script>\n';
  await fireEvent.input(input, { target: { value: content } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  await screen.findByText(/File changed externally/);
  expect(input.value).toBe(content);
  expect(updateSkill).toHaveBeenCalledExactlyOnceWith({
    id: skill.id,
    content,
    expectedContent: skill.content,
    expectedPath: skill.path,
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(document.querySelector('pre')?.textContent).toBe(skill.content);
  await fireEvent.click(screen.getByRole('button', { name: 'Edit', exact: true }));
  expect((screen.getByLabelText('Full Markdown document') as HTMLTextAreaElement).value).toBe(skill.content);
});

test('Local save disables pending controls and returns successful full document to safe read-only view', async () => {
  let resolve: (result: SkillEntry) => void = () => {};
  const updateSkill = vi.fn().mockImplementation(
    () =>
      new Promise<SkillEntry>((done) => {
        resolve = done;
      }),
  );
  const config = makeConfig({ updateSkill });
  page.url = new URL('http://localhost/skills/docs?edit=1');
  render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByLabelText('Full Markdown document');
  await fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  expect((screen.getByRole('button', { name: 'Saving…' }) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole('button', { name: 'Cancel' }) as HTMLButtonElement).disabled).toBe(true);
  resolve({ ...skill, name: 'Saved name' });
  await screen.findByRole('heading', { name: 'Saved name' });
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(document.querySelector('pre')?.textContent).toBe(skill.content);
  expect(screen.getByText('Saved to global configuration. Reload OpenCode manually to apply.')).toBeTruthy();
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});

test('Remote detail rejects edit query and forged form entry without update IPC', async () => {
  const remote: SkillEntry = { ...skill, kind: 'remote', path: '/cached/SKILL.md' };
  const updateSkill = vi.fn();
  const config = makeConfig({ getSkill: vi.fn().mockResolvedValue(remote), updateSkill });
  page.url = new URL('http://localhost/skills/docs?edit=1');
  const mounted = render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByText(/This Remote skill is read-only/);
  expect(screen.queryByRole('button', { name: 'Edit', exact: true })).toBeNull();
  expect(screen.queryByRole('textbox')).toBeNull();
  mounted.unmount();
  render(SkillForm, { source: remote }, { wrapper: Harness, wrapperProps: { config } });
  expect(screen.queryByRole('button', { name: 'Save' })).toBeNull();
  expect(updateSkill).not.toHaveBeenCalled();
});

test('Skill route changes cannot apply stale pending save to a different detail', async () => {
  let resolve: (result: SkillEntry) => void = () => {};
  const updateSkill = vi.fn().mockImplementation(
    () =>
      new Promise<SkillEntry>((done) => {
        resolve = done;
      }),
  );
  const getSkill = vi
    .fn()
    .mockResolvedValueOnce(skill)
    .mockResolvedValueOnce({ ...skill, id: 'other', name: 'Other skill' });
  render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ updateSkill, getSkill }) } });
  await screen.findByRole('heading', { name: skill.name });
  await fireEvent.click(screen.getByRole('button', { name: 'Edit', exact: true }));
  await fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  page.params = { id: 'other' };
  await screen.findByRole('heading', { name: 'Other skill' });
  resolve(skill);
  await waitFor(() => expect(screen.queryByRole('heading', { name: skill.name })).toBeNull());
  expect(screen.queryByText('Saved to global configuration. Reload OpenCode manually to apply.')).toBeNull();
});

test('Skill Edit query on same ID opens Local editor; unmount ignores late creation navigation', async () => {
  const mounted = render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig() } });
  await screen.findByRole('heading', { name: skill.name });
  page.url = new URL('http://localhost/skills/docs?edit=1');
  await screen.findByLabelText('Full Markdown document');
  mounted.unmount();
  let resolve: (result: SkillEntry) => void = () => {};
  const createSkill = vi.fn().mockImplementation(
    () =>
      new Promise<SkillEntry>((done) => {
        resolve = done;
      }),
  );
  const creating = render(SkillNew, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ createSkill }) } });
  await fireEvent.input(screen.getByLabelText('ID'), { target: { value: 'docs' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  creating.unmount();
  resolve(skill);
  await Promise.resolve();
  expect(goto).not.toHaveBeenCalled();
});

test('editing saved explicit autoinvoke false preserves the full document and original snapshot', async () => {
  const content =
    '---\nname: Manual only\ndescription: Use only when requested\nmetadata:\n  opencode/autoinvoke: false\n---\n\n# Saved body\n';
  const manual: SkillEntry = { ...skill, autoinvoke: false, content };
  const updateSkill = vi.fn().mockResolvedValue(manual);
  render(SkillForm, { source: manual }, { wrapper: Harness, wrapperProps: { config: makeConfig({ updateSkill }) } });
  const input = screen.getByLabelText('Full Markdown document') as HTMLTextAreaElement;
  expect(input.value).toBe(content);
  await fireEvent.click(screen.getByRole('button', { name: 'Save', exact: true }));
  await waitFor(() =>
    expect(updateSkill).toHaveBeenCalledExactlyOnceWith({
      id: skill.id,
      content,
      expectedContent: content,
      expectedPath: skill.path,
    }),
  );
  expect(manual.content).toBe(content);
});

test('Skill Refresh only rereads its own list and replaces diagnostics without runtime reload', async () => {
  const listSkills = vi
    .fn()
    .mockResolvedValueOnce({ data: [skill], diagnostics: ['Old diagnostic'] })
    .mockResolvedValueOnce({ data: [], diagnostics: ['New diagnostic'] });
  const otherRead = vi.fn();
  const config = makeConfig({
    listSkills,
    listMcps: otherRead,
    refreshAll: otherRead,
    createSkill: otherRead,
    updateSkill: otherRead,
  });
  render(SkillList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: skill.name });
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await screen.findByText('No skills in user-global directories.');
  expect(screen.queryByText('Old diagnostic')).toBeNull();
  expect(screen.getByText('New diagnostic')).toBeTruthy();
  expect(listSkills).toHaveBeenCalledTimes(2);
  expect(otherRead).not.toHaveBeenCalled();
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});
