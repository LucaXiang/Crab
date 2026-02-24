import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { logger } from '@/utils/logger';
import type { ApiResponse } from '@/core/domain/types/api';
import type {
  ActivationRequiredReason,
  SubscriptionBlockedInfo,
  HealthStatus,
  AppState,
  SubscriptionStatus,
  PlanType,
} from '@/core/domain/types/appState';
import { getActivationReasonMessage } from '@/core/domain/types/appState';

// Types matching Rust definitions
export type ModeType = 'Server' | 'Client';
export type LoginMode = 'Online' | 'Offline';

// Re-export AppState from the canonical source
export type { AppState };

/**
 * AppState 辅助函数
 */
export const AppStateHelpers = {
  /** 是否可以访问 POS 主界面 */
  canAccessPOS: (state: AppState | null): boolean => {
    if (!state) return false;
    return state.type === 'ServerAuthenticated' || state.type === 'ClientAuthenticated';
  },

  /** 是否需要员工登录 */
  needsEmployeeLogin: (state: AppState | null): boolean => {
    if (!state) return false;
    return state.type === 'ServerReady' || state.type === 'ClientConnected';
  },

  /** 获取推荐路由 */
  getRouteForState: (state: AppState | null): string => {
    if (!state) return '/setup';

    switch (state.type) {
      // 无租户 / 租户已验证待选模式
      case 'NeedTenantLogin':
      case 'TenantReady':
        return '/setup';

      // 需要激活（证书过期/设备不匹配等）
      case 'ServerNeedActivation':
      case 'ClientNeedActivation':
        if (state.data.reason.code === 'FirstTimeSetup') {
          return '/setup';
        }
        return '/status/activation-required';

      // 订阅阻止
      case 'ServerSubscriptionBlocked':
        return '/status/subscription-blocked';

      // P12 证书阻止
      case 'ServerP12Blocked':
        return '/status/p12-blocked';

      // Client 断连
      case 'ClientDisconnected':
        return '/setup';

      // 需要登录
      case 'ServerReady':
      case 'ClientConnected':
        return '/login';

      // 可以进入 POS
      case 'ServerAuthenticated':
      case 'ClientAuthenticated':
        return '/pos';

      default:
        return '/setup';
    }
  },

  /** 获取激活原因消息 */
  getActivationReasonMessage,
};

export interface ModeInfo {
  mode: ModeType | null;
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
  subscription_status: string | null;
}

// UserInfo matching shared::client::UserInfo from Rust
export interface UserInfo {
  id: number;
  username: string;
  display_name: string;
  role_id: number;
  role_name: string;
  permissions: string[];
  is_system: boolean;
  is_active: boolean;
  created_at: number;
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

interface AuthData {
  session: EmployeeSession | null;
  mode: LoginMode;
}

export interface QuotaInfo {
  max_slots: number;
  active_count: number;
  active_devices: ActiveDevice[];
}

export interface ActiveDevice {
  entity_id: string;
  device_id: string;
  activated_at: number;
  last_refreshed_at: number | null;
}

export interface ActivationResult {
  tenant_id: string;
  subscription_status: string | null;
  quota_info: QuotaInfo | null;
}

export interface AppConfigResponse {
  current_mode: ModeType | null;
  current_tenant: string | null;
  known_tenants: string[];
}

export interface TenantVerifyData {
  tenant_id: string;
  token: string;
  refresh_token: string;
  subscription_status: SubscriptionStatus;
  plan: PlanType;
  server_slots_remaining: number;
  client_slots_remaining: number;
  has_active_server: boolean;
  has_active_client: boolean;
  has_p12: boolean;
}

interface BridgeStore {
  // State
  appState: AppState | null;
  modeInfo: ModeInfo | null;
  currentSession: EmployeeSession | null;
  isFirstRun: boolean;
  isLoading: boolean;
  error: string | null;
  // App State Actions
  fetchAppState: () => Promise<AppState | null>;
  fetchHealthStatus: () => Promise<HealthStatus | null>;

  // Mode Actions
  fetchModeInfo: () => Promise<void>;
  checkFirstRun: () => Promise<boolean>;
  startServerMode: () => Promise<void>;
  startClientMode: (edgeUrl: string, messageAddr: string) => Promise<void>;
  stopMode: () => Promise<void>;

  // Config Actions
  updateServerConfig: (httpPort: number, messagePort: number) => Promise<void>;
  updateClientConfig: (edgeUrl: string, messageAddr: string) => Promise<void>;

  // Tenant Actions
  verifyTenant: (username: string, password: string) => Promise<TenantVerifyData>;
  activateServerTenant: (replaceEntityId?: string) => Promise<ActivationResult>;
  activateClientTenant: (replaceEntityId?: string) => Promise<ActivationResult>;
  deactivateCurrentMode: () => Promise<void>;
  exitTenant: () => Promise<void>;
  getCurrentTenant: () => Promise<string | null>;

