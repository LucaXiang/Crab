import React, { useEffect, useState } from 'react';
import { Navigate, useLocation } from 'react-router-dom';
import { useBridgeStore, AppStateHelpers, type AppState } from '@/core/stores/bridge';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';

interface ProtectedRouteProps {
  children: React.ReactNode;
}

/**
 * ProtectedRoute - 路由守卫组件
 *
 * 页面刷新时 Zustand 内存状态丢失，需从后端恢复会话：
 * 1. fetchAppState() 获取后端状态机
 * 2. 若后端已认证，fetchCurrentSession() 恢复 auth store
 * 3. 再检查 isAuthenticated 决定路由
 */
export const ProtectedRoute: React.FC<ProtectedRouteProps> = ({ children }) => {
  const { fetchAppState, fetchCurrentSession } = useBridgeStore();
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const location = useLocation();
  const [isChecking, setIsChecking] = useState(true);
  const [validatedState, setValidatedState] = useState<AppState | null>(null);

  useEffect(() => {
    const validateAuth = async () => {
      const state = await fetchAppState();
      setValidatedState(state);

      // 页面刷新后 auth store 已重置，需从后端恢复会话
      if (!useAuthStore.getState().isAuthenticated) {
        if (state?.type === 'ServerAuthenticated' || state?.type === 'ClientAuthenticated') {
          const session = await fetchCurrentSession();
          if (session) {
            useAuthStore.getState().setUser({
              id: session.user_info.id,
              username: session.user_info.username,
              display_name: session.user_info.display_name,
              role_id: session.user_info.role_id,
              role_name: session.user_info.role_name,
              permissions: session.user_info.permissions,
              is_system: session.user_info.is_system,
              is_active: true,
            });
          }
        }
      }

      setIsChecking(false);
    };
    validateAuth();
  }, [fetchAppState, fetchCurrentSession]);

  if (isChecking) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="w-8 h-8 border-4 border-primary-500/30 border-t-primary-500 rounded-full animate-spin" />
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }

  if (!AppStateHelpers.canAccessPOS(validatedState)) {
    const redirectTo = AppStateHelpers.getRouteForState(validatedState);
    return <Navigate to={redirectTo} state={{ from: location }} replace />;
  }

  return <>{children}</>;
};
