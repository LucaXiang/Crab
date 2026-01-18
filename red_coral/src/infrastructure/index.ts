/**
 * Infrastructure Layer - Main index
 * Re-exports all infrastructure modules
 */

// API - Main export for HTTP RESTful API
export { createClient, type ApiClient, ApiError, setAuthToken, clearAuthToken } from './api';
export * from './api/types';

// i18n
export * from './i18n';

// Print
export * from './print';

// Persistence
export * from './persistence';
