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
 * if (hasPermission(Permission.ORDERS_VOID)) {
 *   // Show void order button
 * }
 */
export const usePermission = () => {
  const hasPermission = useHasPermission();
  const hasRole = useHasRole();

  return { hasPermission, hasRole };
};

// ==================== Specific Permission Hooks ====================
// 简化权限系统：模块化权限 + 敏感操作

/**
 * Check if user can manage users (admin only)
 */
export const useCanManageUsers = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.USERS_MANAGE);
};

/**
 * Check if user can manage menu (products, categories, attributes, tags)
 */
export const useCanManageMenu = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.MENU_MANAGE);
};

/**
 * Check if user can manage tables and zones
 */
export const useCanManageTables = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.TABLES_MANAGE);
};

/**
 * Check if user can view reports
 */
export const useCanViewReports = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.REPORTS_VIEW);
};

/**
 * Check if user can manage settings
 */
export const useCanManageSettings = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.SETTINGS_MANAGE);
};

/**
 * Check if user can manage shifts
 */
export const useCanManageShifts = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.SHIFTS_MANAGE);
};

/**
 * Check if user can manage price rules
 */
export const useCanManagePriceRules = () => {
  const { hasPermission } = usePermission();
  return hasPermission(PermissionValues.PRICE_RULES_MANAGE);
};

// === 兼容性别名 (deprecated) ===

/** @deprecated Use useCanManageMenu */
export const useCanManageProducts = () => useCanManageMenu();

/** @deprecated Use useCanManageMenu */
export const useCanUpdateProduct = () => useCanManageMenu();

/** @deprecated Use useCanManageMenu */
export const useCanDeleteProduct = () => useCanManageMenu();

/** @deprecated Use useCanManageMenu */
export const useCanManageCategories = () => useCanManageMenu();

/** @deprecated Use useCanManageTables */
export const useCanManageZones = () => useCanManageTables();
