import React, { useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { useBridgeStore, AppStateHelpers } from '@/core/stores/bridge';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useSyncListener, useConnectionRecovery, useOrderEventListener, useSyncConnection, useShiftRecovery, useSystemIssueGuard } from '@/core/hooks';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

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

// Initial route component that handles first-run detection and mode auto-start
// 使用新的 AppState 状态机进行路由决策
// 参考设计文档: docs/plans/2026-01-18-application-state-machine.md
const InitialRoute: React.FC = () => {
  const {
    appState,
    fetchTenants,
    fetchAppState,
    fetchCurrentSession,
  } = useBridgeStore();
  const [isChecking, setIsChecking] = useState(true);

  useEffect(() => {
    const init = async () => {
      // 1. 获取租户列表
      await fetchTenants();

      // 2. 获取当前应用状态
      await fetchAppState();

      // 3. 如果状态是 Uninitialized 且有租户，说明后端已启动但模式未选择
      // 不再自动启动 Server 模式，让用户通过 Setup 页面选择
      // 后端的 restore_last_session 会根据 config.current_mode 自动恢复

      // 4. 尝试恢复缓存的员工会话 (从后端获取)
      // 并同步 auth store（以后端为唯一认证状态来源）
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

      setIsChecking(false);
    };
    init();
  }, [fetchTenants, fetchAppState, fetchCurrentSession]);

  if (isChecking) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="w-8 h-8 border-4 border-primary-500/30 border-t-primary-500 rounded-full animate-spin" />
      </div>
    );
  }

  // 使用 AppStateHelpers 确定路由
  const targetRoute = AppStateHelpers.getRouteForState(appState);
  return <Navigate to={targetRoute} replace />;
};

const App: React.FC = () => {
  const performanceMode = useSettingsStore((state) => state.performanceMode);

  // 挂载同步相关 hooks
  useSyncListener();
  useConnectionRecovery();
  useSyncConnection();

  // 挂载订单事件监听 hook (Event Sourcing)
  useOrderEventListener();

  // 启动时自动恢复跨营业日的僵尸班次
  useShiftRecovery();

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

  // Disable browser default shortcuts and interactions
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
