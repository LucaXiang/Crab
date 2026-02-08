import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { useShallow } from 'zustand/react/shallow';
import { createTauriClient } from '@/infrastructure/api';
import type { PriceRule } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

export const usePriceRuleStore = createResourceStore<PriceRule>(
  'price_rule',
  () => getApi().listPriceRules()
);

// Convenience hooks
export const usePriceRules = () => usePriceRuleStore((state) => state.items);
export const usePriceRulesLoading = () => usePriceRuleStore((state) => state.isLoading);
export const usePriceRuleById = (id: number) =>
  usePriceRuleStore((state) => state.items.find((r) => r.id === id));
export const useActivePriceRules = () =>
  usePriceRuleStore(
    useShallow((state) => state.items.filter((r) => r.is_active))
  );
