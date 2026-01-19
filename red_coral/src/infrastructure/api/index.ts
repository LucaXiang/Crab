/**
 * API Infrastructure
 *
 * Tauri API Client for communicating with the embedded edge-server.
 * All API calls go through Tauri invoke commands.
 */

// Tauri API Client
export { TauriApiClient, createTauriClient, ApiError } from './tauri-client';

// 类型导出 (从 core/domain/types/api 重导出)
export * from '@/core/domain/types/api';
