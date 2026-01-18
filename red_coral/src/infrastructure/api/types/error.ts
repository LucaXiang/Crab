// Error code definitions matching the Rust project

export enum ErrorCode {
  // General errors 0xxx
  Success = 'E0000',
  Unknown = 'E0001',
  ValidationError = 'E0002',
  NotFound = 'E0003',
  AlreadyExists = 'E0004',
  BusinessRuleViolation = 'E0005',

  // User related errors 1xxx
  UserNotFound = 'E1001',
  UserAlreadyExists = 'E1002',
  InvalidCredentials = 'E1003',
  UserInactive = 'E1004',
  PasswordMismatch = 'E1005',
  InvalidInput = 'E1006',

  // Permission related errors 2xxx
  PermissionDenied = 'E2001',
  PermissionNotFound = 'E2002',
  RoleNotFound = 'E2003',

  // Auth related errors 3xxx
  AuthRequired = 'E3001',
  InvalidToken = 'E3002',
  TokenExpired = 'E3003',

  // Order related errors 4xxx
  OrderNotFound = 'E4001',
  OrderInvalidState = 'E4002',
  OrderAlreadyPaid = 'E4003',
  OrderItemNotFound = 'E4004',

  // Product related errors 5xxx
  ProductNotFound = 'E5001',
  ProductOutOfStock = 'E5002',
  ProductInvalidPrice = 'E5003',
  CategoryNotFound = 'E5004',
  SpecNotFound = 'E5005',

  // Payment related errors 6xxx
  PaymentNotFound = 'E6001',
  PaymentFailed = 'E6002',
  PaymentCancelled = 'E6003',

  // System errors 9xxx
  InternalError = 'E9001',
  DatabaseError = 'E9002',
  TransactionError = 'E9003',
  ConfigError = 'E9004',
}

/**
 * Error message translation keys for i18n
 */
export const ErrorMessageKeys: Record<ErrorCode, string> = {
  // General
  [ErrorCode.Success]: 'errors.general.success',
  [ErrorCode.Unknown]: 'errors.general.unknown',
  [ErrorCode.ValidationError]: 'errors.general.validationError',
  [ErrorCode.NotFound]: 'errors.general.notFound',
  [ErrorCode.AlreadyExists]: 'errors.general.alreadyExists',
  [ErrorCode.BusinessRuleViolation]: 'errors.general.businessRuleViolation',

  // User
  [ErrorCode.UserNotFound]: 'errors.user.notFound',
  [ErrorCode.UserAlreadyExists]: 'errors.user.alreadyExists',
  [ErrorCode.InvalidCredentials]: 'errors.user.invalidCredentials',
  [ErrorCode.UserInactive]: 'errors.user.inactive',
  [ErrorCode.PasswordMismatch]: 'errors.user.passwordMismatch',
  [ErrorCode.InvalidInput]: 'errors.user.invalidInput',

  // Permission
  [ErrorCode.PermissionDenied]: 'errors.permission.denied',
  [ErrorCode.PermissionNotFound]: 'errors.permission.notFound',
  [ErrorCode.RoleNotFound]: 'errors.permission.roleNotFound',

  // Auth
  [ErrorCode.AuthRequired]: 'errors.auth.required',
  [ErrorCode.InvalidToken]: 'errors.auth.invalidToken',
  [ErrorCode.TokenExpired]: 'errors.auth.tokenExpired',

  // Order
  [ErrorCode.OrderNotFound]: 'errors.order.notFound',
  [ErrorCode.OrderInvalidState]: 'errors.order.invalidState',
  [ErrorCode.OrderAlreadyPaid]: 'errors.order.alreadyPaid',
  [ErrorCode.OrderItemNotFound]: 'errors.order.itemNotFound',

  // Product
  [ErrorCode.ProductNotFound]: 'errors.product.notFound',
  [ErrorCode.ProductOutOfStock]: 'errors.product.outOfStock',
  [ErrorCode.ProductInvalidPrice]: 'errors.product.invalidPrice',
  [ErrorCode.CategoryNotFound]: 'errors.product.categoryNotFound',
  [ErrorCode.SpecNotFound]: 'errors.product.specNotFound',

  // Payment
  [ErrorCode.PaymentNotFound]: 'errors.payment.notFound',
  [ErrorCode.PaymentFailed]: 'errors.payment.failed',
  [ErrorCode.PaymentCancelled]: 'errors.payment.cancelled',

  // System
  [ErrorCode.InternalError]: 'errors.system.internalError',
  [ErrorCode.DatabaseError]: 'errors.system.databaseError',
  [ErrorCode.TransactionError]: 'errors.system.transactionError',
  [ErrorCode.ConfigError]: 'errors.system.configError',
};

export const HttpStatusMap: Record<ErrorCode, number> = {
  // Success
  [ErrorCode.Success]: 200,

  // Client errors 4xx
  [ErrorCode.ValidationError]: 400,
  [ErrorCode.InvalidInput]: 400,
  [ErrorCode.InvalidCredentials]: 401,
  [ErrorCode.PasswordMismatch]: 401,
  [ErrorCode.AuthRequired]: 401,
  [ErrorCode.InvalidToken]: 401,
  [ErrorCode.TokenExpired]: 401,
  [ErrorCode.PermissionDenied]: 403,
  [ErrorCode.UserInactive]: 403,
  [ErrorCode.NotFound]: 404,
  [ErrorCode.UserNotFound]: 404,
  [ErrorCode.PermissionNotFound]: 404,
  [ErrorCode.RoleNotFound]: 404,
  [ErrorCode.OrderNotFound]: 404,
  [ErrorCode.OrderItemNotFound]: 404,
  [ErrorCode.ProductNotFound]: 404,
  [ErrorCode.CategoryNotFound]: 404,
  [ErrorCode.SpecNotFound]: 404,
  [ErrorCode.PaymentNotFound]: 404,
  [ErrorCode.AlreadyExists]: 409,
  [ErrorCode.UserAlreadyExists]: 409,
  [ErrorCode.BusinessRuleViolation]: 422,
  [ErrorCode.OrderInvalidState]: 422,
  [ErrorCode.OrderAlreadyPaid]: 422,
  [ErrorCode.ProductOutOfStock]: 422,
  [ErrorCode.ProductInvalidPrice]: 422,
  [ErrorCode.PaymentFailed]: 422,
  [ErrorCode.PaymentCancelled]: 422,

  // Server errors 5xx
  [ErrorCode.Unknown]: 500,
  [ErrorCode.InternalError]: 500,
  [ErrorCode.DatabaseError]: 500,
  [ErrorCode.TransactionError]: 500,
  [ErrorCode.ConfigError]: 500,
};
