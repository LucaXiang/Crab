import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { listMembers } from './mutations';
import type { MemberWithGroup } from '@/core/domain/types/api';

export const useMemberStore = createResourceStore<MemberWithGroup>(
  'member',
  listMembers
);

export const useMembers = () => useMemberStore((s) => s.items);
export const useMembersLoading = () => useMemberStore((s) => s.isLoading);
