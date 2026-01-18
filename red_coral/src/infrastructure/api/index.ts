/**
 * API Infrastructure
 *
 * This module provides the API client for communicating with the embedded Axum server.
 * The server runs on http://localhost:3000 by default.
 */

export { ApiClient, ApiError, createClient, setAuthToken, clearAuthToken } from './client';
export * from './types';
