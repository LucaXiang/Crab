import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import type { ApiResponse } from '@/core/domain/types/api';
import type {
  ActivationRequiredReason,
  ActivationProgress,
  SubscriptionBlockedInfo,
  HealthStatus,
} from '@/core/domain/types/appState';
import { getActivationReasonMessage } from '@/core/domain/types/appState';

// Import ConnectionStatus from the hook (canonical source)
import type { ConnectionStatus } from '@/core/hooks/useConnectionStatus';

// Types matching Rust definitions
export type ModeType = 'Server' | 'Client' | 'Disconnected';
export type LoginMode = 'Online' | 'Offline';

/**
 * AppState - 应用状态枚举
 *
 * 与 Rust 定义保持一致: `src-tauri/src/core/bridge/types.rs`
 * 参考设计文档: `docs/plans/2026-01-26-tenant-auth-detailed-status-design.md`
 */
export type AppState =
  // 通用状态
  | { type: 'Uninitialized' }
  // Server 模式专属
  | { type: 'ServerNoTenant' }
  | {
      type: 'ServerNeedActivation';
      data: {
        reason: ActivationRequiredReason;
        can_auto_recover: boolean;
        recovery_hint: string;
      };
    }
  | { type: 'ServerActivating'; data: { progress: ActivationProgress } }
  | { type: 'ServerCheckingSubscription' }
  | { type: 'ServerSubscriptionBlocked'; data: { info: SubscriptionBlockedInfo } }
  | { type: 'ServerReady' }
  | { type: 'ServerAuthenticated' }
  // Client 模式专属
  | { type: 'ClientDisconnected' }
  | { type: 'ClientNeedSetup' }
  | { type: 'ClientConnecting'; data: { server_url: string } }
  | { type: 'ClientConnected' }
  | { type: 'ClientAuthenticated' };

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

  /** 是否需要设置/激活 */
  needsSetup: (state: AppState | null): boolean => {
    if (!state) return true;
    return [
      'Uninitialized',
      'ServerNoTenant',
      'ServerNeedActivation',
      'ClientDisconnected',
      'ClientNeedSetup',
    ].includes(state.type);
  },

  /** 是否被订阅阻止 */
  isSubscriptionBlocked: (state: AppState | null): boolean => {
    if (!state) return false;
    return state.type === 'ServerSubscriptionBlocked';
  },

  /** 获取推荐路由 */
  getRouteForState: (state: AppState | null): string => {
    if (!state) return '/setup';

    switch (state.type) {
      // 首次设置
      case 'ServerNoTenant':
        return '/setup';

      // 需要激活
      case 'ServerNeedActivation':
        // FirstTimeSetup 直接进入设置页面，其他问题显示具体原因
        if (state.data.reason.code === 'FirstTimeSetup') {
          return '/setup';
        }
        return '/status/activation-required';

      // 激活中 - 显示进度
      case 'ServerActivating':
        return '/status/activating';

      // 检查订阅
      case 'ServerCheckingSubscription':
        return '/status/checking';

      // 订阅阻止
      case 'ServerSubscriptionBlocked':
        return '/status/subscription-blocked';

      // 未初始化
      case 'Uninitialized':
      case 'ClientDisconnected':
      case 'ClientNeedSetup':
      case 'ClientConnecting':
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

// Re-export for consumers who import from this file
export type { ConnectionStatus };

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
  is_system?: boolean;
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

export interface AppConfigResponse {
  current_mode: ModeType;
  current_tenant: string | null;
  known_tenants: string[];
}

interface BridgeStore {
  // State
  appState: AppState | null;
  modeInfo: ModeInfo | null;
  tenants: TenantInfo[];
  currentSession: EmployeeSession | null;
  isFirstRun: boolean;
  isLoading: boolean;
  error: string | null;
  connectionStatus: ConnectionStatus;

  // App State Actions
  fetchAppState: () => Promise<AppState | null>;
  fetchHealthStatus: () => Promise<HealthStatus | null>;

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

  // Auth Actions (unified - ClientBridge based)
  loginEmployee: (username: string, password: string) => Promise<LoginResponse>;
  logoutEmployee: () => Promise<void>;

  // Connection Status Actions
  setConnectionStatus: (status: ConnectionStatus) => void;
}

export const useBridgeStore = create<BridgeStore>()(
  persist(
    (set, get) => ({
      // Initial State
      appState: null,
      modeInfo: null,
      tenants: [],
      currentSession: null,
      isFirstRun: true,
      isLoading: false,
      error: null,
      connectionStatus: { connected: true, reconnecting: false },

      // App State Actions
      fetchAppState: async () => {
        try {
          const state = await invokeApi<AppState>('get_app_state');
          set({ appState: state });
          return state;
        } catch (error: unknown) {
          console.error('Failed to fetch app state:', error);
          set({ error: error instanceof Error ? error.message : 'Failed to fetch app state' });
          return null;
        }
      },

      fetchHealthStatus: async () => {
        try {
          return await invokeApi<HealthStatus>('get_health_status');
        } catch (error) {
          console.error('Failed to fetch health status:', error);
          return null;
        }
      },

      // Mode Actions
      fetchModeInfo: async () => {
        try {
          const info = await invokeApi<ModeInfo>('get_mode_info');
          set({ modeInfo: info });
        } catch (error: unknown) {
          console.error('Failed to fetch mode info:', error);
        }
      },

      checkFirstRun: async () => {
        try {
          return await invokeApi<boolean>('check_first_run');
        } catch (error) {
          console.error('Failed to check first run:', error);
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
          await invokeApi('start_client_mode', { edge_url: edgeUrl, message_addr: messageAddr });
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
          set({
            appState: { type: 'Uninitialized' },
            modeInfo: null,
            currentSession: null
          });
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
        } finally {
          set({ isLoading: false });
        }
      },

      // Tenant Actions
      fetchTenants: async () => {
        try {
          // TenantListData wrapper
          const data = await invokeApi<{ tenants: TenantInfo[] }>('list_tenants');
          set({ tenants: data.tenants });
        } catch (error) {
          console.error('Failed to fetch tenants:', error);
        }
      },

      activateTenant: async (authUrl, username, password) => {
        try {
          set({ isLoading: true, error: null });
          const msg = await invokeApi<string>('activate_tenant', {
            auth_url: authUrl,
            username,
            password,
          });
          await get().fetchTenants();
          await get().fetchAppState();
          return msg;
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      switchTenant: async (tenantId) => {
        try {
          set({ isLoading: true });
          await invokeApi('switch_tenant', { tenant_id: tenantId });
          await get().fetchAppState();
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
          throw error;
        } finally {
          set({ isLoading: false });
        }
      },

      removeTenant: async (tenantId) => {
        try {
          await invokeApi('remove_tenant', { tenantId });
          await get().fetchTenants();
        } catch (error: unknown) {
          set({ error: error instanceof Error ? error.message : 'Operation failed' });
          throw error;
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
          console.error('Logout failed:', error);
        }
      },

      // ==================== Connection Status ====================

      setConnectionStatus: (status: ConnectionStatus) => {
        set({ connectionStatus: status });
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
export const useIsFirstRun = () => useBridgeStore((state) => state.isFirstRun);
export const useModeInfo = () => useBridgeStore((state) => state.modeInfo);
export const useTenants = () => useBridgeStore((state) => state.tenants);
export const useCurrentSession = () => useBridgeStore((state) => state.currentSession);
export const useBridgeLoading = () => useBridgeStore((state) => state.isLoading);
export const useBridgeError = () => useBridgeStore((state) => state.error);

/**
 * 检查是否可以访问 POS
 */
export const useCanAccessPOS = () =>
  useBridgeStore((state) => AppStateHelpers.canAccessPOS(state.appState));

/**
 * 获取推荐路由
 */
export const useRecommendedRoute = () =>
  useBridgeStore((state) => AppStateHelpers.getRouteForState(state.appState));

/**
 * Selector for connection status from the bridge store.
 *
 * Note: This differs from the `useConnectionStatus` hook in @/core/hooks:
 * - The hook (`useConnectionStatus`) listens directly to Tauri events and manages local state
 * - This selector (`useBridgeConnectionStatus`) reads from the global Zustand store
 *
 * Intended usage pattern:
 * - A top-level component (e.g., App.tsx or ConnectionStatusProvider) should use the hook
 *   to listen for events and call `setConnectionStatus` to update the global store
 * - Child components can then use this selector to access the connection status
 */
export const useBridgeConnectionStatus = () => useBridgeStore((state) => state.connectionStatus);
