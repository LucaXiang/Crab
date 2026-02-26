import { create } from 'zustand';
import { createTauriClient, type LoginRequest } from '@/infrastructure/api';
import { logger } from '@/utils/logger';
import type { User } from '@/core/domain/types';

// Lazy getter to break circular dependency:
// tauri-client.ts imports useAuthStore → useAuthStore imports createTauriClient
// createTauriClient() is already a singleton, so repeated calls are free.
const getApi = () => createTauriClient();

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

  // Permission Checks
  hasPermission: (permission: string) => boolean;
  hasRole: (role: string | string[]) => boolean;

  // User Management Actions (Admin only)
  fetchUsers: () => Promise<User[]>;
  createUser: (data: { username: string; password: string; displayName?: string; role_id: number }) => Promise<User>;
  updateUser: (userId: number, data: { displayName?: string; role_id?: number; isActive?: boolean }) => Promise<User>;
  resetPassword: (userId: number, newPassword: string) => Promise<void>;
  deleteUser: (userId: number) => Promise<void>;
}

export const useAuthStore = create<AuthStore>()(
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
       * 
       * Backend returns complete user info including permissions,
       * no need for additional API calls.
       */
      login: async (username: string, password: string) => {
        set({ isLoading: true, error: null });

        try {
          const { token, user: userData } = await getApi().login({ username, password });

          // UserInfo from backend already has all fields we need
          const user: User = {
            id: userData.id,
            username: userData.username,
            name: userData.name,
            role_id: userData.role_id,
            role_name: userData.role_name,
            permissions: userData.permissions,
            is_system: userData.is_system,
            is_active: userData.is_active,
            created_at: userData.created_at,
          };

          set({
            user,
            permissions: userData.permissions,
            isAuthenticated: true,
            isLoading: false,
            error: null,
          });

          return true;
        } catch (error: unknown) {
          logger.error('Login failed', error);
          set({
            isLoading: false,
            error: error instanceof Error ? error.message : 'Authentication failed',
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
          set({ user, permissions: user.permissions ?? [], isAuthenticated: true });
        }
      },

      // ==================== Permission Checks ====================

      /**
       * Check if current user has a specific permission
       * 
       * Supports:
       * - Admin role has all permissions
       * - "all" permission grants everything
       * - Wildcard matching: "products:*" matches "products:read"
       */
      hasPermission: (permission: string) => {
        const { permissions, user } = get();
        if (!user) return false;

        // Admin always has all permissions
        if (user.role_name === 'admin') return true;
        
        // "all" permission grants everything
        if (permissions.includes('all')) return true;

        // Exact match
        if (permissions.includes(permission)) return true;

        // Wildcard matching: "products:*" matches "products:read"
        return permissions.some(p => {
          if (p.endsWith(':*')) {
            const prefix = p.slice(0, -1); // "products:*" -> "products:"
            return permission.startsWith(prefix);
          }
          return false;
        });
      },

      /**
       * Check if current user has one of the specified roles
       */
      hasRole: (role: string | string[]) => {
        const { user } = get();
        if (!user) return false;

        if (Array.isArray(role)) {
          return role.includes(user.role_name);
        }
        return user.role_name === role;
      },

      // ==================== User Management ====================

      fetchUsers: async () => {
        const employees = await getApi().listEmployees();
        return employees.map((e): User => ({
          id: e.id,
          username: e.username,
          name: e.name,
          role_id: e.role_id,
          role_name: '', // Role name not available in list view
          permissions: [],
          is_active: e.is_active,
          is_system: e.is_system,
          created_at: 0,
        }));
      },

      createUser: async (data: { username: string; password: string; displayName?: string; role_id: number }) => {
        const result = await getApi().createEmployee({
          username: data.username,
          password: data.password,
          role_id: data.role_id,
        });
        return {
          id: result.id,
          username: result.username,
          name: data.displayName || result.name,
          role_id: result.role_id,
          role_name: '',
          permissions: [] as string[],
          is_active: result.is_active,
          is_system: result.is_system,
          created_at: 0,
        } satisfies User;
      },

      updateUser: async (userId: number, data: { displayName?: string; role_id?: number; isActive?: boolean }) => {
        const result = await getApi().updateEmployee(userId, {
          role_id: data.role_id,
          is_active: data.isActive,
        });
        return {
          id: result.id,
          username: result.username,
          name: data.displayName || result.name,
          role_id: result.role_id,
          role_name: '',
          permissions: [] as string[],
          is_active: result.is_active,
          is_system: result.is_system,
          created_at: 0,
        } satisfies User;
      },

      resetPassword: async (userId: number, newPassword: string) => {
        await getApi().updateEmployee(userId, { password: newPassword });
      },

      deleteUser: async (userId: number) => {
        await getApi().deleteEmployee(userId);
      },
    })
);

// ==================== Selectors ====================

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
