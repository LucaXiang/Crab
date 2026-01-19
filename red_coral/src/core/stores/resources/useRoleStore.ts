import { createResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Role } from '@/infrastructure/api/types';

const api = createTauriClient();

async function fetchRoles(): Promise<Role[]> {
  const response = await api.listRoles();
  if (response.data?.roles) {
    return response.data.roles;
  }
  throw new Error(response.message || 'Failed to fetch roles');
}

export const useRoleStore = createResourceStore<Role & { id: string }>(
  'role',
  fetchRoles as () => Promise<(Role & { id: string })[]>
);

// Convenience hooks
export const useRoles = () => useRoleStore((state) => state.items);
export const useRolesLoading = () => useRoleStore((state) => state.isLoading);
export const useRoleById = (id: string) =>
  useRoleStore((state) => state.items.find((r) => r.id === id));
