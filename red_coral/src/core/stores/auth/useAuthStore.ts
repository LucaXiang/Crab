import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { createTauriClient, type LoginRequest } from '@/infrastructure/api';
import type { User } from '@/core/domain/types';

// API Client (use TauriApiClient directly for full CRUD support)
const api = createTauriClient();

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
  fetchUserPermissions: (roleId: string) => Promise<void>;
  refreshToken: () => Promise<void>;

  // Permission Checks
  hasPermission: (permission: string) => boolean;
  hasRole: (role: string | string[]) => boolean;

  // User Management Actions (Admin only)
  fetchUsers: () => Promise<User[]>;
  createUser: (data: { username: string; password: string; displayName?: string; role: string }) => Promise<User>;
  updateUser: (userId: string, data: { displayName?: string; role?: string; isActive?: boolean }) => Promise<User>;
  resetPassword: (userId: string, newPassword: string) => Promise<void>;
  deleteUser: (userId: string) => Promise<void>;
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
          const { access_token, user: userData } = await api.login({ username, password });

          // 将 API 用户数据转换为本地 User 类型
          const user: User = {
            id: userData.id,
            username: userData.username,
            display_name: userData.display_name,
            role_id: userData.role_id,
            avatar: userData.avatar,
            is_active: true,
            is_system: userData.is_system ?? false,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          };

          // 获取权限 - API 返回 RolePermission[]，提取 permission 字段
          const rolePermissions = await api.getRolePermissions(userData.role_id);
          const permissions = rolePermissions.permissions.map(p => p.permission);

          set({
            user,
            permissions,
            isAuthenticated: true,
            isLoading: false,
            error: null,
          });

          return true;
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
      fetchUserPermissions: async (roleId: string) => {
        try {
          const rolePermissions = await api.getRolePermissions(roleId);
          const permissions = rolePermissions.permissions.map(p => p.permission);
          set({ permissions });
        } catch (error) {
          console.error('Failed to fetch permissions:', error);
          set({ permissions: [] });
        }
      },

      /**
       * Refresh token (handled by Rust ClientBridge)
       */
      refreshToken: async () => {
        try {
          await api.refreshToken();
        } catch {
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
        if (user?.role_name === 'admin' || user?.role_id === 'role:admin' || permissions.includes('*')) return true;
        return permissions.includes(permission);
      },

      /**
       * Check if current user has one of the specified roles
       */
      hasRole: (role: string | string[]) => {
        const { user } = get();
        if (!user || !user.role_name) return false;

        if (Array.isArray(role)) {
          return role.includes(user.role_name);
        }
        return user.role_name === role;
      },

      // ==================== User Management ====================

      fetchUsers: async () => {
        const employees = await api.listEmployees();
        // 转换 Employee -> User
        return employees.map((e) => ({
          id: e.id ?? '',
          username: e.username,
          display_name: e.display_name,
          role_id: e.role,
          role_name: undefined,
          avatar: null,
          is_active: e.is_active,
          is_system: e.is_system,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        })) as User[];
      },

      createUser: async (data: { username: string; password: string; displayName?: string; role: string }) => {
        const result = await api.createEmployee({
          username: data.username,
          password: data.password,
          role: data.role,
        });
        return {
          id: result.id ?? '',
          username: result.username,
          display_name: data.displayName || result.display_name,
          role_id: result.role,
          avatar: null,
          is_active: result.is_active,
          is_system: result.is_system,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        } as User;
      },

      updateUser: async (userId: string, data: { displayName?: string; role?: string; isActive?: boolean }) => {
        const result = await api.updateEmployee(userId, {
          role: data.role,
          is_active: data.isActive,
        });
        return {
          id: result.id ?? userId,
          username: result.username,
          display_name: data.displayName || result.display_name,
          role_id: result.role,
          avatar: null,
          is_active: result.is_active,
          is_system: result.is_system,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        } as User;
      },

      resetPassword: async (userId: string, newPassword: string) => {
        await api.updateEmployee(userId, { password: newPassword });
      },

      deleteUser: async (userId: string) => {
        await api.deleteEmployee(userId);
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
