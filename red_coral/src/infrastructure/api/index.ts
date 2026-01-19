/**
 * API Infrastructure
 *
 * This module provides the API client for communicating with the embedded Axum server.
 * The server runs on http://localhost:3000 by default.
 *
 * Smart factory automatically selects the appropriate client:
 * - Tauri environment: TauriApiClient (via invoke)
 * - Other environments: ApiClient (direct HTTP, for development/testing)
 */

// 导入 Tauri API Client (用于 Tauri 环境)
import { TauriApiClient, createTauriClient, ApiError } from './tauri-client';

// 导入原 HTTP Client (用于开发/测试)
import { ApiClient, createClient, setAuthToken, clearAuthToken } from './client';

// 重新导出
export { TauriApiClient, createTauriClient, ApiError };
export { ApiClient, createClient, setAuthToken, clearAuthToken };

// 类型导出 (从 core/domain/types/api 重导出)
export * from '@/core/domain/types/api';

// 环境检测
export function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

// 智能工厂函数
export function createApiClient() {
  if (isTauriEnvironment()) {
    return createTauriClient();
  }
  return createClient();
}
