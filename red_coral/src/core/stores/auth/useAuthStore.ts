import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { createApiClient, type LoginRequest } from '@/infrastructure/api';
import type { User } from '@/core/domain/types';

// API Client
const api = createApiClient();

interface AuthStore {
  // State
  user: User | null;
  permissions: string[]; // List of permissions for the current user
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;

  // Authentication Actions
  login: (username: string, password: string) => Promise<boolean>;
  logout: () => void;
  setUser: (user: User | null) => void;
  fetchUserPermissions: (roleId: number) => Promise<void>;
  refreshToken: () => Promise<void>;

  // Permission Checks
  hasPermission: (permission: string) => boolean;
  hasRole: (role: string | string[]) => boolean;

  // User Management Actions (Admin only)
  fetchUsers: () => Promise<User[]>;
  createUser: (data: any) => Promise<User>;
  updateUser: (userId: number, data: any) => Promise<User>;
  resetPassword: (userId: number, newPassword: string) => Promise<void>;
  deleteUser: (userId: number) => Promise<void>;
}

export const useAuthStore = create<AuthStore>()(
  persist(
    (set, get) => ({
      // Initial State
      user: null,
      permissions: [],
      isAuthenticated: false,
      isLoading: false,
      error: null,

      // ==================== Authentication ====================

      /**
       * Login with username and password
       */
      login: async (username: string, password: string) => {
        set({ isLoading: true, error: null });

        try {
          const request: LoginRequest = { username, password };
          const response = await api.login(request);

          if (response.data) {
            const { access_token, user: userData } = response.data;

            // 将 API 用户数据转换为本地 User 类型
            // 注意：API 使用 snake_case，User 类型也使用 snake_case
            const user: User = {
              id: userData.id,
              uuid: userData.uuid,
              username: userData.username,
              display_name: userData.display_name,
              role_id: userData.role_id,
              avatar: userData.avatar,
              is_active: true,
              created_at: new Date().toISOString(),
              updated_at: new Date().toISOString(),
            };

            // 设置访问令牌 (同时存储到 localStorage 供其他 API 客户端使用)
            api.setAccessToken(access_token);

            // 获取权限 - API 返回 RolePermission[]，提取 permission 字段
            // 注意：TauriApiClient 期望 string，ApiClient 期望 number，统一使用 string
            const rolePermissions = await api.getRolePermissions(String(userData.role_id));
            const permissions = rolePermissions.data?.permissions.map(p => p.permission) || [];

            set({
              user,
              permissions,
              isAuthenticated: true,
              isLoading: false,
              error: null,
            });

            return true;
          }

          set({
            isLoading: false,
            error: 'Login failed',
          });
          return false;
        } catch (error: any) {
          console.error('Login failed:', error);
          set({
            isLoading: false,
            error: error.message || 'Authentication failed',
          });
          return false;
        }
      },

      /**
       * Logout current user
       */
      logout: () => {
        api.clearAccessToken(); // This now also clears localStorage
        set({ user: null, permissions: [], isAuthenticated: false, error: null });
      },

      /**
       * Set user directly (for programmatic updates)
       */
      setUser: (user) => {
        if (!user) {
          set({ user, permissions: [], isAuthenticated: false });
        } else {
          set({ user, isAuthenticated: true });
        }
      },

      /**
       * Fetch permissions for a role
       */
      fetchUserPermissions: async (roleId: number) => {
        try {
          const response = await api.getRolePermissions(String(roleId));
          // API 返回 RolePermission[]，提取 permission 字段
          const permissions = response.data?.permissions.map(p => p.permission) || [];
          set({ permissions });
        } catch (error) {
          console.error('Failed to fetch permissions:', error);
          set({ permissions: [] });
        }
      },

      /**
       * Refresh token
       */
      refreshToken: async () => {
        try {
          const response = await api.refreshToken();
          if (response.data?.access_token) {
            api.setAccessToken(response.data.access_token);
          }
        } catch (error) {
          console.error('Failed to refresh token:', error);
          // If refresh fails, logout
          get().logout();
        }
      },

      // ==================== Permission Checks ====================

      /**
       * Check if current user has a specific permission
       */
      hasPermission: (permission: string) => {
        const { permissions, user } = get();
        // Admin always has all permissions
        if (user?.role_name === 'admin' || user?.role_id === 1 || permissions.includes('*')) return true;
        return permissions.includes(permission);
      },

      /**
       * Check if current user has one of the specified roles
       */
      hasRole: (role: string | string[]) => {
        const { user } = get();
        if (!user) return false;

        // 优先检查 role_name
        if (user.role_name) {
          if (Array.isArray(role)) {
            return role.includes(user.role_name);
          }
          return user.role_name === role;
        }

        // Fallback to role_id check (legacy)
        // Note: This is a simplified check. In a real app, you'd fetch role names
        // For now, we'll just check if user is admin (role_id === 1)
        if (Array.isArray(role)) {
          return role.includes('admin') && user.role_id === 1;
        }
        return role === 'admin' && user.role_id === 1;
      },

      // ==================== User Management ====================
      // TODO: 实现用户管理 API

      fetchUsers: async () => {
        throw new Error('Not implemented: Use role-based API instead');
      },

      createUser: async (data: any) => {
        throw new Error('Not implemented: Use role-based API instead');
      },

      updateUser: async (userId: number, data: any) => {
        throw new Error('Not implemented: Use role-based API instead');
      },

      resetPassword: async (userId: number, newPassword: string) => {
        throw new Error('Not implemented');
      },

      deleteUser: async (userId: number) => {
        throw new Error('Not implemented');
      },
    }),
    {
      name: 'auth-storage',
      partialize: (state) => ({
        user: state.user,
        permissions: state.permissions,
        isAuthenticated: state.isAuthenticated,
      }),
    }
  )
);

// ==================== Selectors ====================

/**
 * Hook to check if user is authenticated
 */
export const useIsAuthenticated = () => useAuthStore((state) => state.isAuthenticated);

/**
 * Hook to get current user
 */
export const useCurrentUser = () => useAuthStore((state) => state.user);

/**
 * Hook to get hasPermission function
 */
export const useHasPermission = () => useAuthStore((state) => state.hasPermission);

/**
 * Hook to get hasRole function
 */
export const useHasRole = () => useAuthStore((state) => state.hasRole);
