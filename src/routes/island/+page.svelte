<script lang="ts">
import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  RefreshCw,
  Send,
  ShieldCheck,
  Square,
} from '@lucide/svelte';
import { isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onMount, tick, untrack } from 'svelte';
import { readError } from '$src/components/configuration/read-error.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type {
  IslandMessage,
  IslandPermission,
  IslandPermissionDecision,
  IslandSession,
  IslandState,
  IslandView,
} from '$src/types/island.js';
import QuestionForm from './QuestionForm.svelte';

const config = getConfig();
const i18n = getI18n();
let snapshot = $state.raw<IslandState | null>(null);
let view = $state<IslandView>('compact');
const expanded = $derived(view === 'expanded');
const peeking = $derived(view === 'peek');
let resizing = $state(false);
let selectedId = $state('');
let messages = $state.raw<IslandMessage[]>([]);
let messageSession = $state('');
let reading = $state(false);
let messagesLoading = $state(false);
let stateAvailable = $state(false);
let queryError = $state('');
let operationError = $state('');
let listenerError = $state('');
let busy = $state(false);
let compactPermissionKey = $state('');
let domHovered = $state(false);
let nativeHovered = $state(false);
const compactHovered = $derived(domHovered || nativeHovered);
let compactFocused = $state(false);
let detailsOpen = $state(false);
let permissionMenuKey = $state('');
let permissionAttempt = $state<{
  sessionId: string;
  id: string;
  status: 'replying' | 'checking' | 'pending' | 'uncertain';
  error: string;
} | null>(null);
let drafts = $state<
  Record<string, { text: string; id: string; uncertain: boolean; conflict: boolean; refreshed: boolean }>
>({});
let mounted = false;
let queued = false;
let manualQueued = false;
let refreshQueued = false;
let viewVersion = 0;
let stateVersion = 0;
let connectionGeneration = $state(0);
let messageRequest = 0;
let messagesReading = false;
let messagesQueued = false;
let manualMessagesQueued = false;
let resizeVersion = 0;
let resizeRequest: { view: IslandView; version: number; collapsed: boolean } | null = null;
let resizingTo: IslandView | null = null;
let nativeView: IslandView = 'compact';
let timer: ReturnType<typeof setTimeout> | undefined;
let peekTimer: ReturnType<typeof setTimeout> | undefined;
let entryTimer: ReturnType<typeof setTimeout> | undefined;
const pendingIds = $derived(
  new Set([...(snapshot?.permissions ?? []), ...(snapshot?.questions ?? [])].map((entry) => entry.sessionId)),
);
const pendingCount = $derived((snapshot?.permissions.length ?? 0) + (snapshot?.questions.length ?? 0));
const sessions = $derived.by(() => {
  const entries = snapshot?.sessions ?? [];
  return entries
    .filter((entry) => !entry.parentId || pendingIds.has(entry.id) || entry.status === 'running')
    .sort((a, b) => priority(a) - priority(b) || b.updatedAt - a.updatedAt);
});
const selected = $derived(sessions.find((entry) => entry.id === selectedId));
const permissions = $derived(snapshot?.permissions.filter((entry) => entry.sessionId === selectedId) ?? []);
const questions = $derived(snapshot?.questions.filter((entry) => entry.sessionId === selectedId) ?? []);
const draft = $derived(drafts[selectedId]);
const connection = $derived(
  snapshot?.enabled === false
    ? 'disabled'
    : !stateAvailable && queryError
      ? 'error'
      : (snapshot?.connection ?? 'connecting'),
);
const runningCount = $derived(
  connection === 'connected'
    ? (snapshot?.sessions.filter((session) => session.status === 'running').length ?? 0)
    : null,
);
const canWrite = $derived(snapshot?.enabled === true && connection === 'connected' && !!selected && !busy);
const geometryKnown = $derived(
  !!snapshot &&
    Number.isFinite(snapshot.topInset) &&
    snapshot.topInset >= 0 &&
    Number.isFinite(snapshot.notchWidth) &&
    snapshot.notchWidth >= 0,
);
const topInset = $derived(Number.isFinite(snapshot?.topInset) ? Math.max(0, snapshot?.topInset ?? 0) : 0);
const notchWidth = $derived(Number.isFinite(snapshot?.notchWidth) ? Math.max(0, snapshot?.notchWidth ?? 0) : 0);
const hasNotch = $derived(topInset > 0 && notchWidth > 0);
const idleWidth = $derived(Math.max(notchWidth + 112, 200));
const notchedCompact = $derived(view === 'compact' && hasNotch);
const compactPermissions = $derived(snapshot?.permissions ?? []);
const compactPermission = $derived(
  compactPermissions.find((entry) => permissionKey(entry) === compactPermissionKey) ?? compactPermissions[0],
);
const hasCompactAttention = $derived(
  snapshot?.enabled === true && connection === 'connected' && !!compactPermissions.length,
);
const compactAttention = $derived(peeking && hasCompactAttention);
const compactStatusLabel = $derived(
  runningCount === null
    ? i18n.t(`island.connection.${connection}`)
    : i18n.t('island.runningCount', { count: runningCount }),
);
const compactConnected = $derived(snapshot?.enabled === true && connection === 'connected');
const compactSummary = $derived(
  runningCount === null
    ? i18n.t(
        connection === 'error'
          ? 'island.compact.unavailable'
          : connection === 'connecting'
            ? 'island.compact.connecting'
            : `island.connection.${connection}`,
      )
    : runningCount === 0
      ? i18n.t('island.session.idle')
      : i18n.t('island.compact.runningCount', { count: runningCount > 99 ? '99+' : runningCount }),
);
const compactStateLabel = $derived(
  i18n.t(
    compactConnected
      ? (runningCount ?? 0) > 0
        ? 'island.session.running'
        : 'island.session.idle'
      : connection === 'error'
        ? 'island.compact.offline'
        : connection === 'disabled'
          ? 'island.compact.off'
          : 'island.compact.connecting',
  ),
);
const compactSessionCount = $derived(snapshot?.sessions.length ?? 0);
const compactPrimary = $derived(
  compactConnected
    ? `${compactSummary} · ${i18n.t('island.compact.sessionCount', { count: compactSessionCount > 99 ? '99+' : compactSessionCount })}`
    : compactSummary,
);
const compactSecondary = $derived(
  compactConnected
    ? i18n.t('island.compact.requestCounts', {
        permissions: (snapshot?.permissions.length ?? 0) > 99 ? '99+' : (snapshot?.permissions.length ?? 0),
        questions: (snapshot?.questions.length ?? 0) > 99 ? '99+' : (snapshot?.questions.length ?? 0),
      })
    : i18n.t('island.compact.unknownCounts'),
);
const compactCountsLabel = $derived(
  compactConnected
    ? i18n.t('island.compact.snapshotCounts', {
        sessions: compactSessionCount,
        permissions: snapshot?.permissions.length ?? 0,
        questions: snapshot?.questions.length ?? 0,
      })
    : i18n.t('island.compact.countsUnavailable'),
);
const running = $derived(snapshot?.enabled === true && connection === 'connected' && (runningCount ?? 0) > 0);
const compactSession = $derived(snapshot?.sessions.find((entry) => entry.id === compactPermission?.sessionId));
const compactIndex = $derived(
  compactPermissions.findIndex((entry) => samePermission(entry, compactPermission ?? null)),
);
const compactError = $derived(
  permissionAttempt?.error || operationError || queryError || listenerError || snapshot?.error,
);

