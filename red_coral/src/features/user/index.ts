/**
 * User Feature Module
 *
 * 用户管理功能模块，包含用户 CRUD 操作和员工相关功能。
 */

// Store
export {
  useEmployeeStore,
  useEmployees,
  useEmployeesLoading,
  useEmployeeById,
} from './store';

// Mutations
export {
  fetchUsers,
  createUser,
  updateUser,
  deleteUser,
  resetPassword,
  fetchRoles,
  disableUser,
  enableUser,
} from './mutations';

// Components
export { UserManagement } from './UserManagement';
export { UserFormModal } from './UserFormModal';
export { ResetPasswordModal } from './ResetPasswordModal';
