import { createResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Role } from '@/core/domain/types/api';

const api = createTauriClient();

export const useRoleStore = createResourceStore<Role & { id: string }>(
  'role',
  async () => {
    const data = await api.listRoles();
    return data.roles as (Role & { id: string })[];
  }
);

// Convenience hooks
export const useRoles = () => useRoleStore((state) => state.items);
export const useRolesLoading = () => useRoleStore((state) => state.isLoading);
export const useRoleById = (id: string) =>
  useRoleStore((state) => state.items.find((r) => r.id === id));