function samePermission(a: IslandPermission, b: { sessionId: string; id: string } | null) {
  return a.sessionId === b?.sessionId && a.id === b?.id;
}
function permissionKey(permission: IslandPermission) {
  return `${permission.sessionId}/${permission.id}`;
}
function syncPermission(result: IslandState, manual: boolean) {
  if (
    !result.enabled ||
    result.connection !== 'connected' ||
    !result.permissions.some((entry) => permissionKey(entry) === permissionMenuKey && entry.save?.length)
  )
    permissionMenuKey = '';
  if (!result.enabled || result.connection !== 'connected' || permissionAttempt?.status === 'replying') return;
  if (permissionAttempt) {
    if (!result.permissions.some((entry) => samePermission(entry, permissionAttempt))) permissionAttempt = null;
    else if (manual) permissionAttempt = null;
    else if (permissionAttempt.status === 'checking') permissionAttempt.status = 'pending';
  }
  if (!result.permissions.some((entry) => permissionKey(entry) === compactPermissionKey))
    compactPermissionKey = result.permissions[0] ? permissionKey(result.permissions[0]) : '';
}
function canReplyPermission(permission: IslandPermission) {
  return (
    mounted &&
    stateAvailable &&
    snapshot?.enabled === true &&
    connection === 'connected' &&
    !busy &&
    !permissionAttempt &&
    snapshot.permissions.some((entry) => samePermission(entry, permission))
  );
}
function cyclePermission(direction: number) {
  if (busy || permissionAttempt || !compactAttention || compactIndex < 0) return;
  const index = (compactIndex + direction + compactPermissions.length) % compactPermissions.length;
  compactPermissionKey = permissionKey(compactPermissions[index]);
  permissionMenuKey = '';
}
async function replyPermission(permission: IslandPermission, decision: IslandPermissionDecision, generation: number) {
  if (!mounted || generation !== connectionGeneration || !canReplyPermission(permission)) return;
  const current = snapshot?.permissions.find((entry) => samePermission(entry, permission));
  if (decision === 'always' && !current?.save?.length) return;
  const { sessionId, id } = permission;
  busy = true;
  permissionMenuKey = '';
  operationError = '';
  permissionAttempt = { sessionId, id, status: 'replying', error: '' };
  try {
    await config.islandReplyPermission(sessionId, id, decision, generation);
    if (!mounted || generation !== connectionGeneration) return;
    permissionAttempt.status = 'checking';
    stateVersion += 1;
    requestRead(false, true);
  } catch (cause) {
    if (!mounted || generation !== connectionGeneration) return;
    permissionAttempt.status = 'uncertain';
    permissionAttempt.error = readError(cause);
    requestRead();
  } finally {
    if (mounted && generation === connectionGeneration) {
      busy = false;
      if (!compactHovered && !compactFocused && !detailsOpen && !permissionMenuKey) scheduleCompact();
    }
  }
}

function compactFocus(event: FocusEvent) {
  if (!mounted || expanded || snapshot?.enabled === false) return;
  compactFocused =
    event.type === 'focusin' ||
    (event.currentTarget instanceof HTMLElement &&
      event.relatedTarget instanceof Node &&
      event.currentTarget.contains(event.relatedTarget));
  if (compactFocused) revealDetails();
  else {
    detailsOpen = false;
    permissionMenuKey = '';
    scheduleCompact();
  }
}
function revealDetails(immediate = false) {
  if (!mounted || !geometryKnown || expanded || snapshot?.enabled === false) return;
  clearTimeout(peekTimer);
  peekTimer = undefined;
  if (immediate) {
    detailsOpen = true;
    clearTimeout(entryTimer);
    entryTimer = undefined;
    void setView('peek');
  } else if (entryTimer === undefined) {
    entryTimer = setTimeout(() => {
      entryTimer = undefined;
      if (mounted && (compactHovered || compactFocused)) void setView('peek');
    }, 100);
  }
}
function keepPeek() {
  return (
    compactHovered ||
    compactFocused ||
    detailsOpen ||
    !!permissionMenuKey ||
    busy ||
    permissionAttempt?.status === 'replying' ||
    permissionAttempt?.status === 'checking'
  );
}
function scheduleCompact() {
  clearTimeout(peekTimer);
  peekTimer = undefined;
  if (!mounted || (!peeking && resizingTo !== 'peek') || keepPeek()) return;
  peekTimer = setTimeout(() => {
    peekTimer = undefined;
    if (!keepPeek() && !expanded && resizingTo !== 'expanded') void setView('compact');
  }, 180);
}
function enterDetails(source: 'dom' | 'native') {
  if (!mounted || !geometryKnown || expanded || snapshot?.enabled === false) return;
  if (source === 'dom') {
    if (domHovered) return;
    domHovered = true;
  } else {
    if (nativeHovered) return;
    nativeHovered = true;
  }
  revealDetails();
}
function leaveDetails(source: 'dom' | 'native') {
  if (!mounted || expanded || snapshot?.enabled === false) return;
  if (source === 'dom') domHovered = false;
  else nativeHovered = false;
  if (compactHovered) return;
  if (!compactFocused) {
    detailsOpen = false;
    clearTimeout(entryTimer);
    entryTimer = undefined;
  }
  scheduleCompact();
}
function dismissDetails() {
  clearTimeout(entryTimer);
  entryTimer = undefined;
  clearTimeout(peekTimer);
  peekTimer = undefined;
  detailsOpen = false;
  domHovered = false;
  nativeHovered = false;
  compactFocused = false;
  permissionMenuKey = '';
}
function outsideDetails(event: PointerEvent) {
  if (!(event.target instanceof Element) || !event.target.closest('.approval-options, .approval-dropdown')) {
    permissionMenuKey = '';
    if (!(event.target instanceof Element) || !event.target.closest('.island')) {
      detailsOpen = false;
      scheduleCompact();
    }
  }
}
async function togglePermissionMenu(permission: IslandPermission, toggle = true) {
  if (!canReplyPermission(permission) || !permission.save?.length) return;
  const generation = connectionGeneration;
  const key = permissionKey(permission);
  permissionMenuKey = toggle && permissionMenuKey === key ? '' : key;
  if (!permissionMenuKey) scheduleCompact();
  else if (expanded) {
    await tick();
    if (mounted && generation === connectionGeneration && expanded && permissionMenuKey === key)
      document.getElementById(`approval-options-${key}`)?.scrollIntoView?.({ block: 'nearest' });
  }
}
async function closePermissionMenu(focus = false) {
  const key = permissionMenuKey;
  const generation = connectionGeneration;
  permissionMenuKey = '';
  if (focus && key) {
    await tick();
    if (!mounted || generation !== connectionGeneration) return;
    document.querySelectorAll<HTMLButtonElement>('.menu-trigger').forEach((button) => {
      if (button.dataset.permission === key) button.focus();
    });
  }
  scheduleCompact();
}
function handleEscape(event: KeyboardEvent) {
  if (event.key !== 'Escape') return;
  event.preventDefault();
  if (permissionMenuKey) {
    void closePermissionMenu(true);
    return;
  }
  dismissDetails();
  if (peeking && (busy || permissionAttempt?.status === 'replying' || permissionAttempt?.status === 'checking')) return;
  void setExpanded(false);
}

$effect(() => {
  if (peeking && !keepPeek()) untrack(scheduleCompact);
});

