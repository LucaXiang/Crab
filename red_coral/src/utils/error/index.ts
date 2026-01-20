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

  // Handle new ApiResponse format with numeric code
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
        return localized;
      }
    }

    return message;
  }

  // Handle legacy ApiErrorResponse format with string error_code
  if ('error_code' in obj && obj.error_code !== null) {
    const errorCode = obj.error_code as string;
    const message = (obj.message as string) || '';

    // Try legacy string-based error code lookup
    const legacyLocalizedKey = `error.codes.${errorCode}`;
    const legacyLocalized = t(legacyLocalizedKey);

    if (legacyLocalized !== legacyLocalizedKey) {
      return legacyLocalized;
    }

    return message || t('errors.1');
  }

  // Handle simple message objects
  if ('message' in obj && typeof obj.message === 'string') {
    return obj.message;
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

/**
 * Legacy compatibility mapping - maps old string codes to new numeric codes
 */
export const LegacyErrorCodes: Record<string, number> = {
  // Auth
  'AUTH_NOT_AUTHENTICATED': ErrorCode.NotAuthenticated,
  'AUTH_INVALID_CREDENTIALS': ErrorCode.InvalidCredentials,
  'AUTH_TOKEN_EXPIRED': ErrorCode.TokenExpired,
  'AUTH_TOKEN_INVALID': ErrorCode.TokenInvalid,
  'AUTH_SESSION_EXPIRED': ErrorCode.SessionExpired,
  'AUTH_PERMISSION_DENIED': ErrorCode.PermissionDenied,
  'AUTH_USER_DISABLED': ErrorCode.AccountDisabled,

  // Bridge
  'BRIDGE_NOT_INITIALIZED': ErrorCode.BridgeNotInitialized,
  'BRIDGE_NOT_CONNECTED': ErrorCode.BridgeNotConnected,
  'BRIDGE_CONNECTION_FAILED': ErrorCode.BridgeConnectionFailed,
  'BRIDGE_TIMEOUT': ErrorCode.TimeoutError,

  // Server
  'SERVER_INTERNAL_ERROR': ErrorCode.InternalError,
  'SERVER_DATABASE_ERROR': ErrorCode.DatabaseError,
  'SERVER_UNAVAILABLE': ErrorCode.NetworkError,

  // Validation
  'VALIDATION_REQUIRED_FIELD': ErrorCode.RequiredField,
  'VALIDATION_INVALID_FORMAT': ErrorCode.InvalidFormat,
  'VALIDATION_VALUE_OUT_OF_RANGE': ErrorCode.ValueOutOfRange,

  // General
  'UNKNOWN_ERROR': ErrorCode.Unknown,
  'NETWORK_ERROR': ErrorCode.NetworkError,

  // Order
  'ORDER_NOT_FOUND': ErrorCode.OrderNotFound,
  'ORDER_ALREADY_COMPLETED': ErrorCode.OrderAlreadyCompleted,
  'ORDER_ALREADY_VOIDED': ErrorCode.OrderAlreadyVoided,
  'ORDER_HAS_PAYMENTS': ErrorCode.OrderHasPayments,

  // Payment
  'PAYMENT_FAILED': ErrorCode.PaymentFailed,
  'PAYMENT_INSUFFICIENT_AMOUNT': ErrorCode.PaymentInsufficientAmount,
  'PAYMENT_INVALID_METHOD': ErrorCode.PaymentInvalidMethod,

  // Product
  'PRODUCT_NOT_FOUND': ErrorCode.ProductNotFound,
  'PRODUCT_INVALID_PRICE': ErrorCode.ProductInvalidPrice,
  'CATEGORY_NOT_FOUND': ErrorCode.CategoryNotFound,
  'CATEGORY_HAS_PRODUCTS': ErrorCode.CategoryHasProducts,

  // Table
  'TABLE_NOT_FOUND': ErrorCode.TableNotFound,
  'TABLE_OCCUPIED': ErrorCode.TableOccupied,
  'ZONE_NOT_FOUND': ErrorCode.ZoneNotFound,
  'ZONE_HAS_TABLES': ErrorCode.ZoneHasTables,

  // Employee
  'EMPLOYEE_NOT_FOUND': ErrorCode.EmployeeNotFound,
  'EMPLOYEE_USERNAME_EXISTS': ErrorCode.EmployeeUsernameExists,
  'EMPLOYEE_CANNOT_DELETE_SELF': ErrorCode.EmployeeCannotDeleteSelf,
  'ROLE_NOT_FOUND': ErrorCode.RoleNotFound,

  // Tenant
  'TENANT_NOT_SELECTED': ErrorCode.TenantNotSelected,
  'TENANT_NOT_FOUND': ErrorCode.TenantNotFound,
  'TENANT_ACTIVATION_FAILED': ErrorCode.ActivationFailed,

  // Printer
  'PRINTER_NOT_AVAILABLE': ErrorCode.PrinterNotAvailable,
  'PRINTER_PRINT_FAILED': ErrorCode.PrintFailed,
};

/**
 * Convert legacy string error code to numeric code
 */
export function legacyCodeToNumeric(legacyCode: string): number {
  return LegacyErrorCodes[legacyCode] ?? ErrorCode.Unknown;
}

/**
 * Check if response indicates an error (for legacy API responses with string codes)
 */
export function isLegacyErrorResponse(response: unknown): boolean {
  if (typeof response !== 'object' || response === null) return false;
  const obj = response as Record<string, unknown>;
  return obj.error_code !== undefined && obj.error_code !== null;
}

// Re-export for convenience
export { ErrorCode, isError, isSuccess };
export type { ApiResponse };
