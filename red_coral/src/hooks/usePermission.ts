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
 * Check if user can void orders
 * Manager and Admin only
 */
export const useCanVoidOrder = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.VOID_ORDER);
};

/**
 * Check if user can restore voided orders
 * Manager and Admin only
 */
export const useCanRestoreOrder = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.RESTORE_ORDER);
};

/**
 * Check if user can manage products (create/edit/delete)
 * Manager and Admin only
 */
export const useCanManageProducts = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.CREATE_PRODUCT); // CREATE implies UPDATE/DELETE
};

/**
 * Check if user can create products
 * Manager and Admin only
 */
export const useCanCreateProduct = () => {
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

/**
 * Check if user can modify prices
 * All roles (admin, manager, cashier)
 */
export const useCanModifyPrice = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.MODIFY_PRICE);
};

/**
 * Check if user can apply discounts
 * All roles (admin, manager, cashier)
 */
export const useCanApplyDiscount = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.APPLY_DISCOUNT);
};

/**
 * Check if user can view statistics/reports
 * Manager and Admin only
 */
export const useCanViewStatistics = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.VIEW_STATISTICS);
};

/**
 * Check if user can manage printers
 * Admin only
 */
export const useCanManagePrinters = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.MANAGE_PRINTERS);
};

/**
 * Check if user can manage product attributes
 * Manager and Admin only
 */
export const useCanManageAttributes = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.MANAGE_ATTRIBUTES);
};

// ==================== Role-Based Hooks ====================

/**
 * Check if current user is an admin
 */
export const useIsAdmin = () => {
  const { hasRole } = usePermission();
  return hasRole('admin');
};

/**
 * Check if current user is a manager
 */
export const useIsManager = () => {
  const { hasRole } = usePermission();
  return hasRole('manager');
};

/**
 * Check if current user is a cashier
 */
export const useIsCashier = () => {
  const { hasRole } = usePermission();
  return hasRole('cashier');
};

/**
 * Check if current user is manager or admin
 * (has elevated privileges)
 */
export const useIsManagerOrAbove = () => {
  const { hasRole } = usePermission();
  return hasRole(['admin', 'manager']);
};

/**
 * Check if current user is admin or manager or cashier
 * (any authenticated user)
 */
export const useIsAuthenticated = () => {
  const { hasRole } = usePermission();
  return hasRole(['admin', 'manager', 'cashier']);
};
