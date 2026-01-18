import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

// Types matching Rust definitions
export type ModeType = 'Server' | 'Client' | 'Disconnected';
export type LoginMode = 'Online' | 'Offline';

export interface ModeInfo {
  mode: ModeType;
  is_connected: boolean;
  is_authenticated: boolean;
  tenant_id: string | null;
  username: string | null;
}

export interface TenantInfo {
  tenant_id: string;
  tenant_name: string | null;
  has_certificates: boolean;
  last_used: number | null;
}

// UserInfo matching shared::client::UserInfo from Rust
export interface UserInfo {
  id: string;
  username: string;
  role: string;
  permissions: string[];
}

export interface EmployeeSession {
  username: string;
  token: string;
  user_info: UserInfo;
  login_mode: LoginMode;
  expires_at: number | null;
  logged_in_at: number;
}

export interface LoginResponse {
  success: boolean;
  session: EmployeeSession | null;
  error: string | null;
  mode: LoginMode;
}

export interface AppConfigResponse {
  current_mode: ModeType;
  current_tenant: string | null;
  known_tenants: string[];
}

interface BridgeStore {
  // State
  modeInfo: ModeInfo | null;
  tenants: TenantInfo[];
  currentSession: EmployeeSession | null;
  isFirstRun: boolean;
  isLoading: boolean;
  error: string | null;

  // Mode Actions
  fetchModeInfo: () => Promise<void>;
  checkFirstRun: () => Promise<boolean>;
  startServerMode: () => Promise<void>;
  startClientMode: (edgeUrl: string, messageAddr: string) => Promise<void>;
  stopMode: () => Promise<void>;

  // Tenant Actions
  fetchTenants: () => Promise<void>;
  activateTenant: (authUrl: string, username: string, password: string) => Promise<string>;
  switchTenant: (tenantId: string) => Promise<void>;
  removeTenant: (tenantId: string) => Promise<void>;
  getCurrentTenant: () => Promise<string | null>;

  // Auth Actions (legacy - TenantManager based)
  loginOnline: (username: string, password: string, edgeUrl: string) => Promise<LoginResponse>;
  loginOffline: (username: string, password: string) => Promise<LoginResponse>;
  loginAuto: (username: string, password: string, edgeUrl: string) => Promise<LoginResponse>;
  logout: () => Promise<void>;
  fetchCurrentSession: () => Promise<void>;
  hasOfflineCache: (username: string) => Promise<boolean>;

  // Auth Actions (unified - ClientBridge based)
  loginEmployee: (username: string, password: string) => Promise<LoginResponse>;
  logoutEmployee: () => Promise<void>;
}

