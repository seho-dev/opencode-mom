import { getContext, setContext } from 'svelte';
import type { ConfigStore } from './store.svelte.js';
const key = Symbol('config-store');
export const setConfig = (store: ConfigStore) => setContext(key, store);
export const getConfig = () => getContext<ConfigStore>(key);
