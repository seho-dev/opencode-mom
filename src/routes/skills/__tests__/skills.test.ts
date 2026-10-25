import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page as routePage } from '$app/state';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { SkillEntry } from '$src/types/skills.js';
import SkillList from '../+page.svelte';
import SkillDetail from '../[id]/+page.svelte';
import SkillNew from '../new/+page.svelte';
import SkillForm from '../SkillForm.svelte';

const page: { params: { id?: string }; url: URL } = routePage;

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
  const preview = screen.getByRole('region', { name: `${en['skills.content']}: ${skill.name}` });
  expect(preview.tabIndex).toBe(0);
  preview.focus();
  expect(document.activeElement).toBe(preview);
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
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText(/Frontmatter is invalid/);
  expect(input.value).toBe(content);
  expect((screen.getByLabelText('ID') as HTMLInputElement).value).toBe('new-skill');
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/skills/new-skill?saved=1'));
  expect(createSkill).toHaveBeenNthCalledWith(2, { id: 'new-skill', content });
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});

test('Local edit keeps expected snapshot and draft after conflict; cancel restores saved document', async () => {
  const updateSkill = vi.fn().mockRejectedValue({ code: 'configuration_failed', message: 'File changed externally' });
  render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ updateSkill }) } });
  await screen.findByRole('heading', { name: skill.name });
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  const input = screen.getByLabelText('Full Markdown document') as HTMLTextAreaElement;
  expect(input.value).toBe(skill.content);
  expect((screen.getByLabelText('ID') as HTMLInputElement).disabled).toBe(true);
  const content = '# Edited\n\n<script>not executed</script>\n';
  await fireEvent.input(input, { target: { value: content } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
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
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
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
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
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
  await screen.findByText(en['skills.remoteHint']);
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
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
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
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
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
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
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
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

test('project skills retain same-name Global comparison as a second readonly entry', async () => {
  const listSkills = vi.fn(async (id?: string) => ({
    data: [{ ...skill, path: id ? '/work/alpha/.opencode/skills/docs/SKILL.md' : skill.path }],
    diagnostics: [],
  }));
  render(SkillList, {}, { wrapper: Harness, wrapperProps: { config: makeScopedConfig({ listSkills }) } });
  const link = await screen.findByRole('link', { name: skill.name });
  expect(listSkills).toHaveBeenCalledExactlyOnceWith('project-a');
  expect(link.getAttribute('href')).toBe('/skills/docs%2Fa%20b?project=project-a');
  expect(screen.queryByRole('link', { name: 'New skill' })).toBeNull();
  expect(screen.queryByRole('link', { name: `Edit ${skill.name}` })).toBeNull();
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  await waitFor(() => expect(screen.getAllByRole('link', { name: skill.name })).toHaveLength(2));
  expect(screen.getAllByRole('link', { name: skill.name }).map((entry) => entry.getAttribute('href'))).toEqual([
    '/skills/docs%2Fa%20b?project=project-a',
    '/skills/docs%2Fa%20b?readonly=1',
  ]);
  expect(screen.queryByRole('link', { name: `Edit ${skill.name}` })).toBeNull();
});

test('late scoped skills cannot overwrite returned Global; scope switch resets search', async () => {
  let resolve: (value: { data: SkillEntry[]; diagnostics: string[] }) => void = () => {};
  const globalSkill = { ...skill, id: 'global', name: 'Global skill' };
  const listSkills = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    )
    .mockResolvedValueOnce({ data: [globalSkill], diagnostics: [] });
  render(SkillList, {}, { wrapper: Harness, wrapperProps: { config: makeScopedConfig({ listSkills }) } });
  await waitFor(() => expect(listSkills).toHaveBeenCalledWith('project-a'));
  await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'old query' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Return to Global' }));
  await screen.findByRole('link', { name: globalSkill.name });
  expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('');
  resolve({ data: [skill], diagnostics: ['late skill'] });
  await waitFor(() => expect(screen.queryByRole('link', { name: skill.name })).toBeNull());
  expect(screen.queryByText('late skill')).toBeNull();
  expect(listSkills).toHaveBeenCalledTimes(2);
});

test('project skill detail ignores direct edit intent and keeps exact project on refresh', async () => {
  page.url = new URL('http://localhost/skills/docs?project=project-a&edit=1&saved=1');
  const getSkill = vi.fn().mockResolvedValue(skill);
  const config = makeScopedConfig({ getSkill, updateSkill: vi.fn() });
  render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('heading', { name: skill.name });
  expect(getSkill).toHaveBeenCalledExactlyOnceWith(skill.id, 'project-a');
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(screen.queryByText(/Saved to global configuration/)).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(getSkill).toHaveBeenCalledTimes(2));
  expect(getSkill).toHaveBeenLastCalledWith(skill.id, 'project-a');
  expect(config.updateSkill).not.toHaveBeenCalled();
});

test('Skill Global comparison failure is isolated and keeps project rows without previous Global data', async () => {
  const listSkills = vi
    .fn()
    .mockResolvedValueOnce({ data: [skill], diagnostics: [] })
    .mockRejectedValueOnce(new Error('global read denied'))
    .mockResolvedValueOnce({ data: [], diagnostics: [] });
  const config = makeScopedConfig({ listSkills });
  render(SkillList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: skill.name });
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('Global comparison could not be loaded');
  expect(screen.getAllByRole('link', { name: skill.name })).toHaveLength(1);
  expect(config.selectedProjectId).toBe('project-a');
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  expect(screen.getAllByRole('link', { name: skill.name })).toHaveLength(1);
  expect(listSkills).toHaveBeenCalledTimes(3);
  expect(listSkills).toHaveBeenLastCalledWith();
});

test('Skill Global comparison detail ignores edit query and says switch Global to edit', async () => {
  page.url = new URL('http://localhost/skills/docs?readonly=1&edit=1');
  const getSkill = vi.fn().mockResolvedValue(skill);
  const config = makeScopedConfig({ getSkill });
  render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('heading', { name: skill.name });
  expect(getSkill).toHaveBeenCalledExactlyOnceWith(skill.id);
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  expect(screen.getByText(en['scope.globalReadOnly'])).toBeTruthy();
  expect(config.selectedProjectId).toBe('project-a');
});

test('same skill ID across project queries never retains content from a late different scope', async () => {
  page.url = new URL('http://localhost/skills/docs?project=project-a&edit=1');
  let resolve: (value: SkillEntry) => void = () => {};
  const getSkill = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    )
    .mockResolvedValueOnce({
      ...skill,
      path: '/other/alpha/.opencode/skills/docs/SKILL.md',
      content: 'Project B body',
    });
  const config = makeScopedConfig({ getSkill });
  render(SkillDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await waitFor(() => expect(getSkill).toHaveBeenCalledWith(skill.id, 'project-a'));
  page.url = new URL('http://localhost/skills/docs?project=project-b&edit=1');
  await screen.findByText('Project B body');
  expect(getSkill).toHaveBeenLastCalledWith(skill.id, 'project-b');
  resolve({ ...skill, content: 'Late Project A body' });
  await waitFor(() => expect(screen.queryByText('Late Project A body')).toBeNull());
  expect(screen.getByText('/other/alpha/.opencode/skills/docs/SKILL.md')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(config.selectedProjectId).toBe('project-a');
});
