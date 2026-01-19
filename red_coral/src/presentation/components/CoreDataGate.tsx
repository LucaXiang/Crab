import React from 'react';
import { usePreloadCoreData } from '@/core/hooks/usePreloadCoreData';

interface CoreDataGateProps {
  children: React.ReactNode;
}

/**
 * CoreDataGate - 核心数据预加载门控组件
 *
 * 在渲染子组件之前预加载核心数据（product, category, zone, table）。
 * 用于包装需要这些数据的路由（如 POS 页面）。
 */
export const CoreDataGate: React.FC<CoreDataGateProps> = ({ children }) => {
  const ready = usePreloadCoreData();

  if (!ready) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="w-12 h-12 border-4 border-[#FF5E5E]/30 border-t-[#FF5E5E] rounded-full animate-spin mx-auto mb-4" />
          <p className="text-gray-500 text-sm">Loading data...</p>
        </div>
      </div>
    );
  }

  return <>{children}</>;
};
