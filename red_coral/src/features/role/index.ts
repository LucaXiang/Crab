/**
 * Role Feature Module
 *
 * 角色管理功能模块，包含：
 * - 角色 Store (useRoleStore)
 * - 角色权限编辑器组件 (RolePermissionsEditor)
 */

// Store
export {
  useRoleStore,
  useRoles,
  useRolesLoading,
  useRoleById,
} from './store';

// Components
export { RolePermissionsEditor } from './RolePermissionsEditor';
