import { createSubscriber } from 'svelte/reactivity';
import { vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';

export function makeScopedConfig(overrides: Record<string, unknown> = {}, initial: string | null = 'project-a') {
  let selected = initial;
  let projects = [
    { id: 'project-a', path: '/work/alpha' },
    { id: 'project-b', path: '/other/alpha' },
  ];
  let notify = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  const config = {
    preferences: { locale: 'en', theme: 'light' },
    saving: false,
    loading: false,
    projectSaving: false,
    agents: [],
    get selectedProjectId() {
      track();
      return selected;
    },
    get projects() {
      track();
      return projects;
    },
    selectProject: vi.fn(async (id: string | null) => {
      selected = id;
      notify();
    }),
    pickProjectFolder: vi.fn().mockResolvedValue(null),
    addProject: vi.fn(async (path: string) => {
      const project = projects.find((entry) => entry.path === path) ?? { id: 'project-new', path };
      if (!projects.includes(project)) projects = [...projects, project];
      selected = project.id;
      notify();
      return project;
    }),
    removeProject: vi.fn(async (id: string) => {
      projects = projects.filter((entry) => entry.id !== id);
      if (selected === id) selected = null;
      notify();
    }),
    ...overrides,
  };
  return config as unknown as ConfigStore;
}
