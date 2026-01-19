/**
 * Infrastructure Layer - Main index
 * Re-exports all infrastructure modules
 */

// API - Tauri API Client
export { createTauriClient, TauriApiClient, ApiError } from './api';

// i18n
export * from './i18n';

// Print
export * from './print';

// Label
export * from './label';

// Data Source
export * from './dataSource';

// Persistence
export * from './persistence';
