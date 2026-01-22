import { createResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { EmployeeResponse } from '@/core/domain/types/api';

const api = createTauriClient();

export const useEmployeeStore = createResourceStore<EmployeeResponse>(
  'employee',
  () => api.listEmployees()
);

// Convenience hooks
export const useEmployees = () => useEmployeeStore((state) => state.items);
export const useEmployeesLoading = () => useEmployeeStore((state) => state.isLoading);
export const useEmployeeById = (id: string) =>
  useEmployeeStore((state) => state.items.find((e) => e.id === id));
