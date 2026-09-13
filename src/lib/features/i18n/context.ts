import { getContext, setContext } from 'svelte';
import type { I18n } from './i18n.svelte.js';
const key = Symbol('i18n');
export const setI18n = (value: I18n) => setContext(key, value);
export const getI18n = () => getContext<I18n>(key);
