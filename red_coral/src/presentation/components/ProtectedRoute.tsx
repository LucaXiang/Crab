import React from 'react';
import { Navigate, useLocation } from 'react-router-dom';
import { useBridgeStore, AppStateHelpers } from '@/core/stores/bridge';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';

interface ProtectedRouteProps {
  children: React.ReactNode;
}

/**
 * ProtectedRoute - 路由守卫组件 (同步检查，无 API 调用)
 *
 * InitialRoute 已完成所有初始化工作：
 * - fetchAppState() 获取后端状态机
 * - fetchCurrentSession() 恢复 auth store
 *
 * 这里只做同步状态检查，避免重复 API 调用。
 */
export const ProtectedRoute: React.FC<ProtectedRouteProps> = ({ children }) => {
  const appState = useBridgeStore((state) => state.appState);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const location = useLocation();

  // 同步检查：无需等待，直接使用 store 中已有状态
  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }

  if (!AppStateHelpers.canAccessPOS(appState)) {
    const redirectTo = AppStateHelpers.getRouteForState(appState);
    return <Navigate to={redirectTo} state={{ from: location }} replace />;
  }

  return <>{children}</>;
};
