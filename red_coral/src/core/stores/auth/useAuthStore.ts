import { create } from 'zustand';
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
          const { token, user: userData } = await api.login({ username, password });

          // UserInfo from backend already has all fields we need
          const user: User = {
            id: userData.id,
            username: userData.username,
            display_name: userData.display_name,
            role_id: userData.role_id,
            role_name: userData.role_name,
            permissions: userData.permissions,
            is_system: userData.is_system,
            is_active: true,
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
          console.error('Login failed:', error);
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
          set({ user, isAuthenticated: true });
        }
      },

      /**
       * Fetch permissions for a role
       * 
       * Note: Permissions are now included in login response,
       * so this is mainly for refreshing permissions after role changes.
       */
      fetchUserPermissions: async (_roleId: string) => {
        // Permissions are now embedded in user object from login
        // To refresh, we should re-fetch current user from /api/auth/me
        const { user } = get();
        if (user?.permissions) {
          set({ permissions: user.permissions });
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
        const employees = await api.listEmployees();
        // Convert Employee -> User for display
        // Note: role_name is extracted from role_id (e.g., "role:admin" -> "admin")
        return employees.map((e) => ({
          id: e.id ?? '',
          username: e.username,
          display_name: e.display_name,
          role_id: e.role,
          role_name: e.role.replace(/^role:/, ''), // Extract name from RecordId
          permissions: [], // Permissions not loaded for list view
          is_active: e.is_active,
          is_system: e.is_system,
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
          role_name: result.role.replace(/^role:/, ''),
          permissions: [],
          is_active: result.is_active,
          is_system: result.is_system,
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
          role_name: result.role.replace(/^role:/, ''),
          permissions: [],
          is_active: result.is_active,
          is_system: result.is_system,
        } as User;
      },

      resetPassword: async (userId: string, newPassword: string) => {
        await api.updateEmployee(userId, { password: newPassword });
      },

      deleteUser: async (userId: string) => {
        await api.deleteEmployee(userId);
      },
    })
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
