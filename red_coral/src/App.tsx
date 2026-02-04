import React, { useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { useBridgeStore, AppStateHelpers } from '@/core/stores/bridge';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useSyncListener, useOrderEventListener, useOrderTimelineSync, useSyncConnection, useSystemIssueGuard } from '@/core/hooks';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { t } from '@/infrastructure/i18n';

// Components
import { ToastContainer } from '@/presentation/components/Toast';
import { ServerMessageToastContainer } from '@/presentation/components/ServerMessageToast';
import { ProtectedRoute } from '@/presentation/components/ProtectedRoute';
import { PermissionEscalationProvider } from '@/presentation/components/auth/PermissionEscalationProvider';
import { NotificationProvider } from '@/presentation/components/notifications';
import { ShiftGuard } from '@/presentation/components/shift';
import { SystemIssueDialog } from '@/presentation/components/modals/SystemIssueDialog';

// Screens
import { LoginScreen } from '@/screens/Login';
import { POSScreen } from '@/screens/POS';
import { SetupScreen } from '@/screens/Setup';
import { ActivateScreen } from '@/screens/Activate';

import { OrderDebug } from '@/screens/Debug';
import { ActivationRequiredScreen, SubscriptionBlockedScreen } from '@/screens/Status';

/**
 * 全局初始化状态 hook
 *
 * 在 App 级别运行，确保在渲染任何路由前完成：
 * 1. 获取租户列表
 * 2. 获取应用状态
 * 3. 恢复员工会话 (如果已认证)
 *
 * 这样无论用户刷新哪个页面，session 都会被正确恢复。
 */
const useAppInitialization = () => {
  const { fetchTenants, fetchAppState, fetchCurrentSession } = useBridgeStore();
  const [isInitialized, setIsInitialized] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const init = async () => {
    setError(null);
    try {
      // 1. 获取租户列表
      await fetchTenants();

      // 2. 获取当前应用状态
      await fetchAppState();

      // 3. 尝试恢复缓存的员工会话 (从后端获取)
      const currentAppState = useBridgeStore.getState().appState;
      if (currentAppState?.type === 'ServerAuthenticated' || currentAppState?.type === 'ClientAuthenticated') {
        const session = await fetchCurrentSession();
        if (session) {
          // 后端有有效 session，同步到 auth store
          const user = {
            id: session.user_info.id,
            username: session.user_info.username,
            display_name: session.user_info.display_name,
            role_id: session.user_info.role_id,
            role_name: session.user_info.role_name,
            permissions: session.user_info.permissions,
            is_system: session.user_info.is_system,
            is_active: true,
          };
          useAuthStore.getState().setUser(user);
        } else {
          // 后端无 session，清除前端 auth 状态
          useAuthStore.getState().logout();
        }
      } else {
        // 后端未认证，确保前端 auth 状态也清除
        useAuthStore.getState().logout();
      }

      setIsInitialized(true);
    } catch (err) {
      console.error('[AppInit] 初始化失败:', err);
      setError(err instanceof Error ? err.message : t('app.init.error_default'));
    }
  };

  useEffect(() => {
    init();
  }, [fetchTenants, fetchAppState, fetchCurrentSession]);

  return { isInitialized, error, retry: init };
};

// Initial route component - 已初始化完成，只做路由决策
// 使用新的 AppState 状态机进行路由决策
// 参考设计文档: docs/plans/2026-01-18-application-state-machine.md
const InitialRoute: React.FC = () => {
  const appState = useBridgeStore((state) => state.appState);

  // 使用 AppStateHelpers 确定路由
  const targetRoute = AppStateHelpers.getRouteForState(appState);
  return <Navigate to={targetRoute} replace />;
};

