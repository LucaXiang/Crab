/**
 * Error Handling Utilities
 * Provides unified error message extraction and localization
 */

import { t } from '@/infrastructure/i18n';

/**
 * API Response error structure (matches Rust ApiResponse)
 */
export interface ApiErrorResponse {
  error_code: string | null;
  message: string;
  data?: unknown;
}

/**
 * Check if a response is an error response
 */
export function isErrorResponse(response: unknown): response is ApiErrorResponse {
  if (typeof response !== 'object' || response === null) return false;
  const obj = response as Record<string, unknown>;
  return obj.error_code !== undefined && obj.error_code !== null;
}

/**
 * Get localized error message from an API response
 *
 * Priority:
 * 1. Localized message for error_code (if exists in i18n)
 * 2. Raw message from response
 * 3. Default unknown error message
 *
 * @param response - The API response object
 * @returns Localized error message string
 */
export function getErrorMessage(response: ApiErrorResponse | unknown): string {
  // Handle non-object responses
  if (typeof response === 'string') {
    return response;
  }

  if (typeof response !== 'object' || response === null) {
    return t('error.codes.UNKNOWN_ERROR');
  }

  const errorResponse = response as ApiErrorResponse;

  // Try to get localized message for error code
  if (errorResponse.error_code) {
    const localizedKey = `error.codes.${errorResponse.error_code}`;
    const localized = t(localizedKey);

    // If translation exists (not returning the key itself), use it
    if (localized !== localizedKey) {
      return localized;
    }
  }

  // Fall back to raw message
  if (errorResponse.message) {
    return errorResponse.message;
  }

  // Ultimate fallback
  return t('error.codes.UNKNOWN_ERROR');
}

/**
 * Extract error from various error types (catch blocks, API responses, etc.)
 *
 * @param error - Any error type
 * @returns Localized error message string
 */
export function extractError(error: unknown): string {
  // Handle Error objects
  if (error instanceof Error) {
    return error.message;
  }

  // Handle API response objects
  if (typeof error === 'object' && error !== null) {
    return getErrorMessage(error);
  }

  // Handle string errors
  if (typeof error === 'string') {
    return error;
  }

  return t('error.codes.UNKNOWN_ERROR');
}

/**
 * Create a standardized error handler for async operations
 *
 * @param onError - Callback to handle the error message
 * @returns Error handler function
 */
export function createErrorHandler(onError: (message: string) => void) {
  return (error: unknown) => {
    const message = extractError(error);
    onError(message);
  };
}

/**
 * Common error codes for quick reference
 */
export const ErrorCodes = {
  // Auth
  AUTH_NOT_AUTHENTICATED: 'AUTH_NOT_AUTHENTICATED',
  AUTH_INVALID_CREDENTIALS: 'AUTH_INVALID_CREDENTIALS',
  AUTH_PERMISSION_DENIED: 'AUTH_PERMISSION_DENIED',

  // Bridge/Connection
  BRIDGE_NOT_INITIALIZED: 'BRIDGE_NOT_INITIALIZED',
  BRIDGE_NOT_CONNECTED: 'BRIDGE_NOT_CONNECTED',
  BRIDGE_TIMEOUT: 'BRIDGE_TIMEOUT',

  // Server
  SERVER_INTERNAL_ERROR: 'SERVER_INTERNAL_ERROR',
  SERVER_UNAVAILABLE: 'SERVER_UNAVAILABLE',

  // Validation
  VALIDATION_REQUIRED_FIELD: 'VALIDATION_REQUIRED_FIELD',
  VALIDATION_INVALID_FORMAT: 'VALIDATION_INVALID_FORMAT',

  // General
  UNKNOWN_ERROR: 'UNKNOWN_ERROR',
  NETWORK_ERROR: 'NETWORK_ERROR',
} as const;

export type ErrorCode = typeof ErrorCodes[keyof typeof ErrorCodes];
