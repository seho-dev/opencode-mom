<script lang="ts">
import { getI18n } from '$src/i18n/context.js';
import type { IslandAnswer, IslandQuestion, IslandQuestionField } from '$src/types/island.js';

let {
  question,
  busy,
  onanswer,
  onskip,
}: {
  question: IslandQuestion;
  busy: boolean;
  onanswer: (answer: IslandAnswer) => void;
  onskip: () => void;
} = $props();
const i18n = getI18n();
let answers = $state<IslandAnswer>({});
let custom = $state<Record<string, boolean>>({});
let customText = $state<Record<string, string>>({});
let invalid = $state(false);
const answerable = $derived(
  question.answerable && question.fields.every((field) => field.type === 'string' || field.type === 'multiselect'),
);

function choices(field: IslandQuestionField): string[] {
  const value = answers[field.key];
  return Array.isArray(value) ? value : [];
}
function toggle(field: IslandQuestionField, value: string, checked: boolean) {
  answers[field.key] = checked ? [...choices(field), value] : choices(field).filter((entry) => entry !== value);
}
function addCustom(field: IslandQuestionField) {
  const value = (customText[field.key] ?? '').trim();
  if (!value) return;
  answers[field.key] = [...new Set([...choices(field), value])];
  customText[field.key] = '';
}
function submit(event: SubmitEvent) {
  event.preventDefault();
  if (busy || !answerable) return;
  const result: IslandAnswer = {};
  for (const field of question.fields) {
    const value =
      field.type === 'multiselect'
        ? choices(field)
        : custom[field.key]
          ? (customText[field.key] ?? '')
          : typeof answers[field.key] === 'string'
            ? (answers[field.key] as string)
            : '';
    if (field.required && (Array.isArray(value) ? value.length === 0 : !value.trim())) {
      invalid = true;
      return;
    }
    result[field.key] = Array.isArray(value) ? [...value] : value;
  }
  invalid = false;
  onanswer(result);
}
</script>

<form onsubmit={submit} class="space-y-3">
  <h3 class="m-0 text-[13px] font-semibold">{question.title}</h3>
  {#if !answerable}
    <p class="m-0 text-xs leading-5 text-[#b6b6bd]">{i18n.t('island.questionReadOnly')}</p>
  {/if}
  {#each question.fields as field (field.key)}
    <fieldset disabled={busy || !answerable} class="m-0 min-w-0 space-y-2 border-0 p-0">
      <legend class="mb-1 text-xs font-medium">
        {field.title ?? field.key}{field.required ? ` · ${i18n.t('island.required')}` : ''}
      </legend>
      {#if field.description}
        <p class="m-0 text-xs leading-5 text-[#b6b6bd]">{field.description}</p>
      {/if}
      {#each field.options as option (option.value)}
        {@const inputId = `${question.id}-${field.key}-${option.value}`}
        <label for={inputId} class="flex cursor-pointer items-start gap-2 text-xs leading-5">
          {#if field.type === 'multiselect'}
            <input
              id={inputId}
              type="checkbox"
              class="mt-1 accent-[#d8b66e]"
              checked={choices(field).includes(option.value)}
              onchange={(event) => toggle(field, option.value, event.currentTarget.checked)}
            >
          {:else}
            <input
              id={inputId}
              type="radio"
              name={`${question.id}-${field.key}`}
              class="mt-1 accent-[#d8b66e]"
              checked={!custom[field.key] && answers[field.key] === option.value}
              onchange={() => { custom[field.key] = false; answers[field.key] = option.value; }}
            >
          {/if}
          <span
            >{option.label}
            {#if option.description}
              <span class="block text-[#b6b6bd]">{option.description}</span>
            {/if}</span
          >
        </label>
      {/each}
      {#if field.custom}
        {#if field.type === 'string'}
          <label class="flex items-center gap-2 text-xs">
            <input
              type="radio"
              name={`${question.id}-${field.key}`}
              class="accent-[#d8b66e]"
              checked={custom[field.key] ?? false}
              onchange={() => { custom[field.key] = true; }}
            >
            {i18n.t('island.customAnswer')}
          </label>
        {/if}
        <div class="flex gap-2">
          <input
            type="text"
            aria-label={i18n.t('island.customFor', { field: field.title ?? field.key })}
            class="min-w-0 flex-1 rounded-lg border border-[#35353a] bg-[#18181b] px-2 py-1.5 text-xs"
            disabled={field.type === 'string' && !custom[field.key]}
            bind:value={customText[field.key]}
          >
          {#if field.type === 'multiselect'}
            <button
              type="button"
              class="rounded-lg border border-[#35353a] px-2 text-xs disabled:opacity-40"
              disabled={!customText[field.key]?.trim()}
              onclick={() => addCustom(field)}
            >
              {i18n.t('island.addAnswer')}
            </button>
          {/if}
        </div>
        {#if field.type === 'multiselect'}
          {#each choices(field).filter((value) => !field.options.some((option) => option.value === value)) as value (value)}
            <label class="flex items-start gap-2 text-xs leading-5"
              ><input
                type="checkbox"
                checked
                class="mt-1 accent-[#d8b66e]"
                onchange={() => toggle(field, value, false)}
              ><span class="break-all">{value}</span></label
            >
          {/each}
        {/if}
      {/if}
    </fieldset>
  {/each}
  {#if invalid}
    <p role="alert" class="m-0 text-xs text-[#f0b3a9]">{i18n.t('island.answerRequired')}</p>
  {/if}
  <div class="flex gap-2">
    {#if answerable}
      <button
        type="submit"
        disabled={busy}
        class="rounded-lg bg-[#e9e9ed] px-3 py-1.5 text-xs font-semibold text-black disabled:opacity-40"
      >
        {i18n.t('island.submitAnswer')}
      </button>
    {/if}
    <button
      type="button"
      disabled={busy}
      onclick={onskip}
      class="rounded-lg border border-[#35353a] px-3 py-1.5 text-xs disabled:opacity-40"
    >
      {i18n.t('island.skip')}
    </button>
  </div>
</form>
