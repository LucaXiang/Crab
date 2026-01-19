/**
 * Permission checking hooks
 *
 * Provides convenient hooks for checking user permissions and roles
 * throughout the application.
 */

import { Permission as PermissionValues } from '@/core/domain/types';
import { useHasPermission, useHasRole } from '@/core/stores/auth/useAuthStore';

/**
 * General permission hook
 * Returns both hasPermission and hasRole functions
 *
 * @example
 * const { hasPermission, hasRole } = usePermission();
 * if (hasPermission(Permission.VOID_ORDER)) {
 *   // Show void order button
 * }
 */
export const usePermission = () => {
  const hasPermission = useHasPermission();
  const hasRole = useHasRole();

  return { hasPermission, hasRole };
};

// ==================== Specific Permission Hooks ====================

/**
 * Check if user can manage other users (create/edit/delete)
 * Admin only
 */
export const useCanManageUsers = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.MANAGE_USERS);
};

/**
 * Check if user can manage products (create/edit/delete)
 * Manager and Admin only
 */
export const useCanManageProducts = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.CREATE_PRODUCT);
};

/**
 * Check if user can update products
 * Manager and Admin only
 */
export const useCanUpdateProduct = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.UPDATE_PRODUCT);
};

/**
 * Check if user can delete products
 * Manager and Admin only
 */
export const useCanDeleteProduct = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.DELETE_PRODUCT);
};

/**
 * Check if user can manage categories
 * Manager and Admin only
 */
export const useCanManageCategories = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.MANAGE_CATEGORIES);
};

/**
 * Check if user can manage zones
 * Manager and Admin only
 */
export const useCanManageZones = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.MANAGE_ZONES);
};

/**
 * Check if user can manage tables
 * Manager and Admin only
 */
export const useCanManageTables = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.MANAGE_TABLES);
};
