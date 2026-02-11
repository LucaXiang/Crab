import { invokeApi } from '@/infrastructure/api/tauri-client';
import type {
  MemberWithGroup,
  MemberCreate,
  MemberUpdate,
} from '@/core/domain/types/api';

export async function listMembers(): Promise<MemberWithGroup[]> {
  return invokeApi<MemberWithGroup[]>('api_get', { path: '/api/members' });
}

export async function searchMembers(query: string): Promise<MemberWithGroup[]> {
  return invokeApi<MemberWithGroup[]>('api_get', {
    path: `/api/members/search?q=${encodeURIComponent(query)}`,
  });
}

export async function createMember(data: MemberCreate): Promise<MemberWithGroup> {
  return invokeApi<MemberWithGroup>('api_post', { path: '/api/members', body: data });
}

export async function updateMember(id: number, data: MemberUpdate): Promise<MemberWithGroup> {
  return invokeApi<MemberWithGroup>('api_put', { path: `/api/members/${id}`, body: data });
}

export async function deleteMember(id: number): Promise<void> {
  await invokeApi<void>('api_delete', { path: `/api/members/${id}` });
}
