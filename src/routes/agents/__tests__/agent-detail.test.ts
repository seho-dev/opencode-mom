import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import { page as routePage } from '$app/state';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { AgentDefinition } from '$src/types/agents.js';
import AgentDetail from '../[id]/+page.svelte';

const page: { params: { id?: string }; url: URL } = routePage;

vi.mock('$app/state', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  let params = { id: 'helper' };
  let url = new URL('http://localhost/agents/helper?project=project-a');
  return {
    page: {
      get params() {
        track();
        return params;
      },
      set params(value) {
        params = value;
        notify();
      },
      get url() {
        track();
        return url;
      },
      set url(value) {
        url = value;
        notify();
      },
    },
  };
});
const agent: AgentDefinition = {
  id: 'helper',
  source: 'markdown',
  description: 'Project helper',
  mode: 'subagent',
  sources: [
    {
      storage: 'global_markdown',
      path: '/work/alpha/.opencode/agents/helper.md',
      raw: '---\nmode: subagent\n---\n\n<script>unsafe()</script>',
      fields: { mode: 'subagent' },
      prompt: '<script>unsafe()</script>',
    },
  ],
};
beforeEach(() => {
  page.params = { id: 'helper' };
  page.url = new URL('http://localhost/agents/helper?project=project-a&edit=1');
});

test('agent detail loads explicit project, shows source and safe text, and offers no writes or scope selector', async () => {
  const getAgent = vi.fn().mockResolvedValue(agent);
  const config = makeScopedConfig({ getAgent }, 'project-b');
  const { container } = render(AgentDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('heading', { name: agent.id });
  expect(getAgent).toHaveBeenCalledExactlyOnceWith('helper', 'project-a');
  expect(screen.getByText('/work/alpha/.opencode/agents/helper.md')).toBeTruthy();
  expect(container.querySelector('pre')?.textContent).toContain('<script>unsafe()</script>');
  const preview = screen.getByRole('region', {
    name: `${en['agents.sourcePreview']}: /work/alpha/.opencode/agents/helper.md`,
  });
  expect(preview.tabIndex).toBe(0);
  preview.focus();
  expect(document.activeElement).toBe(preview);
  expect(container.querySelector('script')).toBeNull();
  expect(screen.queryByRole('button', { name: /Edit|Delete|Select configuration scope/ })).toBeNull();
  expect(config.selectedProjectId).toBe('project-b');
  expect(config.selectProject).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(getAgent).toHaveBeenCalledTimes(2));
  expect(getAgent).toHaveBeenLastCalledWith('helper', 'project-a');
});

test('same resource ID with another scope ignores late detail and Global comparison stays readonly', async () => {
  let resolve: (value: AgentDefinition) => void = () => {};
  const getAgent = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    )
    .mockResolvedValueOnce({ ...agent, description: 'Global comparison' });
  render(AgentDetail, {}, { wrapper: Harness, wrapperProps: { config: makeScopedConfig({ getAgent }) } });
  await waitFor(() => expect(getAgent).toHaveBeenCalledTimes(1));
  page.url = new URL('http://localhost/agents/helper');
  await screen.findByText('Global comparison');
  resolve(agent);
  await waitFor(() => expect(screen.queryByText('Project helper')).toBeNull());
  expect(getAgent).toHaveBeenLastCalledWith('helper');
  expect(screen.getByText(/Global comparison · read-only/)).toBeTruthy();
  expect(screen.queryByRole('link', { name: 'Edit helper' })).toBeNull();
});

test('source and prompt previews expose separate named keyboard entries without changing content', async () => {
  const path = '/global/opencode.jsonc';
  const raw = { mode: 'subagent' };
  const prompt = 'Prompt body\n'.repeat(100);
  const entry: AgentDefinition = {
    ...agent,
    sources: [{ storage: 'inline', path, raw, fields: raw, prompt }],
  };
  const getAgent = vi.fn().mockResolvedValue(entry);
  render(AgentDetail, {}, { wrapper: Harness, wrapperProps: { config: makeScopedConfig({ getAgent }) } });
  await screen.findByRole('heading', { name: agent.id });
  for (const [label, content] of [
    [en['agents.sourcePreview'], JSON.stringify(raw, null, 2)],
    [en['agents.prompt'], prompt],
  ]) {
    const preview = screen.getByRole('region', { name: `${label}: ${path}` });
    expect(preview.textContent).toBe(content);
    expect(preview.tabIndex).toBe(0);
    preview.focus();
    expect(document.activeElement).toBe(preview);
  }
});

test('legacy source preview keeps a named keyboard entry', async () => {
  const getAgent = vi.fn().mockResolvedValue({ ...agent, sources: [] });
  render(AgentDetail, {}, { wrapper: Harness, wrapperProps: { config: makeScopedConfig({ getAgent }) } });
  const preview = await screen.findByRole('region', { name: `${en['agents.sourcePreview']}: ${agent.id}` });
  expect(preview.tabIndex).toBe(0);
  preview.focus();
  expect(document.activeElement).toBe(preview);
});
