import type { ModelDef, ModelModality, ModelVariant } from '$src/types/models.js';
import { prettyJson, safeParseJsonObject } from '$src/utils/index.js';

export type VariantRow = { key: string; json: string };

export type ModelFormValues = {
  providerId: string;
  id: string;
  family: string;
  disabled: boolean;
  tools: boolean;
  costInput: string;
  costOutput: string;
  costCacheRead: string;
  costCacheWrite: string;
  limitContext: string;
  limitOutput: string;
  limitInput: string;
  modalityInput: Record<ModelModality, boolean>;
  modalityOutput: Record<ModelModality, boolean>;
  settings: string;
  headers: string;
  variants: VariantRow[];
};

export const MODALITIES: ModelModality[] = ['text', 'audio', 'image', 'video', 'pdf'];
export const emptyVariant = (): VariantRow => ({ key: '', json: '{}' });
const numStr = (value: number | null | undefined) => (value == null ? '' : String(value));
export const initMods = (values?: ModelModality[]) => {
  const selected: Record<ModelModality, boolean> = {
    text: false,
    audio: false,
    image: false,
    video: false,
    pdf: false,
  };
  for (const value of values ?? []) selected[value] = true;
  return selected;
};

export const parseObject = (text: string): Record<string, unknown> | undefined =>
  !text.trim() ? {} : (safeParseJsonObject(text) ?? undefined);
export const anyFilled = (...values: string[]) => values.some((value) => value.trim() !== '');

export function modelToFormValues(initial: ModelDef, providerId: string, defaults: ModelFormValues): ModelFormValues {
  const rows = initial.variants ?? [];
  return {
    ...defaults,
    providerId,
    id: initial.id,
    family: initial.family ?? '',
    disabled: initial.disabled === true,
    tools: initial.capabilities?.tools ?? false,
    costInput: numStr(initial.cost?.input),
    costOutput: numStr(initial.cost?.output),
    costCacheRead: numStr(initial.cost?.cache?.read),
    costCacheWrite: numStr(initial.cost?.cache?.write),
    limitContext: numStr(initial.limit?.context),
    limitOutput: numStr(initial.limit?.output),
    limitInput: numStr(initial.limit?.input),
    modalityInput: initMods(initial.capabilities?.input),
    modalityOutput: initMods(initial.capabilities?.output),
    settings: prettyJson(initial.settings),
    headers: prettyJson(initial.headers),
    variants: rows.length
      ? rows.map((entry) => {
          const { id: variantId, ...rest } = entry ?? {};
          return { key: variantId ?? '', json: prettyJson(rest) };
        })
      : [emptyVariant()],
  };
}

export function formValuesToModel(value: ModelFormValues, initial?: ModelDef): ModelDef {
  const {
    id,
    family,
    disabled,
    tools,
    costInput,
    costOutput,
    costCacheRead,
    costCacheWrite,
    limitContext,
    limitOutput,
    limitInput,
    modalityInput,
    modalityOutput,
    settings,
    headers,
    variants,
  } = value;
  const settingsValue = parseObject(settings);
  const headersValue = parseObject(headers);
  const variantEntries = variants
    .filter((row) => row.key.trim())
    .map((row) => ({ key: row.key.trim(), entry: parseObject(row.json) ?? {} }));
  const costUsed = anyFilled(costInput, costOutput, costCacheRead, costCacheWrite);
  const limitUsed = anyFilled(limitContext, limitOutput, limitInput);
  const inputModalities = MODALITIES.filter((modality) => modalityInput[modality]);
  const outputModalities = MODALITIES.filter((modality) => modalityOutput[modality]);
  const cacheUsed = anyFilled(costCacheRead, costCacheWrite);
  const capabilitiesNeeded =
    initial?.capabilities != null || tools || inputModalities.length > 0 || outputModalities.length > 0;
  return {
    id: id.trim(),
    ...(family.trim() && { family: family.trim() }),
    ...((disabled || initial?.disabled != null) && { disabled }),
    ...(capabilitiesNeeded && {
      capabilities: {
        tools,
        ...((inputModalities.length || initial?.capabilities?.input != null) && { input: inputModalities }),
        ...((outputModalities.length || initial?.capabilities?.output != null) && { output: outputModalities }),
      },
    }),
    ...(costUsed && {
      cost: {
        ...(costInput.trim() && { input: Number(costInput) }),
        ...(costOutput.trim() && { output: Number(costOutput) }),
        ...(cacheUsed && {
          cache: {
            ...(costCacheRead.trim() && { read: Number(costCacheRead) }),
            ...(costCacheWrite.trim() && { write: Number(costCacheWrite) }),
          },
        }),
      },
    }),
    ...(limitUsed && {
      limit: {
        ...(limitContext.trim() && { context: Number(limitContext) }),
        ...(limitOutput.trim() && { output: Number(limitOutput) }),
        ...(limitInput.trim() && { input: Number(limitInput) }),
      },
    }),
    ...(settingsValue && Object.keys(settingsValue).length && { settings: settingsValue }),
    ...(headersValue && Object.keys(headersValue).length && { headers: headersValue as Record<string, string> }),
    ...(variantEntries.length && {
      variants: variantEntries.map(({ key, entry }) => ({ id: key, ...entry }) as ModelVariant),
    }),
  };
}
