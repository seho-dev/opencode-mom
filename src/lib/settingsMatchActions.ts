import {
  countExactModelMatches,
  emptyMatchCounts,
  matchCountTotal,
  replaceExactModelMatches,
  type ExactModelMatchCounts
} from './settingsGroupDraft';
import type { ModelGroup } from './contracts';
import { success, warning, type SettingsFacts } from './settingsState';

type MatchActionContext = Readonly<{
  readFacts: () => SettingsFacts;
  updateFacts: (updater: (facts: SettingsFacts) => SettingsFacts) => void;
  advanceContext: () => void;
}>;

export type SettingsMatchActions = Readonly<{
  setMatchSearch: (value: string) => Promise<void>;
  setMatchReplace: (value: string) => void;
  replaceExactMatches: () => Promise<void>;
}>;

export const matchCountsForDraft = (
  matchSearch: string,
  draftGroup: ModelGroup | null
): ExactModelMatchCounts => draftGroup
  ? countExactModelMatches(matchSearch, draftGroup)
  : emptyMatchCounts;

export const createSettingsMatchActions = ({
  readFacts,
  updateFacts,
  advanceContext
}: MatchActionContext): SettingsMatchActions => {
  const setMatchSearch = async (matchSearch: string): Promise<void> => {
    updateFacts((facts) => ({
      ...facts,
      matchSearch,
      matchCounts: matchCountsForDraft(matchSearch, facts.draftGroup)
    }));
  };

  const replaceExactMatches = async (): Promise<void> => {
    const facts = readFacts();
    const selected = facts.draftGroup;
    if (!selected) {
      updateFacts((value) => ({ ...value, matchCounts: emptyMatchCounts, message: warning('Select a group first.') }));
      return;
    }
    const replaced = replaceExactModelMatches(facts.matchSearch, facts.matchReplace, selected);
    if (matchCountTotal(replaced.matchCounts) === 0) {
      updateFacts((value) => ({ ...value, matchCounts: replaced.matchCounts }));
      return;
    }
    advanceContext();
    updateFacts((value) => ({
      ...value,
      draftGroup: replaced.group,
      matchCounts: matchCountsForDraft(value.matchSearch, replaced.group),
      message: success(`Replaced ${matchCountTotal(replaced.matchCounts)} exact model references.`)
    }));
  };

  return {
    setMatchSearch,
    setMatchReplace: (matchReplace) => updateFacts((facts) => ({ ...facts, matchReplace })),
    replaceExactMatches
  };
};
