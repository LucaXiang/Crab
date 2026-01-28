import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Employee } from '@/core/domain/types/api';

// Employee.id is optional, but createResourceStore requires { id: string }
type EmployeeEntity = Employee & { id: string };

const getApi = () => createTauriClient();

export const useEmployeeStore = createResourceStore<EmployeeEntity>(
  'employee',
  async () => {
    const employees = await getApi().listEmployees();
    return employees as EmployeeEntity[];
  }
);

// Convenience hooks
export const useEmployees = () => useEmployeeStore((state) => state.items);
export const useEmployeesLoading = () => useEmployeeStore((state) => state.isLoading);
export const useEmployeeById = (id: string) =>
  useEmployeeStore((state) => state.items.find((e) => e.id === id));