function priority(session: IslandSession) {
  return pendingIds.has(session.id) ? 0 : session.status === 'running' ? 1 : 2;
}
function project(session: IslandSession) {
  return (
    session.directory
      .replace(/[\\/]+$/, '')
      .split(/[\\/]/)
      .pop() || session.directory
  );
}
function choose(id: string, reload = true) {
  if (selectedId === id) return;
  selectedId = id;
  viewVersion += 1;
  operationError = '';
  if (!drafts[id]) clearDraft(id);
  if (expanded && reload) requestRead();
}
function requestRead(manual = false, refresh = false) {
  if (!mounted) return;
  clearTimeout(timer);
  timer = undefined;
  queued = true;
  manualQueued ||= manual;
  refreshQueued ||= manual || refresh;
  void read();
}
function invalidate() {
  if (!mounted || timer !== undefined) return;
  timer = setTimeout(() => requestRead(), 200);
}
function resetConnection(generation: number) {
  connectionGeneration = generation;
  stateVersion += 1;
  viewVersion += 1;
  messageRequest += 1;
  resizeVersion += 1;
  resizeRequest = null;
  resizing = false;
  resizingTo = null;
  selectedId = '';
  messages = [];
  messageSession = '';
  drafts = {};
  permissionAttempt = null;
  compactPermissionKey = '';
  permissionMenuKey = '';
  operationError = '';
  queryError = '';
  busy = false;
  messagesLoading = false;
  messagesReading = false;
  messagesQueued = false;
  manualMessagesQueued = false;
  manualQueued = false;
  refreshQueued = false;
}
async function read() {
  if (!mounted || reading) return;
  reading = true;
  while (mounted && queued) {
    queued = false;
    let manual = manualQueued;
    manualQueued = false;
    const refresh = refreshQueued;
    refreshQueued = false;
    let stateSeq = stateVersion;
    try {
      if (refresh) await config.refreshIsland();
      if (!mounted) break;
      const result = await config.getIslandState();
      if (!mounted) break;
      const generation = result.connectionGeneration;
      if (generation < connectionGeneration) continue;
      if (generation > connectionGeneration) {
        resetConnection(generation);
        stateSeq = stateVersion;
        manual = false;
      }
      if (stateSeq !== stateVersion) continue;
      snapshot = result;
      if (!result.enabled) {
        dismissDetails();
        if (resizingTo === 'peek') collapsed();
      }
      stateAvailable = true;
      syncPermission(result, manual);
      config.applyPreferences(result.preferences);
      queryError = '';
      if (!sessions.some((entry) => entry.id === selectedId)) choose(sessions[0]?.id ?? '', false);
      if (expanded && selectedId && result.enabled && result.connection === 'connected') void readMessages(manual);
    } catch (cause) {
      if (mounted && stateSeq === stateVersion) {
        stateAvailable = false;
        queryError = readError(cause);
      }
    }
  }
  if (mounted) reading = false;
}
async function readMessages(manual: boolean) {
  if (messagesReading) {
    messagesQueued = true;
    manualMessagesQueued ||= manual;
    return;
  }
  const request = ++messageRequest;
  const generation = connectionGeneration;
  const id = selectedId;
  const seq = viewVersion;
  const stateSeq = stateVersion;
  const current = () =>
    mounted &&
    expanded &&
    id === selectedId &&
    seq === viewVersion &&
    stateSeq === stateVersion &&
    generation === connectionGeneration &&
    snapshot?.enabled === true &&
    connection === 'connected';
  messagesReading = true;
  messagesLoading = true;
  try {
    const recent = await config.islandMessages(id, generation);
    if (!current()) return;
    messages = recent.filter(
      (message) => (message.role === 'user' || message.role === 'assistant') && typeof message.text === 'string',
    );
    messageSession = id;
    if (manual && drafts[id]?.uncertain) {
      if (messages.some((message) => message.role === 'user' && message.id === drafts[id].id)) {
        clearDraft(id);
        operationError = '';
      } else drafts[id].refreshed = true;
    }
  } catch (cause) {
    if (current()) queryError = readError(cause);
  } finally {
    if (mounted && request === messageRequest) {
      messagesReading = false;
      messagesLoading = false;
      const nextManual = manualMessagesQueued;
      const next = messagesQueued;
      messagesQueued = false;
      manualMessagesQueued = false;
      if (next && expanded && selectedId && snapshot?.enabled && connection === 'connected')
        void readMessages(nextManual);
    }
  }
}
function setExpanded(next: boolean) {
  clearTimeout(entryTimer);
  entryTimer = undefined;
  clearTimeout(peekTimer);
  peekTimer = undefined;
  return setView(next ? 'expanded' : 'compact');
}
async function setView(next: IslandView) {
  if (!mounted || !geometryKnown) return;
  if (next === 'peek' && snapshot?.enabled === false) return;
  if (!resizing && next === view && next === nativeView) return;
  if (next === 'peek' && (expanded || resizingTo === 'expanded')) return;
  if (resizingTo === next) return;
  const request = { view: next, version: ++resizeVersion, collapsed: false };
  resizeRequest = request;
  resizingTo = next;
  resizing = true;
  operationError = '';
  try {
    await config.setIslandView(next);
    if (!mounted || resizeRequest !== request || request.version !== resizeVersion) return;
    nativeView = next;
    view = next;
    if (next !== 'peek') dismissDetails();
    viewVersion += 1;
    if (next === 'expanded') requestRead();
  } catch (cause) {
    if (mounted && resizeRequest === request && request.version === resizeVersion) operationError = readError(cause);
  } finally {
    if (mounted && resizeRequest === request) {
      resizeRequest = null;
      resizing = false;
      resizingTo = null;
      if (peeking) scheduleCompact();
    }
  }
}
function collapsed() {
  if (!mounted) return;
  // Compact's first native event acknowledges this request before final readback.
  if (resizeRequest?.view === 'compact' && !resizeRequest.collapsed) resizeRequest.collapsed = true;
  else {
    resizeVersion += 1;
    resizeRequest = null;
    resizing = false;
    resizingTo = null;
  }
  viewVersion += 1;
  nativeView = 'compact';
  view = 'compact';
  dismissDetails();
}
function windowBlur() {
  compactFocused = false;
  detailsOpen = false;
  permissionMenuKey = '';
  scheduleCompact();
}
async function act(action: () => Promise<void>, generation: number) {
  if (!mounted || generation !== connectionGeneration || !canWrite) return;
  const id = selectedId;
  busy = true;
  operationError = '';
  try {
    await action();
    if (!mounted || generation !== connectionGeneration) return;
    stateVersion += 1;
    requestRead();
  } catch (cause) {
    if (mounted && generation === connectionGeneration) {
      if (id === selectedId) operationError = readError(cause);
      requestRead();
    }
  } finally {
    if (mounted && generation === connectionGeneration) busy = false;
  }
}
function editDraft(text: string) {
  if (!draft || busy || draft.uncertain) return;
  draft.text = text;
  draft.id = '';
}
function clearDraft(id: string) {
  drafts[id] = { text: '', id: '', uncertain: false, conflict: false, refreshed: false };
}
function discardAttempt() {
  if (!mounted || busy || !draft?.uncertain || !draft.refreshed) return;
  clearDraft(selectedId);
  operationError = '';
}
async function send(event: SubmitEvent) {
  event.preventDefault();
  if (!mounted || !canWrite || !draft?.text.trim() || (draft.uncertain && !draft.refreshed)) return;
  const id = selectedId;
  const generation = connectionGeneration;
  const submission = drafts[id];
  if (!submission.id) submission.id = `msg_${crypto.randomUUID()}`;
  busy = true;
  operationError = '';
  try {
    await config.islandSend(id, submission.text, submission.id, generation);
    if (!mounted || generation !== connectionGeneration) return;
    clearDraft(id);
    stateVersion += 1;
    requestRead();
  } catch (cause) {
    if (!mounted || generation !== connectionGeneration) return;
    const message = readError(cause);
    const code = cause && typeof cause === 'object' && 'code' in cause ? cause.code : undefined;
    submission.conflict = code === 'conflict' || /\bHTTP 409\b/.test(message);
    submission.uncertain =
      submission.conflict ||
      !(code === 'validation_failed' || code === 'not_found' || /\bHTTP (400|401|403|404|422)\b/.test(message));
    submission.refreshed = false;
    if (id === selectedId) operationError = message;
    if (code === 'validation_failed' || code === 'conflict') requestRead();
  } finally {
    if (mounted && generation === connectionGeneration) busy = false;
  }
}