export const useBridgeStore = create<BridgeStore>()(
  persist(
    (set, get) => ({
      // Initial State
      modeInfo: null,
      tenants: [],
      currentSession: null,
      isFirstRun: true,
      isLoading: false,
      error: null,

      // ==================== Mode Actions ====================

      fetchModeInfo: async () => {
        try {
          const info = await invoke<ModeInfo>('get_mode_info');
          set({ modeInfo: info });
        } catch (error: any) {
          console.error('Failed to fetch mode info:', error);
          set({ error: error.message || 'Failed to fetch mode info' });
        }
      },

      checkFirstRun: async () => {
        try {
          const isFirst = await invoke<boolean>('check_first_run');
          set({ isFirstRun: isFirst });
          return isFirst;
        } catch (error: any) {
          console.error('Failed to check first run:', error);
          return true;
        }
      },

      startServerMode: async () => {
        set({ isLoading: true, error: null });
        try {
          await invoke('start_server_mode');
          await get().fetchModeInfo();
        } catch (error: any) {
          set({ error: error.message || 'Failed to start server mode' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      startClientMode: async (edgeUrl: string, messageAddr: string) => {
        set({ isLoading: true, error: null });
        try {
          await invoke('start_client_mode', { edgeUrl, messageAddr });
          await get().fetchModeInfo();
        } catch (error: any) {
          set({ error: error.message || 'Failed to start client mode' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      stopMode: async () => {
        set({ isLoading: true, error: null });
        try {
          await invoke('stop_mode');
          await get().fetchModeInfo();
        } catch (error: any) {
          set({ error: error.message || 'Failed to stop mode' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      // ==================== Tenant Actions ====================

      fetchTenants: async () => {
        try {
          const tenants = await invoke<TenantInfo[]>('list_tenants');
          set({ tenants });
        } catch (error: any) {
          console.error('Failed to fetch tenants:', error);
          set({ error: error.message || 'Failed to fetch tenants' });
        }
      },

      activateTenant: async (authUrl: string, username: string, password: string) => {
        set({ isLoading: true, error: null });
        try {
          const tenantId = await invoke<string>('activate_tenant', {
            authUrl,
            username,
            password,
          });
          await get().fetchTenants();
          set({ isFirstRun: false });
          return tenantId;
        } catch (error: any) {
          set({ error: error.message || 'Failed to activate tenant' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      switchTenant: async (tenantId: string) => {
        set({ isLoading: true, error: null });
        try {
          await invoke('switch_tenant', { tenantId });
          await get().fetchModeInfo();
        } catch (error: any) {
          set({ error: error.message || 'Failed to switch tenant' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      removeTenant: async (tenantId: string) => {
        set({ isLoading: true, error: null });
        try {
          await invoke('remove_tenant', { tenantId });
          await get().fetchTenants();
        } catch (error: any) {
          set({ error: error.message || 'Failed to remove tenant' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      getCurrentTenant: async () => {
        try {
          return await invoke<string | null>('get_current_tenant');
        } catch (error: any) {
          console.error('Failed to get current tenant:', error);
          return null;
        }
      },

      // ==================== Auth Actions ====================

      loginOnline: async (username: string, password: string, edgeUrl: string) => {
        set({ isLoading: true, error: null });
        try {
          const response = await invoke<LoginResponse>('login_online', {
            username,
            password,
            edgeUrl,
          });
          if (response.success && response.session) {
            set({ currentSession: response.session });
          }
          return response;
        } catch (error: any) {
          set({ error: error.message || 'Login failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      loginOffline: async (username: string, password: string) => {
        set({ isLoading: true, error: null });
        try {
          const response = await invoke<LoginResponse>('login_offline', {
            username,
            password,
          });
          if (response.success && response.session) {
            set({ currentSession: response.session });
          }
          return response;
        } catch (error: any) {
          set({ error: error.message || 'Offline login failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      loginAuto: async (username: string, password: string, edgeUrl: string) => {
        set({ isLoading: true, error: null });
        try {
          const response = await invoke<LoginResponse>('login_auto', {
            username,
            password,
            edgeUrl,
          });
          if (response.success && response.session) {
            set({ currentSession: response.session });
          }
          return response;
        } catch (error: any) {
          set({ error: error.message || 'Auto login failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      logout: async () => {
        try {
          await invoke('logout');
          set({ currentSession: null });
        } catch (error: any) {
          console.error('Logout failed:', error);
        }
      },

      fetchCurrentSession: async () => {
        try {
          const session = await invoke<EmployeeSession | null>('get_current_session');
          set({ currentSession: session });
        } catch (error: any) {
          console.error('Failed to fetch current session:', error);
        }
      },

      hasOfflineCache: async (username: string) => {
        try {
          return await invoke<boolean>('has_offline_cache', { username });
        } catch (error: any) {
          console.error('Failed to check offline cache:', error);
          return false;
        }
      },

      // ==================== Unified Auth (ClientBridge) ====================

      loginEmployee: async (username: string, password: string) => {
        set({ isLoading: true, error: null });
        try {
          const response = await invoke<LoginResponse>('login_employee', {
            username,
            password,
          });
          if (response.success && response.session) {
            set({ currentSession: response.session });
          }
          return response;
        } catch (error: any) {
          set({ error: error.message || 'Login failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      logoutEmployee: async () => {
        try {
          await invoke('logout_employee');
          set({ currentSession: null });
        } catch (error: any) {
          console.error('Logout failed:', error);
        }
      },
    }),
    {
      name: 'bridge-storage',
      partialize: () => ({
        // Don't persist isFirstRun - it should always come from backend
        // Don't persist session - fetch from backend on startup
      }),
    }
  )
);

// ==================== Selectors ====================

export const useIsFirstRun = () => useBridgeStore((state) => state.isFirstRun);
export const useModeInfo = () => useBridgeStore((state) => state.modeInfo);
export const useTenants = () => useBridgeStore((state) => state.tenants);
export const useCurrentSession = () => useBridgeStore((state) => state.currentSession);
export const useBridgeLoading = () => useBridgeStore((state) => state.isLoading);
export const useBridgeError = () => useBridgeStore((state) => state.error);
