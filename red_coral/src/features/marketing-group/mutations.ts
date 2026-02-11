import { invokeApi } from '@/infrastructure/api/tauri-client';
import type {
  MarketingGroup,
  MarketingGroupCreate,
  MarketingGroupUpdate,
  MarketingGroupDetail,
  MgDiscountRule,
  MgDiscountRuleCreate,
  MgDiscountRuleUpdate,
  StampActivityDetail,
  StampActivityCreate,
  StampActivityUpdate,
} from '@/core/domain/types/api';

// ============ Marketing Groups ============

export async function listMarketingGroups(): Promise<MarketingGroup[]> {
  return invokeApi<MarketingGroup[]>('api_get', { path: '/api/marketing-groups' });
}

export async function getMarketingGroupDetail(id: number): Promise<MarketingGroupDetail> {
  return invokeApi<MarketingGroupDetail>('api_get', { path: `/api/marketing-groups/${id}` });
}

export async function createMarketingGroup(data: MarketingGroupCreate): Promise<MarketingGroup> {
  return invokeApi<MarketingGroup>('api_post', { path: '/api/marketing-groups', body: data });
}

export async function updateMarketingGroup(id: number, data: MarketingGroupUpdate): Promise<MarketingGroup> {
  return invokeApi<MarketingGroup>('api_put', { path: `/api/marketing-groups/${id}`, body: data });
}

export async function deleteMarketingGroup(id: number): Promise<void> {
  await invokeApi<void>('api_delete', { path: `/api/marketing-groups/${id}` });
}

// ============ Discount Rules ============

export async function createDiscountRule(groupId: number, data: MgDiscountRuleCreate): Promise<MgDiscountRule> {
  return invokeApi<MgDiscountRule>('api_post', {
    path: `/api/marketing-groups/${groupId}/discount-rules`,
    body: data,
  });
}

export async function updateDiscountRule(groupId: number, ruleId: number, data: MgDiscountRuleUpdate): Promise<MgDiscountRule> {
  return invokeApi<MgDiscountRule>('api_put', {
    path: `/api/marketing-groups/${groupId}/discount-rules/${ruleId}`,
    body: data,
  });
}

export async function deleteDiscountRule(groupId: number, ruleId: number): Promise<void> {
  await invokeApi<void>('api_delete', { path: `/api/marketing-groups/${groupId}/discount-rules/${ruleId}` });
}

// ============ Stamp Activities ============

export async function createStampActivity(groupId: number, data: StampActivityCreate): Promise<StampActivityDetail> {
  return invokeApi<StampActivityDetail>('api_post', {
    path: `/api/marketing-groups/${groupId}/stamp-activities`,
    body: data,
  });
}

export async function updateStampActivity(groupId: number, activityId: number, data: StampActivityUpdate): Promise<StampActivityDetail> {
  return invokeApi<StampActivityDetail>('api_put', {
    path: `/api/marketing-groups/${groupId}/stamp-activities/${activityId}`,
    body: data,
  });
}

export async function deleteStampActivity(groupId: number, activityId: number): Promise<void> {
  await invokeApi<void>('api_delete', { path: `/api/marketing-groups/${groupId}/stamp-activities/${activityId}` });
}