onMount(() => {
  mounted = true;
  const unlisteners: UnlistenFn[] = [];
  if (isTauri()) {
    const registrations = [
      listen('island:changed', invalidate, { target: 'island' }),
      listen('island:collapsed', collapsed, { target: 'island' }),
      listen<void>('island:pointer-enter', () => enterDetails('native'), { target: 'island' }),
      listen<void>('island:pointer-leave', () => leaveDetails('native'), { target: 'island' }),
      listen('app:preferences-changed', invalidate, { target: 'island' }),
    ];
    for (const registration of registrations) {
      void registration
        .then((stop) => {
          if (!mounted) stop();
          else unlisteners.push(stop);
        })
        .catch((cause) => {
          if (mounted) listenerError = readError(cause);
        });
    }
    void Promise.allSettled(registrations).then(() => {
      if (mounted) requestRead();
    });
  } else {
    queryError = i18n.t('island.desktopOnly');
  }
  return () => {
    mounted = false;
    viewVersion += 1;
    stateVersion += 1;
    resizeVersion += 1;
    clearTimeout(timer);
    dismissDetails();
    for (const stop of unlisteners) stop();
  };
});
</script>

<svelte:head><title>{i18n.t('island.title')}</title></svelte:head>
<svelte:window onkeydown={handleEscape} onpointerdown={outsideDetails} onblur={windowBlur} />

