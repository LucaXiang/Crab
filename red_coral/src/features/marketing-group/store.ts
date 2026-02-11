import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { listMarketingGroups } from './mutations';
import type { MarketingGroup } from '@/core/domain/types/api';

export const useMarketingGroupStore = createResourceStore<MarketingGroup>(
  'marketing_group',
  listMarketingGroups
);

export const useMarketingGroups = () => useMarketingGroupStore((s) => s.items);
export const useMarketingGroupsLoading = () => useMarketingGroupStore((s) => s.isLoading);
