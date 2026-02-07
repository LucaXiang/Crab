import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Employee } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

export const useEmployeeStore = createResourceStore<Employee>(
  'employee',
  async () => {
    return await getApi().listEmployees();
  }
);

// Convenience hooks
export const useEmployees = () => useEmployeeStore((state) => state.items);
export const useEmployeesLoading = () => useEmployeeStore((state) => state.isLoading);
export const useEmployeeById = (id: number) =>
  useEmployeeStore((state) => state.items.find((e) => e.id === id));