{#snippet approvalOptions(permission: IslandPermission, generation: number)}
  {#if permission.save?.length}
    {@const key = permissionKey(permission)}
    <section
      class="approval-options"
      aria-label={i18n.t('island.approvalOptions')}
      onpointerenter={() => togglePermissionMenu(permission, false)}
      onpointerleave={(event) => { if (!event.currentTarget.contains(document.activeElement)) { permissionMenuKey = ''; scheduleCompact(); } }}
    >
      <button
        type="button"
        class="menu-trigger"
        data-permission={key}
        disabled={!canReplyPermission(permission)}
        aria-label={i18n.t('island.approvalOptions')}
        aria-expanded={permissionMenuKey === key}
        aria-controls={`approval-options-${key}`}
        onclick={() => togglePermissionMenu(permission)}
      >
        <ChevronDown size={12} aria-hidden="true" />
      </button>
      {#if permissionMenuKey === key}
        <div id={`approval-options-${key}`} class="approval-dropdown">
          <button
            type="button"
            class="menu-close"
            aria-label={i18n.t('island.closeApprovalOptions')}
            onclick={() => closePermissionMenu(true)}
          >
            <ChevronUp size={12} aria-hidden="true" />
          </button>
          <section id={`approval-scope-${key}`} class="approval-scope" aria-label={i18n.t('island.alwaysProjectHint')}>
            <p>{i18n.t('island.rememberRules', { action: permission.action })}</p>
            <textarea
              class="save-patterns"
              readonly
              rows="1"
              aria-label={i18n.t('island.rememberRules', { action: permission.action })}
              value={permission.save.join(' · ')}
            ></textarea>
            <span id={`approval-patterns-${key}`} class="sr-only">{permission.save.join(' · ')}</span>
            <p>{i18n.t('island.matchingRequestsHint')}</p>
          </section>
          <button
            type="button"
            class="always-button"
            disabled={!canReplyPermission(permission)}
            aria-describedby={`approval-scope-${key} approval-patterns-${key}`}
            onclick={() => replyPermission(permission, 'always', generation)}
          >
            {i18n.t('island.alwaysProject')}
          </button>
        </div>
      {/if}
    </section>
  {/if}
{/snippet}

<main
  class="island"
  class:expanded
  class:compact-attention={peeking}
  class:peek={peeking}
  class:notched-compact={notchedCompact}
  class:unnotched-compact={view === 'compact' && !hasNotch}
  class:geometry-pending={!geometryKnown && isTauri()}
  data-view={view}
  data-state={compactAttention ? 'attention' : running ? 'running' : connection === 'connected' ? 'idle' : 'unavailable'}
  style:--top-inset={`${topInset}px`}
  style:--notch-width={`${notchWidth}px`}
  style:--idle-width={`${idleWidth}px`}
  style:--compact-height={`${Math.max(topInset, 28)}px`}
  style:--compact-width={`${peeking ? 400 : idleWidth}px`}
  aria-label={i18n.t('island.title')}
  onpointerenter={() => enterDetails('dom')}
  onpointermove={() => enterDetails('dom')}
  onpointerleave={() => leaveDetails('dom')}
  onpointerup={() => { if (view === 'compact') revealDetails(true); }}
  onfocusin={compactFocus}
  onfocusout={compactFocus}
>
  {#if !notchedCompact}
    <div class="notch-gap" aria-hidden="true"></div>
  {/if}
  {#if !expanded}
    <section class="compact-shell" aria-label={i18n.t('island.title')} data-details-visible={peeking}>
      <header class="compact-header">
        <button
          type="button"
          class="compact compact-left"
          aria-label={i18n.t('island.peek')}
          aria-expanded={peeking}
          aria-controls={hasCompactAttention ? 'compact-permission-details' : 'compact-quick-area'}
          aria-describedby={!peeking ? `compact-status-hint compact-counts-hint${compactConnected && pendingCount ? ' compact-pending-hint' : ''}` : undefined}
          aria-busy={resizing}
          onclick={() => revealDetails(true)}
        >
          {#if peeking}
            <span class="peek-status content-arrival">{compactStatusLabel}</span>
          {:else if hasNotch}
            <span class="compact-wing-state content-arrival" aria-hidden="true">
              <span class="wing-label">{compactStateLabel}</span>
              <span class="wing-count">{compactConnected ? (runningCount ?? 0) > 99 ? '99+' : runningCount : '—'}</span>
            </span>
          {:else}
            <span class="compact-summary content-arrival" aria-hidden="true">
              <span class="compact-primary">{compactPrimary}</span>
              <span class="compact-secondary">{compactSecondary}</span>
            </span>
          {/if}
        </button>
        {#if notchedCompact}
          <div class="notch-gap" aria-hidden="true"></div>
        {/if}
        <div class="compact-lane">
          <button
            type="button"
            class="compact compact-status-control"
            aria-busy={resizing}
            title={view === 'compact' ? operationError || compactStatusLabel : compactError || (snapshot?.questions.length ? i18n.t('island.openQuestions') : compactStatusLabel)}
            aria-expanded={peeking}
            aria-controls={hasCompactAttention ? 'compact-permission-details' : 'compact-quick-area'}
            aria-label={i18n.t('island.statusControl')}
            aria-describedby={`compact-status-hint${!peeking ? ' compact-counts-hint' : ''}${compactConnected && pendingCount ? ' compact-pending-hint' : ''}${snapshot?.questions.length ? ' compact-questions-hint' : ''}`}
            onclick={() => revealDetails(true)}
          >
            <span class="compact-status">
              <span class="compact-activity-row" aria-hidden="true">
                <span
                  class="activity"
                  class:running
                  class:pending={compactConnected && pendingCount > 0}
                  data-running={running}
                  ><i></i><i></i><i></i><i></i></span
                >
                {#if notchedCompact}
                  <span class="wing-pending-count" class:attention={compactConnected && pendingCount > 0}
                    >{compactConnected ? pendingCount > 99 ? '99+' : pendingCount : '—'}</span
                  >
                {/if}
              </span>
              {#if notchedCompact}
                <span class="wing-label" aria-hidden="true">{i18n.t('island.compact.pendingLabel')}</span>
              {/if}
              <span id="compact-status-hint" class="sr-only">{compactStatusLabel}</span>
              <span id="compact-counts-hint" class="sr-only">{compactCountsLabel}</span>
              {#if compactConnected && pendingCount}
                <span id="compact-pending-hint" class="sr-only"
                  >{i18n.t('island.pendingCount', { count: pendingCount })}</span
                >
              {/if}
            </span>
          </button>
          {#if peeking}
            <button
              type="button"
              class="icon-button session-disclosure"
              disabled={resizing}
              aria-label={i18n.t('island.expand')}
              aria-expanded="false"
              aria-controls="island-session-window"
              onclick={() => setExpanded(true)}
            >
              <ChevronDown size={16} aria-hidden="true" />
            </button>
          {/if}
          {#if snapshot?.questions.length}
            <span id="compact-questions-hint" class="sr-only">{i18n.t('island.openQuestions')}</span>
          {/if}
        </div>
      </header>
      {#if compactAttention && compactPermission}
        {#key `${connectionGeneration}/${compactPermission.sessionId}/${compactPermission.id}`}
          {@const generation = connectionGeneration}
          {@const permission = compactPermission}
          <section
            id="compact-permission-details"
            class="compact-permission content-arrival"
            aria-label={i18n.t('island.permission')}
            aria-busy={busy || permissionAttempt?.status === 'checking'}
          >
            <div class="permission-context">
              <ShieldCheck size={12} aria-hidden="true" />
              {#if compactError}
                <span class="permission-error" role="alert" title={compactError}>{compactError}</span>
              {:else if permissionAttempt}
                <span role="status"
                  >{i18n.t(permissionAttempt.status === 'replying' ? 'island.replying' : 'island.permissionReadback')}</span
                >
              {:else}
                <span
                  class="permission-origin"
                  title={compactSession ? `${compactSession.directory} · ${compactSession.title}` : compactPermission.sessionId}
                >
                  {compactSession ? project(compactSession) : compactPermission.sessionId}
                  <span class="origin-divider">/</span>
                  {compactSession?.title || i18n.t('island.untitled')}
                </span>
              {/if}
            </div>
            <div
              class="permission-preview"
              title={[compactPermission.action, compactPermission.message, ...compactPermission.resources].filter(Boolean).join('\n')}
            >
              <span class="permission-action">{compactPermission.action}</span>
              <span class="permission-resource"
                >{compactPermission.resources.join(' · ') || compactPermission.message || i18n.t('island.permission')}</span
              >
            </div>
            <div class="permission-controls">
              <div class="permission-queue">
                {#if compactPermissions.length > 1}
                  <button
                    type="button"
                    class="queue-button"
                    disabled={busy || !!permissionAttempt}
                    aria-label={i18n.t('island.previousRequest')}
                    onclick={() => cyclePermission(-1)}
                  >
                    <ChevronLeft size={13} aria-hidden="true" />
                  </button>
                  <span
                    ><span class="sr-only"
                      >{i18n.t('island.requestPosition', { current: compactIndex + 1, total: compactPermissions.length })}</span
                    ><span aria-hidden="true"
                      >{compactIndex + 1}<span class="queue-divider">/</span>
                      {Math.min(compactPermissions.length, 99)}{compactPermissions.length > 99 ? '+' : ''}</span
                    ></span
                  >
                  <button
                    type="button"
                    class="queue-button"
                    disabled={busy || !!permissionAttempt}
                    aria-label={i18n.t('island.nextRequest')}
                    onclick={() => cyclePermission(1)}
                  >
                    <ChevronRight size={13} aria-hidden="true" />
                  </button>
                {:else}
                  <span>{i18n.t('island.permission')}</span>
                {/if}
              </div>
              <div class="permission-decisions">
                <button
                  type="button"
                  class="reject-button"
                  disabled={!canReplyPermission(permission)}
                  onclick={() => replyPermission(permission, 'reject', generation)}
                >
                  {i18n.t('island.reject')}
                </button>
                <div class="approval-split" class:has-options={!!permission.save?.length}>
                  <button
                    type="button"
                    class="allow-button"
                    disabled={!canReplyPermission(permission)}
                    onclick={() => replyPermission(permission, 'once', generation)}
                  >
                    {i18n.t('island.allowOnce')}
                  </button>
                  {@render approvalOptions(permission, generation)}
                </div>
              </div>
              <button
                type="button"
                class="icon-button quick-refresh"
                disabled={reading || busy}
                aria-label={i18n.t('island.refresh')}
                title={compactError || i18n.t('island.refresh')}
                onclick={() => requestRead(true)}
              >
                <RefreshCw size={13} aria-hidden="true" />
              </button>
            </div>
          </section>
        {/key}
      {:else if peeking}
        <section id="compact-quick-area" class="quick-area content-arrival" aria-label={i18n.t('island.quickArea')}>
          <div class="quick-summary">
            <span>{i18n.t('island.quickArea')}</span>
            <span role="status">{compactStatusLabel}</span>
          </div>
          <p class:error={!!compactError} title={compactError || selected?.directory}>
            {compactError || (snapshot?.questions.length ? i18n.t('island.openQuestions') : selected?.title || i18n.t('island.noSessions'))}
          </p>
          <button
            type="button"
            class="icon-button quick-refresh"
            disabled={reading || busy}
            aria-label={i18n.t('island.refresh')}
            title={compactError || i18n.t('island.refresh')}
            onclick={() => requestRead(true)}
          >
            <RefreshCw size={13} aria-hidden="true" />
          </button>
        </section>
      {/if}
    </section>
  {:else}
    <div id="island-session-window" class="expanded-shell">
      <header class="content-arrival flex h-11 shrink-0 items-center justify-between gap-3 px-5">
        <div class="flex min-w-0 items-center gap-2">
          <h1 class="m-0 truncate text-[13px] font-semibold">{i18n.t('island.title')}</h1>
          {#if pendingCount}
            <span class="pending-badge">{i18n.t('island.pendingCount', { count: pendingCount })}</span>
          {/if}
        </div>
        <div class="flex gap-1">
          <button
            type="button"
            class="icon-button"
            disabled={reading || busy}
            aria-label={i18n.t('island.refresh')}
            onclick={() => requestRead(true)}
          >
            <RefreshCw size={14} aria-hidden="true" />
          </button>
          <button
            type="button"
            class="icon-button"
            disabled={resizing}
            aria-label={i18n.t('island.collapse')}
            onclick={() => setExpanded(false)}
          >
            <ChevronUp size={16} aria-hidden="true" />
          </button>
        </div>
      </header>
      <div class="border-y border-[#262629] px-5 py-2 text-[11px] text-[#b6b6bd]" role="status">
        {i18n.t(`island.connection.${connection}`)}
        {#if snapshot?.error}
          <span class="ml-2 text-[#f0b3a9]">{snapshot.error}</span>
        {/if}
      </div>
      {#if queryError || operationError || listenerError || permissionAttempt?.error}
        <p class="mx-5 my-2 shrink-0 break-words text-xs text-[#f0b3a9]" role="alert">
          {queryError || operationError || listenerError || permissionAttempt?.error}
        </p>
      {/if}
      {#if permissionAttempt}
        <p class="mx-5 my-2 shrink-0 text-[11px] text-[#e3c787]" role="status">
          {i18n.t(permissionAttempt.status === 'replying' ? 'island.replying' : 'island.permissionReadback')}
        </p>
      {/if}
      <div class="island-content">
        <nav class="session-list content-arrival" aria-label={i18n.t('island.sessions')}>
          {#each sessions as session (session.id)}
            <button
              type="button"
              class="session"
              aria-current={session.id === selectedId ? 'true' : undefined}
              onclick={() => choose(session.id)}
            >
              <span class="block truncate text-[10px] text-[#b6b6bd]">{project(session)}</span>
              <span class="mt-1 block line-clamp-2 text-xs leading-4"
                >{session.title || i18n.t('island.untitled')}</span
              >
              <span class:attention={pendingIds.has(session.id)} class="mt-1.5 block text-[10px] text-[#b6b6bd]"
                >{i18n.t(connection !== 'connected' ? 'island.session.unknown' : pendingIds.has(session.id) ? 'island.pending' : `island.session.${session.status}`)}</span
              >
            </button>
          {:else}
            <p class="m-0 p-3 text-xs leading-5 text-[#b6b6bd]">{i18n.t('island.noSessions')}</p>
          {/each}
        </nav>
        <section class="flex min-h-0 min-w-0 flex-1 flex-col" aria-label={i18n.t('island.sessionDetails')}>
          {#if selected}
            <div class="flex shrink-0 items-center justify-between gap-2 border-b border-[#262629] px-4 py-2">
              {#key selectedId}
                <h2 class="content-arrival m-0 truncate text-xs font-medium">
                  {selected.title || i18n.t('island.untitled')}
                </h2>
              {/key}
              {#if selected.status === 'running'}
                {@const generation = connectionGeneration}
                <button
                  type="button"
                  class="flex items-center gap-1 rounded-lg border border-[#35353a] px-2 py-1 text-[11px] disabled:opacity-40"
                  disabled={!canWrite}
                  onclick={() => { const id = selectedId; void act(() => config.islandInterrupt(id, generation), generation); }}
                >
                  <Square size={10} aria-hidden="true" />{i18n.t('island.stop')}
                </button>
              {/if}
            </div>
            {#key `${connectionGeneration}/${selectedId}`}
              {@const generation = connectionGeneration}
              <section
                class="conversation content-arrival"
                aria-label={i18n.t('island.recentMessages')}
                aria-busy={messagesLoading}
              >
                {#each permissions as permission (`${permission.sessionId}/${permission.id}`)}
                  <article class="request-card">
                    <h3 class="m-0 text-[11px] font-semibold text-[#e3c787]">{i18n.t('island.permission')}</h3>
                    <p class="my-2 whitespace-pre-wrap break-words text-xs font-medium">{permission.action}</p>
                    {#if permission.message}
                      <p class="my-2 whitespace-pre-wrap break-words text-xs leading-5 text-[#c6c6cc]">
                        {permission.message}
                      </p>
                    {/if}
                    {#if permission.resources.length}
                      <ul class="my-2 list-none space-y-1 p-0 text-[11px] text-[#b6b6bd]">
                        {#each permission.resources as resource}
                          <li class="whitespace-pre-wrap break-all font-mono">{resource}</li>
                        {/each}
                      </ul>
                    {/if}
                    <div class="expanded-permission-controls mt-3 flex gap-2">
                      <div class="approval-split" class:has-options={!!permission.save?.length}>
                        <button
                          type="button"
                          class="allow-button"
                          disabled={!canReplyPermission(permission)}
                          onclick={() => replyPermission(permission, 'once', generation)}
                        >
                          {i18n.t('island.allowOnce')}
                        </button>
                        {@render approvalOptions(permission, generation)}
                      </div>
                      <button
                        type="button"
                        class="rounded-lg border border-[#35353a] px-3 py-1.5 text-xs disabled:opacity-40"
                        disabled={!canReplyPermission(permission)}
                        onclick={() => replyPermission(permission, 'reject', generation)}
                      >
                        {i18n.t('island.reject')}
                      </button>
                    </div>
                  </article>
                {/each}
                {#each questions as question (`${question.sessionId}/${question.id}`)}
                  <article class="request-card">
                    <QuestionForm
                      {question}
                      busy={!canWrite}
                      onanswer={(answer) => act(() => config.islandReplyQuestion(question.sessionId, question.id, answer, generation), generation)}
                      onskip={() => act(() => config.islandCancelQuestion(question.sessionId, question.id, generation), generation)}
                    />
                  </article>
                {/each}
                {#if messagesLoading && messageSession !== selectedId}
                  <p class="text-xs text-[#b6b6bd]">{i18n.t('common.loading')}</p>
                {/if}
                {#if messageSession === selectedId}
                  {#each messages as message (message.id)}
                    <article class="mb-4">
                      <h3 class="mb-1.5 text-[10px] font-medium text-[#b6b6bd]">
                        {i18n.t(`island.role.${message.role}`)}
                      </h3>
                      <p class="m-0 whitespace-pre-wrap break-words text-xs leading-[1.65] [overflow-wrap:anywhere]">
                        {message.text}
                      </p>
                      {#if message.truncated}
                        <p class="mt-1 text-[10px] text-[#b6b6bd]">{i18n.t('island.truncated')}</p>
                      {/if}
                    </article>
                  {:else}
                    <p class="text-xs text-[#b6b6bd]">{i18n.t('island.noMessages')}</p>
                  {/each}
                {/if}
              </section>
            {/key}
            <form onsubmit={send} class="shrink-0 border-t border-[#262629] px-4 py-3">
              {#if draft?.uncertain}
                <p class="mt-0 mb-2 text-[11px] leading-4 text-[#e3c787]" role="status">
                  {i18n.t(draft.conflict ? 'island.sendConflict' : 'island.sendUncertain')}
                </p>
                {#if draft.refreshed}
                  <p class="mt-0 mb-2 text-[11px] leading-4 text-[#b6b6bd]">{i18n.t('island.attemptNotFound')}</p>
                {/if}
              {/if}
              <label for="island-message" class="sr-only">{i18n.t('island.message')}</label>
              <textarea
                id="island-message"
                rows="2"
                class="block max-h-24 min-h-14 w-full resize-none rounded-xl border border-[#35353a] bg-[#151518] px-3 py-2 text-xs leading-5 placeholder:text-[#94949d] disabled:opacity-60"
                placeholder={i18n.t('island.messagePlaceholder')}
                value={draft?.text ?? ''}
                disabled={busy || draft?.uncertain}
                oninput={(event) => editDraft(event.currentTarget.value)}
              ></textarea>
              <div class="mt-2 flex items-center justify-between gap-2">
                {#if draft?.uncertain && draft.refreshed}
                  <button
                    type="button"
                    disabled={busy}
                    onclick={discardAttempt}
                    class="rounded-lg border border-[#35353a] px-2 py-1.5 text-[11px] disabled:opacity-40"
                  >
                    {i18n.t('island.discardAttempt')}
                  </button>
                {:else}
                  <span class="text-[10px] text-[#b6b6bd]">{i18n.t('island.plainText')}</span>
                {/if}
                <button
                  type="submit"
                  class="flex items-center gap-1.5 rounded-lg bg-[#e9e9ed] px-3 py-1.5 text-xs font-semibold text-black disabled:opacity-40"
                  disabled={!canWrite || !draft?.text.trim() || (draft.uncertain && !draft.refreshed)}
                >
                  <Send size={12} aria-hidden="true" />{i18n.t(draft?.uncertain ? 'island.retrySend' : 'island.send')}
                </button>
              </div>
            </form>
          {:else}
            <p class="m-0 p-5 text-xs leading-5 text-[#b6b6bd]">{i18n.t('island.selectSession')}</p>
          {/if}
        </section>
      </div>
      <footer class="shrink-0 border-t border-[#262629] px-5 py-2 text-[10px] text-[#b6b6bd]">
        {i18n.t('island.closeHint')}
      </footer>
    </div>
  {/if}
</main>

<style>
.island {
  --island-amber: #edc786;
  --island-muted: #97979f;
  --island-ease: cubic-bezier(0.22, 1, 0.36, 1);
  /* Native frames own the size; keep their entire changing viewport opaque. */
  width: 100vw;
  height: 100dvh;
  margin: 0 auto;
  overflow: hidden;
  border-radius: 0 0 18px 18px;
  background: #070708;
  pointer-events: auto;
  color: #f0f0f3;
  color-scheme: dark;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", sans-serif;
  -webkit-font-smoothing: antialiased;
  transition: border-radius 220ms var(--island-ease);
}
.island.compact-attention {
  border-radius: 0 0 20px 20px;
}
.island.expanded {
  display: flex;
  flex-direction: column;
  border-radius: 0 0 24px 24px;
}
.expanded-shell {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  overflow: hidden;
  border-radius: inherit;
  background: #070708;
  pointer-events: auto;
}
.notch-gap {
  height: var(--top-inset);
  flex-shrink: 0;
  background: #070708;
  pointer-events: none;
}
.island.geometry-pending {
  visibility: hidden;
}
.compact-lane {
  display: flex;
  min-width: 0;
  flex: 0 0 56px;
  align-items: center;
  transition: flex-basis 220ms var(--island-ease);
}
.peek .compact-lane {
  flex-basis: 84px;
}
.compact-shell {
  position: relative;
  width: min(var(--compact-width), 100%);
  height: 28px;
  margin: 0 auto;
  overflow: hidden;
  border-radius: 0 0 14px 14px;
  background: #070708;
  box-shadow: inset 0 -1px 0 #ffffff12;
  pointer-events: auto;
  transition: border-radius 220ms var(--island-ease);
}
.compact-attention .compact-shell {
  width: 100%;
  height: 132px;
  border-radius: 0 0 20px 20px;
}
.quick-area {
  display: flex;
  height: 96px;
  box-sizing: border-box;
  flex-direction: column;
  gap: 8px;
  border-top: 1px solid #ffffff10;
  padding: 9px 14px 10px;
  background: #0d0c0a;
  font-size: 11px;
}
.quick-summary {
  display: flex;
  justify-content: space-between;
  color: var(--island-muted);
  font-size: 10px;
}
.quick-area p {
  overflow: hidden;
  margin: 0;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.quick-area p.error {
  color: #f0b3a9;
}
.quick-refresh {
  margin-left: auto;
  flex-shrink: 0;
  align-self: flex-end;
}
.quick-area .quick-refresh {
  margin-top: auto;
}
.session-disclosure {
  margin-right: 8px;
  flex-shrink: 0;
}
.compact-header {
  display: flex;
  height: 28px;
  flex-shrink: 0;
  align-items: center;
  transition: height 220ms var(--island-ease);
}
.compact {
  display: flex;
  min-width: 0;
  height: 28px;
  flex: 1;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border: 0;
  padding: 0 10px;
  background: transparent;
  color: inherit;
  font-size: 11px;
  cursor: pointer;
  transition:
    height 220ms var(--island-ease),
    background 120ms ease;
}
.compact-attention .compact-header,
.compact-attention .compact {
  height: 36px;
}
.compact:hover {
  background: #ffffff06;
}
.peek .compact-left {
  justify-content: flex-start;
  padding-left: 14px;
}
.unnotched-compact .compact-left {
  justify-content: flex-start;
  padding: 0 8px 0 12px;
}
.unnotched-compact .compact-lane {
  flex-basis: 36px;
}
.unnotched-compact .compact-status-control {
  padding: 0;
}
.compact-summary {
  display: flex;
  min-width: 0;
  max-width: 100%;
  flex-direction: column;
  align-items: flex-start;
  gap: 1px;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.compact-primary,
.compact-secondary {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
}
.compact-primary {
  color: #e3e3e8;
  font-size: 10px;
  font-weight: 550;
  line-height: 12px;
}
.compact-secondary {
  color: #b4b4be;
  font-size: 9px;
  line-height: 11px;
}
.compact-wing-state {
  display: flex;
  align-items: center;
  flex-direction: column;
  gap: 1px;
}
.wing-label {
  color: #b4b4be;
  font-size: 9px;
  line-height: 10px;
  white-space: nowrap;
}
.wing-count {
  color: #e3e3e8;
  font-size: 12px;
  font-weight: 600;
  line-height: 14px;
  font-variant-numeric: tabular-nums;
}
.compact-activity-row {
  display: flex;
  align-items: center;
  gap: 4px;
}
.wing-pending-count {
  color: #e3e3e8;
  font-size: 11px;
  font-weight: 600;
  line-height: 14px;
  font-variant-numeric: tabular-nums;
}
.peek-status {
  overflow: hidden;
  color: var(--island-muted);
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.compact-status {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 8px;
  color: var(--island-muted);
}
.activity {
  display: flex;
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  gap: 2px;
  color: #66666f;
}
.island[data-state="unavailable"] .activity {
  color: #b6a187;
}
.activity i {
  width: 3px;
  height: 3px;
  flex-shrink: 0;
  border-radius: 2px;
  background: currentColor;
}
.activity:not(.running) i:nth-child(2),
.activity:not(.running) i:nth-child(3) {
  height: 6px;
}
.activity.running {
  color: #b7c8c1;
}
.activity.pending {
  color: var(--island-amber);
}
.activity.running i {
  height: 16px;
  animation: activity 1.2s ease-in-out infinite;
  transform: scaleY(0.35);
}
.activity.running i:nth-child(2) {
  animation-delay: -0.9s;
}
.activity.running i:nth-child(3) {
  animation-delay: -0.6s;
}
.activity.running i:nth-child(4) {
  animation-delay: -0.3s;
}
.compact-permission {
  position: relative;
  display: flex;
  height: 96px;
  box-sizing: border-box;
  flex-direction: column;
  gap: 6px;
  border-top: 1px solid #ffffff10;
  padding: 9px 14px 10px;
  background: #0d0c0a;
  pointer-events: auto;
}
.permission-context {
  display: flex;
  min-width: 0;
  width: calc(100% - 138px);
  height: 13px;
  flex-shrink: 0;
  align-items: center;
  gap: 6px;
  color: var(--island-amber);
  font-size: 10px;
}
.compact-shell:is(:hover, :focus-within) .permission-context {
  color: #f2d5a3;
}
.permission-context > :global(svg) {
  flex-shrink: 0;
}
.permission-origin,
.permission-error {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.permission-error {
  color: #f0b3a9;
}
.origin-divider {
  margin: 0 5px;
  color: #80715c;
}
.permission-preview {
  display: flex;
  min-width: 0;
  width: calc(100% - 138px);
  height: 15px;
  flex-shrink: 0;
  align-items: center;
  gap: 8px;
  font-size: 11px;
}
.permission-action {
  max-width: 35%;
  overflow: hidden;
  flex-shrink: 0;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.permission-resource {
  overflow: hidden;
  color: #b5b5be;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.permission-controls,
.permission-decisions,
.permission-queue {
  display: flex;
  align-items: center;
}
.permission-controls {
  justify-content: flex-start;
  gap: 8px;
  margin-top: auto;
}
.permission-queue {
  min-width: 0;
  gap: 2px;
  color: var(--island-muted);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}
.permission-queue > span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.queue-divider {
  margin: 0 3px;
  color: #57575f;
}
.permission-decisions {
  flex-shrink: 0;
  gap: 6px;
}
.approval-split {
  position: absolute;
  top: 9px;
  right: 14px;
  display: flex;
}
.menu-trigger {
  display: grid;
  width: 25px;
  height: 28px;
  place-items: center;
  border: 1px solid #edc786;
  border-left-color: #211a0e33;
  border-radius: 0 8px 8px 0;
  background: var(--island-amber);
  color: #211a0e;
  cursor: pointer;
}
.approval-dropdown {
  position: absolute;
  z-index: 1;
  top: 100%;
  right: 0;
  display: flex;
  width: 320px;
  box-sizing: border-box;
  flex-direction: column;
  gap: 4px;
  border: 1px solid #5a4d37;
  border-radius: 8px;
  padding: 6px;
  background: #171510;
  box-shadow: 0 4px 12px #0006;
}
.compact-permission .approval-dropdown {
  top: -4px;
  right: -4px;
  width: min(372px, calc(100vw - 28px));
  height: 88px;
}
.approval-scope {
  min-height: 0;
  max-height: 110px;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding-right: 20px;
  color: #d6c19f;
  font-size: 10px;
  line-height: 13px;
  overflow-wrap: anywhere;
}
.approval-scope p {
  margin: 0;
}
.save-patterns {
  display: block;
  width: 100%;
  height: 14px;
  min-height: 0;
  resize: none;
  border: 0;
  padding: 0;
  background: transparent;
  color: #f0f0f3;
  font-family: monospace;
  font-size: 10px;
  line-height: 13px;
}
.menu-close {
  position: absolute;
  top: 4px;
  right: 4px;
  display: grid;
  width: 20px;
  height: 20px;
  place-items: center;
  border: 0;
  border-radius: 4px;
  background: #ffffff0d;
  color: #d6c19f;
}
.always-button {
  min-height: 26px;
  flex-shrink: 0;
  border: 1px solid #5a4d37;
  border-radius: 8px;
  padding: 0 10px;
  background: #272116;
  color: #f0d5aa;
  font-size: 11px;
  white-space: nowrap;
  box-shadow: 0 4px 12px #0006;
}
.expanded-permission-controls .approval-split {
  display: contents;
}
.expanded-permission-controls {
  flex-wrap: wrap;
  align-items: flex-start;
}
.expanded-permission-controls .approval-options {
  display: contents;
}
.expanded-permission-controls .menu-trigger {
  margin-left: -8px;
}
.expanded-permission-controls .approval-dropdown {
  position: relative;
  top: auto;
  right: auto;
  left: auto;
  width: 100%;
  flex: 0 0 100%;
  order: 1;
}
.always-button:hover:not(:disabled) {
  background: #382d1d;
}
.queue-button {
  display: grid;
  width: 24px;
  height: 26px;
  place-items: center;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: inherit;
}
.reject-button,
.allow-button {
  height: 28px;
  border: 1px solid #ffffff22;
  border-radius: 8px;
  padding: 0 11px;
  background: #ffffff05;
  color: #dedee3;
  font-size: 11px;
  font-weight: 500;
}
.allow-button {
  border-color: #edc786;
  background: #edc786;
  color: #211a0e;
  font-weight: 650;
}
.approval-split.has-options .allow-button {
  border-radius: 8px 0 0 8px;
}
.allow-button:hover:not(:disabled) {
  background: #f4d7a5;
}
.reject-button:hover:not(:disabled),
.queue-button:hover:not(:disabled) {
  background: #ffffff12;
}
.island :global(button:not(:disabled)) {
  cursor: pointer;
}
.island :global(button:disabled) {
  cursor: default;
}
.island :global(button:focus-visible) {
  outline: 2px solid var(--island-amber);
  outline-offset: -3px;
}
.permission-controls button:disabled {
  opacity: 0.4;
}
.compact:disabled {
  opacity: 0.6;
}
.compact-permission .allow-button:focus-visible {
  outline-color: #211a0e;
}
.pending-badge {
  border-radius: 6px;
  background: #edc78613;
  padding: 2px 6px;
  color: var(--island-amber);
  font-size: 10px;
  white-space: nowrap;
}
.icon-button {
  display: grid;
  width: 28px;
  height: 28px;
  place-items: center;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: #b6b6bd;
}
.icon-button:hover {
  background: #202023;
  color: #ededf0;
}
.icon-button:disabled {
  opacity: 0.4;
}
.island-content {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.session-list {
  width: 150px;
  flex-shrink: 0;
  overflow-y: auto;
  border-right: 1px solid #262629;
  padding: 6px;
}
.session {
  display: block;
  width: 100%;
  border: 1px solid transparent;
  border-radius: 10px;
  padding: 9px;
  background: transparent;
  color: inherit;
  text-align: left;
  transition:
    background 180ms var(--island-ease),
    border-color 180ms var(--island-ease);
}
.session:hover {
  background: #151518;
}
.session[aria-current="true"] {
  border-color: #35353a;
  background: #1b1b1e;
}
.attention {
  color: #e3c787;
}
.conversation {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 12px 16px;
}
.request-card {
  margin-bottom: 12px;
  border: 1px solid #51452d;
  border-radius: 12px;
  background: #171510;
  padding: 12px;
}
.notched-compact .compact-shell {
  height: var(--compact-height);
  box-shadow: none;
}
.notched-compact .compact-header {
  height: var(--compact-height);
}
.notched-compact .compact-left {
  width: calc((var(--idle-width) - var(--notch-width)) / 2);
  height: var(--compact-height);
  flex: 0 0 calc((var(--idle-width) - var(--notch-width)) / 2);
  justify-content: center;
  border-radius: 0 0 0 14px;
  padding: 0;
  background: #070708;
  pointer-events: auto;
}
.notched-compact .notch-gap {
  width: var(--notch-width);
  height: 100%;
  flex: 0 0 var(--notch-width);
}
.island:not(.peek):not(.expanded) > .notch-gap {
  display: none;
}
.notched-compact .compact-lane {
  height: 100%;
  flex: 0 0 calc((var(--idle-width) - var(--notch-width)) / 2);
  overflow: hidden;
  border-radius: 0 0 14px 0;
  background: #070708;
  pointer-events: auto;
}
.notched-compact .compact-lane .compact {
  height: var(--compact-height);
  justify-content: center;
  gap: 0;
  padding: 0 1px;
}
.notched-compact .compact-status {
  flex-direction: column;
  gap: 0;
}
@keyframes activity {
  0%,
  100% {
    transform: scaleY(0.3);
  }
  50% {
    transform: scaleY(1);
  }
}
.content-arrival {
  animation: content-arrival 180ms var(--island-ease);
}
.conversation.content-arrival,
.session-list.content-arrival {
  animation-duration: 220ms;
}
.peek .compact-status,
.session-disclosure {
  animation: content-arrival 180ms var(--island-ease);
}
@keyframes content-arrival {
  from {
    opacity: 0.72;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
@media (prefers-reduced-motion: reduce) {
  .island,
  .compact-shell,
  .compact-header,
  .compact-lane,
  .compact,
  .session {
    transition: none;
  }
  .content-arrival,
  .peek .compact-status,
  .session-disclosure,
  .activity.running i {
    animation: none;
  }
  .activity.running i:nth-child(2),
  .activity.running i:nth-child(4) {
    transform: scaleY(0.75);
  }
}
@media (max-width: 430px) {
  .session-list {
    width: 120px;
  }
  .conversation {
    padding: 10px;
  }
}
</style>
