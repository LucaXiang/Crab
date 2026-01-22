import { createResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { PriceRule } from '@/core/domain/types/api';

const api = createTauriClient();

export const usePriceRuleStore = createResourceStore<PriceRule & { id: string }>(
  'price_rule',
  () => api.listPriceRules() as Promise<(PriceRule & { id: string })[]>
);

// Convenience hooks
export const usePriceRules = () => usePriceRuleStore((state) => state.items);
export const usePriceRulesLoading = () => usePriceRuleStore((state) => state.isLoading);
export const usePriceRuleById = (id: string) =>
  usePriceRuleStore((state) => state.items.find((r) => r.id === id));
export const useActivePriceRules = () =>
  usePriceRuleStore((state) => state.items.filter((r) => r.is_active));
