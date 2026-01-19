/**
 * Infrastructure Layer - Main index
 * Re-exports all infrastructure modules
 */

// API - Main export for HTTP RESTful API
export { createClient, createApiClient, type ApiClient, ApiError, setAuthToken, clearAuthToken } from './api';

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