  // Auth Actions (unified - ClientBridge based)
  loginEmployee: (username: string, password: string) => Promise<LoginResponse>;
  logoutEmployee: () => Promise<void>;
  fetchCurrentSession: () => Promise<EmployeeSession | null>;

}

export const useBridgeStore = create<BridgeStore>()(
  persist(
    (set, get) => ({
      // Initial State
      appState: null as AppState | null,
      modeInfo: null as ModeInfo | null,
      currentSession: null as EmployeeSession | null,
      isFirstRun: true,
      isLoading: false,
      error: null as string | null,
      // App State Actions
      fetchAppState: async () => {
        try {
          const state = await invokeApi<AppState>('get_app_state');
          set({ appState: state });
          return state;
        } catch (error: unknown) {
          logger.error('Failed to fetch app state', error);
          set({ error: error instanceof Error ? error.message : 'Failed to fetch app state' });
          return null;
        }
      },

      fetchHealthStatus: async () => {
        try {
          return await invokeApi<HealthStatus>('get_health_status');
        } catch (error) {
          logger.error('Failed to fetch health status', error);
          return null;
        }
      },

      // Mode Actions
      fetchModeInfo: async () => {
        try {
          const info = await invokeApi<ModeInfo>('get_mode_info');
          set({ modeInfo: info });
        } catch (error: unknown) {
          logger.error('Failed to fetch mode info', error);
        }
      },

      checkFirstRun: async () => {
        try {
          return await invokeApi<boolean>('check_first_run');
        } catch (error) {
          logger.error('Failed to check first run', error);
          return false;
        }
      },

      startServerMode: async () => {
        try {
          set({ isLoading: true, error: null });
          await invokeApi('start_server_mode');
          await get().fetchAppState();
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      startClientMode: async (edgeUrl: string, messageAddr: string) => {
        try {
          set({ isLoading: true, error: null });
          await invokeApi('start_client_mode', { edgeUrl, messageAddr });
          await get().fetchAppState();
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      stopMode: async () => {
        try {
          set({ isLoading: true });
          await invokeApi('stop_mode');
          set({ modeInfo: null, currentSession: null });
          await get().fetchAppState();
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
        } finally {
          set({ isLoading: false });
        }
      },

      // Config Actions
      updateServerConfig: async (httpPort: number, messagePort: number) => {
        try {
          set({ isLoading: true, error: null });
          await invokeApi('update_server_config', { httpPort, messagePort });
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Failed to update server config' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      updateClientConfig: async (edgeUrl: string, messageAddr: string) => {
        try {
          set({ isLoading: true, error: null });
          await invokeApi('update_client_config', { edgeUrl, messageAddr });
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Failed to update client config' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      // Tenant Actions
      verifyTenant: async (username, password) => {
        try {
          set({ isLoading: true, error: null });
          const data = await invokeApi<TenantVerifyData>('verify_tenant', { username, password });
          await get().fetchAppState();
          return data;
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Verification failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      activateServerTenant: async (replaceEntityId?) => {
        try {
          set({ isLoading: true, error: null });
          const result = await invokeApi<ActivationResult>('activate_server_tenant', {
            replaceEntityId: replaceEntityId ?? null,
          });
          await get().fetchAppState();
          return result;
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      activateClientTenant: async (replaceEntityId?) => {
        try {
          set({ isLoading: true, error: null });
          const result = await invokeApi<ActivationResult>('activate_client_tenant', {
            replaceEntityId: replaceEntityId ?? null,
          });
          await get().fetchAppState();
          return result;
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      deactivateCurrentMode: async () => {
        try {
          set({ isLoading: true, error: null });
          await invokeApi('deactivate_current_mode');
          set({
            appState: { type: 'TenantReady' },
            modeInfo: null,
            currentSession: null,
          });
          await get().fetchAppState();
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Deactivation failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      exitTenant: async () => {
        try {
          set({ isLoading: true, error: null });
          await invokeApi('exit_tenant');
          set({
            appState: { type: 'NeedTenantLogin' },
            modeInfo: null,
            currentSession: null,
          });
          await get().fetchAppState();
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      getCurrentTenant: async () => {
        try {
          return await invokeApi<string | null>('get_current_tenant');
        } catch (error) {
          return null;
        }
      },

      // ==================== Auth (ClientBridge) ====================

      loginEmployee: async (username: string, password: string) => {
        set({ isLoading: true, error: null });
        try {
          const data = await invokeApi<AuthData>('login_employee', {
            username,
            password,
          });

          if (data.session) {
            set({ currentSession: data.session });
            // 刷新 appState 以反映认证状态变化
            await get().fetchAppState();
          }

          return {
            success: true,
            session: data.session,
            error: null,
            mode: data.mode
          };
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Login failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      logoutEmployee: async () => {
        try {
          await invokeApi('logout_employee');
          set({ currentSession: null });
          // 刷新 appState 以反映登出状态
          await get().fetchAppState();
        } catch (error: unknown) {
          logger.error('Logout failed', error);
        }
      },

      fetchCurrentSession: async () => {
        try {
          const session = await invokeApi<EmployeeSession | null>('get_current_session');
          if (session) {
            set({ currentSession: session });
            logger.debug('Restored session from backend', { component: 'Bridge', username: session.username });
          }
          return session;
        } catch (error) {
          logger.error('Failed to fetch current session', error);
          return null;
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

export const useAppState = () => useBridgeStore((state) => state.appState);

