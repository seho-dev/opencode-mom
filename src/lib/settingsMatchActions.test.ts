import { describe, expect, it } from 'vitest';

import type { ModelGroup } from './contracts';
import { countExactModelMatches, replaceExactModelMatches, trimExactModelValue } from './settingsGroupDraft';

const group: ModelGroup = {
  id: '11111111-1111-4111-8111-111111111111',
  name: 'Match fixture',
  description: null,
  categoryMappings: [
    { categoryName: 'build', modelRef: ' shared/model ' },
    { categoryName: 'case-sensitive', modelRef: 'Shared/Model' },
    { categoryName: 'other', modelRef: 'other/model' }
  ],
  agentOverrides: [
    { agentName: 'planner', modelRef: 'shared/model' },
    { agentName: 'reviewer', modelRef: 'other/model' }
  ],
  openCodeAgentOverrides: [
    { agentName: 'writer', modelRef: 'shared/model' },
    { agentName: 'tester', modelRef: ' shared/model ' },
    { agentName: 'legacy', modelRef: 'other/model' }
  ],
  isEnabled: true,
  updatedAt: '2026-07-09T00:00:00Z'
};

describe('settings exact model match actions', () => {
  it('counts trimmed, case-sensitive exact matches independently in every mapping section', () => {
    expect(countExactModelMatches('  shared/model  ', group)).toEqual({
      categoryMappings: 1,
      agentOverrides: 1,
      openCodeAgentOverrides: 2
    });
    expect(countExactModelMatches('Shared/Model', group)).toEqual({
      categoryMappings: 1,
      agentOverrides: 0,
      openCodeAgentOverrides: 0
    });
  });

  it('keeps the draft unchanged and reports zero matches for an empty trimmed search', () => {
    const result = replaceExactModelMatches('   ', ' next/model ', group);

    expect(result.matchCounts).toEqual({
      categoryMappings: 0,
      agentOverrides: 0,
      openCodeAgentOverrides: 0
    });
    expect(result.group).toBe(group);
  });

  it('trims replacements, clears matched values for an empty replacement, and preserves nonmatches', () => {
    const replacement = replaceExactModelMatches(' shared/model ', '  next/model  ', group);

    expect(replacement.matchCounts).toEqual({
      categoryMappings: 1,
      agentOverrides: 1,
      openCodeAgentOverrides: 2
    });
    expect(replacement.group.categoryMappings.map((mapping) => mapping.modelRef)).toEqual([
      'next/model',
      'Shared/Model',
      'other/model'
    ]);
    expect(replacement.group.agentOverrides.map((override) => override.modelRef)).toEqual([
      'next/model',
      'other/model'
    ]);
    expect(replacement.group.openCodeAgentOverrides.map((override) => override.modelRef)).toEqual([
      'next/model',
      'next/model',
      'other/model'
    ]);

    const cleared = replaceExactModelMatches('shared/model', '   ', group);
    expect(cleared.group.categoryMappings[0]?.modelRef).toBe('');
    expect(cleared.group.agentOverrides[0]?.modelRef).toBe('');
    expect(cleared.group.openCodeAgentOverrides.map((override) => override.modelRef)).toEqual([
      '',
      '',
      'other/model'
    ]);
  });

  it('uses Rust whitespace semantics for exact model values, including U+0085 and U+FEFF', () => {
    const groupWith = (modelRef: string): ModelGroup => ({
      ...group,
      categoryMappings: [{ categoryName: 'build', modelRef }],
      agentOverrides: [{ agentName: 'planner', modelRef }],
      openCodeAgentOverrides: [{ agentName: 'reviewer', modelRef }]
    });
    const expectedCounts = { categoryMappings: 1, agentOverrides: 1, openCodeAgentOverrides: 1 };

    expect(trimExactModelValue(' \tshared/model\r\n')).toBe('shared/model');
    expect(trimExactModelValue('\u0085shared/model\u0085')).toBe('shared/model');
    const nelReplacement = replaceExactModelMatches(
      '\u0085shared/model\u0085',
      '\u0085next/model\u0085',
      groupWith('\u0085shared/model\u0085')
    );
    expect(nelReplacement.matchCounts).toEqual(expectedCounts);
    expect(nelReplacement.group.categoryMappings[0]?.modelRef).toBe('next/model');
    expect(nelReplacement.group.agentOverrides[0]?.modelRef).toBe('next/model');
    expect(nelReplacement.group.openCodeAgentOverrides[0]?.modelRef).toBe('next/model');

    const bomWrapped = '\uFEFFshared/model\uFEFF';
    expect(trimExactModelValue(bomWrapped)).toBe(bomWrapped);
    const bomGroup = groupWith(bomWrapped);
    expect(countExactModelMatches('shared/model', bomGroup)).toEqual({
      categoryMappings: 0,
      agentOverrides: 0,
      openCodeAgentOverrides: 0
    });
    const bomReplacement = replaceExactModelMatches(bomWrapped, '\uFEFFnext/model\uFEFF', bomGroup);
    expect(bomReplacement.matchCounts).toEqual(expectedCounts);
    expect(bomReplacement.group.categoryMappings[0]?.modelRef).toBe('\uFEFFnext/model\uFEFF');
    expect(bomReplacement.group.agentOverrides[0]?.modelRef).toBe('\uFEFFnext/model\uFEFF');
    expect(bomReplacement.group.openCodeAgentOverrides[0]?.modelRef).toBe('\uFEFFnext/model\uFEFF');
  });
});
