import type { ToastAction, ToastVariant } from '$lib/components/ui/toast/index.js';

export type { ToastAction, ToastVariant };

export type ToastOptions = {
  title?: string;
  description?: string;
  variant?: ToastVariant;
  /** Primary action rendered as a single button (e.g. Retry). */
  action?: ToastAction;
  /** Optional multiple actions rendered as a row of buttons (used by conflict recovery). */
  actions?: ToastAction[];
  /** Time in ms before the toast auto-dismisses. Defaults to 5000 (8000 for errors). */
  duration?: number;
};

type ToastEntry = {
  id: number;
  title?: string;
  description?: string;
  variant: ToastVariant;
  actions: ToastAction[];
  leaving: boolean;
};

const toasts = $state<ToastEntry[]>([]);
let nextId = 0;

const DEFAULT_DURATION = 5000;
const ERROR_DURATION = 8000;
const EXIT_MS = 250;

function remove(id: number) {
  const index = toasts.findIndex((entry) => entry.id === id);
  if (index !== -1) toasts.splice(index, 1);
}

function dismiss(id: number) {
  const entry = toasts.find((item) => item.id === id);
  if (!entry || entry.leaving) return;
  entry.leaving = true;
  setTimeout(() => remove(id), EXIT_MS);
}

/** Push a toast onto the global stack and schedule its auto-dismissal. */
export function toast(options: ToastOptions) {
  const id = ++nextId;
  const entry: ToastEntry = {
    id,
    title: options.title,
    description: options.description,
    variant: options.variant ?? 'info',
    actions: options.actions ?? (options.action ? [options.action] : []),
    leaving: false,
  };
  toasts.push(entry);
  const duration = options.duration ?? (options.variant === 'error' ? ERROR_DURATION : DEFAULT_DURATION);
  setTimeout(() => dismiss(id), duration);
  return id;
}

/** Manually dismiss (and close) a toast, running its exit animation. */
export function dismissToast(id: number) {
  dismiss(id);
}

/** Reactive list of active toasts consumed by the Toaster viewport. */
export function getToasts() {
  return toasts;
}
