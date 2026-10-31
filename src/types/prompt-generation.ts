import type { PROMPT_GENERATION_KINDS, PROMPT_GENERATION_PRESETS } from '$src/utils/constants.js';
import type { ModelRef } from './models.js';

export { PROMPT_GENERATION_PRESETS } from '$src/utils/constants.js';

export type PromptGenerationKind = (typeof PROMPT_GENERATION_KINDS)[number];
export type PromptGenerationPreset = (typeof PROMPT_GENERATION_PRESETS)[number];

export interface PromptGenerationRequest {
  kind: PromptGenerationKind;
  preset: PromptGenerationPreset;
  instructions: string;
  currentPrompt: string;
  model?: ModelRef;
  variant?: string;
}
