import { createResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { EmployeeResponse } from '@/core/domain/types/api';

const api = createTauriClient();

async function fetchEmployees(): Promise<EmployeeResponse[]> {
  const response = await api.listEmployees();
  if (Array.isArray(response)) {
    return response as EmployeeResponse[];
  }
  if (response.data?.employees) {
    return response.data.employees as EmployeeResponse[];
  }
  throw new Error(response.message || 'Failed to fetch employees');
}

export const useEmployeeStore = createResourceStore<EmployeeResponse>(
  'employee',
  fetchEmployees
);

// Convenience hooks
export const useEmployees = () => useEmployeeStore((state) => state.items);
export const useEmployeesLoading = () => useEmployeeStore((state) => state.isLoading);
export const useEmployeeById = (id: string) =>
  useEmployeeStore((state) => state.items.find((e) => e.id === id));
