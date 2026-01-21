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
 * 双重验证：
 * 1. 检查 isAuthenticated (前端 auth store) - 优先检查
 * 2. 检查 AppState (后端状态机)
 */
export const ProtectedRoute: React.FC<ProtectedRouteProps> = ({ children }) => {
  const fetchAppState = useBridgeStore((state) => state.fetchAppState);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const location = useLocation();
  const [isChecking, setIsChecking] = useState(true);
  const [validatedState, setValidatedState] = useState<AppState | null>(null);

  // 在挂载时验证 AppState
  useEffect(() => {
    const validateAuth = async () => {
      const state = await fetchAppState();
      setValidatedState(state);
      setIsChecking(false);
    };
    validateAuth();
  }, [fetchAppState]);

  // 显示加载状态
  if (isChecking) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="w-8 h-8 border-4 border-[#FF5E5E]/30 border-t-[#FF5E5E] rounded-full animate-spin" />
      </div>
    );
  }

  // 优先检查前端 isAuthenticated
  if (!isAuthenticated) {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }

  // 再检查后端 AppState
  if (!AppStateHelpers.canAccessPOS(validatedState)) {
    const redirectTo = AppStateHelpers.getRouteForState(validatedState);
    return <Navigate to={redirectTo} state={{ from: location }} replace />;
  }

  return <>{children}</>;
};