const App: React.FC = () => {
  const performanceMode = useSettingsStore((state) => state.performanceMode);

  // 全局初始化：在渲染路由前恢复 session
  const { isInitialized, error, retry } = useAppInitialization();

  // 挂载同步相关 hooks
  useSyncListener();
  useSyncConnection();

  // 挂载订单事件监听 hook (Event Sourcing)
  useOrderEventListener();
  useOrderTimelineSync();

  // System issue guard (Server 模式: 登录后检查 pending issues)
  const { currentIssue, resolveIssue } = useSystemIssueGuard();

  // Check for first run and clear storage if needed
  useEffect(() => {
    // Listen for the clear event from backend
    const unlistenPromise = listen('clear-local-storage', () => {
      localStorage.clear();
      sessionStorage.clear();
      // Reload to apply clean state
      window.location.reload();
    });

    return () => {
      unlistenPromise.then(unlisten => unlisten());
    };
  }, []);

  // Optimize startup: Show window only after React is mounted to avoid black screen
  useEffect(() => {
    const showWindow = async () => {
      // Small delay to ensure initial render paint is complete
      await new Promise(resolve => setTimeout(resolve, 50));
      try {
        await getCurrentWindow().show();
        // Ensure focus
        await getCurrentWindow().setFocus();
      } catch (err) {
        // Ignore errors if window is already visible or API fails
        console.debug('Failed to show window:', err);
      }
    };

    if ('__TAURI__' in window) {
      showWindow();
    }
  }, []);

  useEffect(() => {
    if (performanceMode) {
      document.body.classList.add('performance-mode');
    } else {
      document.body.classList.remove('performance-mode');
    }
  }, [performanceMode]);

  // Shift+R to force reload (dev helper)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.shiftKey && e.key.toLowerCase() === 'r' && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        window.location.reload();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Disable browser default shortcuts and interactions (生产环境)
  useEffect(() => {
    if (!import.meta.env.PROD) {
      return undefined;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
        const key = e.key.toLowerCase();
        const ctrlOrMeta = e.ctrlKey || e.metaKey;
        const shift = e.shiftKey;
        const alt = e.altKey;

        // 1. Refresh: F5, Ctrl+R, Ctrl+Shift+R
        if (key === 'f5' || (ctrlOrMeta && key === 'r')) {
          e.preventDefault();
          return;
        }

        // 2. Developer Tools: F12, Ctrl+Shift+I, Ctrl+Shift+J, Ctrl+Shift+C
        if (
          key === 'f12' ||
          (ctrlOrMeta && shift && (key === 'i' || key === 'j' || key === 'c'))
        ) {
          e.preventDefault();
          return;
        }

        // 3. Find: Ctrl+F, Ctrl+G
        if (ctrlOrMeta && (key === 'f' || key === 'g')) {
          e.preventDefault();
          return;
        }

        // 4. Print: Ctrl+P
        if (ctrlOrMeta && key === 'p') {
          e.preventDefault();
          return;
        }

        // 5. Save: Ctrl+S
        if (ctrlOrMeta && key === 's') {
          e.preventDefault();
          return;
        }

        // 6. Zoom: Ctrl++, Ctrl+-, Ctrl+0
        if (
          ctrlOrMeta &&
          (key === '+' || key === '-' || key === '=' || key === '0')
        ) {
          e.preventDefault();
          return;
        }

        // 7. Navigation: Alt+Left, Alt+Right (Browser Back/Forward)
        if (alt && (key === 'arrowleft' || key === 'arrowright')) {
          e.preventDefault();
          return;
        }

        // 8. New Tab/Window: Ctrl+T, Ctrl+N
        if (ctrlOrMeta && (key === 't' || key === 'n')) {
          e.preventDefault();
          return;
        }
      };

      // Disable Zoom via Scroll (Ctrl + Scroll)
      const handleWheel = (e: WheelEvent) => {
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault();
        }
      };

      // Disable Context Menu (Right Click)
      const handleContextMenu = (e: MouseEvent) => {
        // Allow context menu only on inputs if needed, or completely disable
        // For a POS, usually completely disable is safer unless specific areas need it
        if (
          import.meta.env.PROD ||
          !e.target ||
          (e.target as HTMLElement).tagName !== 'INPUT'
        ) {
          e.preventDefault();
        }
      };

      window.addEventListener('keydown', handleKeyDown);
      window.addEventListener('wheel', handleWheel, { passive: false });
      window.addEventListener('contextmenu', handleContextMenu);

      return () => {
        window.removeEventListener('keydown', handleKeyDown);
        window.removeEventListener('wheel', handleWheel);
        window.removeEventListener('contextmenu', handleContextMenu);
      };
  }, []);

  // 初始化未完成时显示 loading
  if (!isInitialized) {
    if (error) {
      return (
        <div className="min-h-screen flex items-center justify-center bg-gray-50">
          <div className="text-center max-w-md px-6">
            <div className="w-12 h-12 rounded-full bg-red-100 flex items-center justify-center mx-auto mb-4">
              <svg className="w-6 h-6 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
            </div>
            <h3 className="text-lg font-bold text-gray-800 mb-2">{t('app.init.error_title')}</h3>
            <p className="text-sm text-gray-600 mb-6">{error}</p>
            <button
              onClick={retry}
              className="px-6 py-2.5 bg-primary-500 text-white font-semibold rounded-xl hover:bg-primary-600 transition-colors"
            >
              {t('common.action.retry')}
            </button>
          </div>
        </div>
      );
    }

    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="w-8 h-8 border-4 border-primary-500/30 border-t-primary-500 rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <BrowserRouter>
      <NotificationProvider>
        {/* Global Toast Containers */}
        <ToastContainer />
        <ServerMessageToastContainer />
        <PermissionEscalationProvider />
        <SystemIssueDialog issue={currentIssue} onResolve={resolveIssue} />

        <Routes>
        {/* Activate & Setup Routes */}
        <Route path="/activate" element={<ActivateScreen />} />
        <Route path="/setup" element={<SetupScreen />} />

        {/* Status Routes */}
        <Route path="/status/activation-required" element={<ActivationRequiredScreen />} />
        <Route path="/status/subscription-blocked" element={<SubscriptionBlockedScreen />} />

        {/* Public Routes */}
        <Route path="/login" element={<LoginScreen />} />

        {/* Protected Routes */}
        <Route
          path="/pos"
          element={
            <ProtectedRoute>
              <ShiftGuard>
                <POSScreen />
              </ShiftGuard>
            </ProtectedRoute>
          }
        />

        {/* Debug Route */}
        <Route path="/debug/orders" element={<OrderDebug />} />

        {/* Default Route - handles first-run detection */}
        <Route path="/" element={<InitialRoute />} />
        <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </NotificationProvider>
    </BrowserRouter>
  );
};

export default App;
