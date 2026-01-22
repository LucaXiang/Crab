/**
 * Unified error handling utilities
 */

// Re-export from generated file
export * from '@/generated/error-codes';
import { ErrorCode, isError, isSuccess, type ApiResponse } from '@/generated/error-codes';

import { t } from '@/infrastructure/i18n';

/**
 * Get localized error message from API response or unknown error
 * Looks up translation by numeric error code when available
 */
export function getErrorMessage(response: ApiResponse | unknown): string {
  // Handle string errors directly
  if (typeof response === 'string') {
    return response;
  }

  // Handle non-object responses
  if (typeof response !== 'object' || response === null) {
    return t('errors.1'); // Unknown error
  }

  const obj = response as Record<string, unknown>;
  const details = obj.details as Record<string, unknown> | undefined;

  // Helper to append details to message
  const withDetails = (msg: string): string => {
    if (!details || Object.keys(details).length === 0) return msg;
    const detailStr = Object.entries(details)
      .map(([k, v]) => `${k}: ${v}`)
      .join(', ');
    return `${msg} (${detailStr})`;
  };

  // Handle ApiResponse/ApiError format with numeric code
  if ('code' in obj && (typeof obj.code === 'number' || obj.code === null)) {
    const code = obj.code as number | null;
    const message = (obj.message as string) || '';

    if (isSuccess(code)) {
      return message;
    }

    // Try to get localized message for numeric error code
    if (code !== null && code !== undefined) {
      const localizedKey = `errors.${code}`;
      const localized = t(localizedKey);

      // If translation exists (not returning the key itself), use it
      if (localized !== localizedKey) {
        return withDetails(localized);
      }
    }

    return withDetails(message);
  }

  // Handle simple message objects (like Error instances)
  if ('message' in obj && typeof obj.message === 'string') {
    return withDetails(obj.message);
  }

  return t('errors.1'); // Unknown error
}

/**
 * Extract error from unknown error type
 */
export function extractError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message);
  }
  return t('errors.1'); // Unknown error
}

/**
 * Create error handler callback
 */
export function createErrorHandler(onError: (message: string) => void) {
  return (error: unknown) => {
    const message = extractError(error);
    onError(message);
  };
}

// Re-export for convenience
export { ErrorCode, isError, isSuccess };
export type { ApiResponse };
