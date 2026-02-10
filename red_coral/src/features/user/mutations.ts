/**
 * User Mutations - 用户 CRUD 操作
 *
 * 用户管理相关的 mutation 逻辑，包括创建、更新、删除用户和重置密码。
 * 这些操作主要通过 useAuthStore 完成，这里提供一个统一的 API 层。
 */

import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { invokeApi } from '@/infrastructure/api';
import type { User, Role } from '@/core/domain/types';

/**
 * 获取用户列表
 */
export async function fetchUsers(): Promise<User[]> {
  return useAuthStore.getState().fetchUsers();
}

/**
 * 创建用户
 */
export async function createUser(params: {
  username: string;
  password: string;
  displayName: string;
  role_id: number;
}): Promise<User> {
  return useAuthStore.getState().createUser(params);
}

/**
 * 更新用户
 */
export async function updateUser(
  userId: number,
  params: {
    displayName?: string;
    role_id?: number;
    isActive?: boolean;
  }
): Promise<User> {
  return useAuthStore.getState().updateUser(userId, params);
}

/**
 * 删除用户
 */
export async function deleteUser(userId: number): Promise<void> {
  return useAuthStore.getState().deleteUser(userId);
}

/**
 * 重置用户密码
 */
export async function resetPassword(userId: number, newPassword: string): Promise<void> {
  return useAuthStore.getState().resetPassword(userId, newPassword);
}

/**
 * 获取角色列表
 */
export async function fetchRoles(): Promise<Role[]> {
  return invokeApi<Role[]>('list_roles');
}

/**
 * 禁用用户（软删除）
 */
export async function disableUser(userId: number): Promise<User> {
  return updateUser(userId, { isActive: false });
}

/**
 * 启用用户
 */
export async function enableUser(userId: number): Promise<User> {
  return updateUser(userId, { isActive: true });
}
