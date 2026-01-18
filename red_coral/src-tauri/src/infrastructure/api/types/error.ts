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

export const ErrorMessages: Record<ErrorCode, string> = {
  // General
  [ErrorCode.Success]: '操作成功',
  [ErrorCode.Unknown]: '未知错误',
  [ErrorCode.ValidationError]: '参数验证失败',
  [ErrorCode.NotFound]: '资源不存在',
  [ErrorCode.AlreadyExists]: '资源已存在',
  [ErrorCode.BusinessRuleViolation]: '业务规则冲突',

  // User
  [ErrorCode.UserNotFound]: '用户不存在',
  [ErrorCode.UserAlreadyExists]: '用户已存在',
  [ErrorCode.InvalidCredentials]: '用户名或密码错误',
  [ErrorCode.UserInactive]: '用户已被禁用',
  [ErrorCode.PasswordMismatch]: '密码错误',
  [ErrorCode.InvalidInput]: '输入参数无效',

  // Permission
  [ErrorCode.PermissionDenied]: '权限不足，无法执行此操作',
  [ErrorCode.PermissionNotFound]: '权限不存在',
  [ErrorCode.RoleNotFound]: '角色不存在',

  // Auth
  [ErrorCode.AuthRequired]: '请先登录',
  [ErrorCode.InvalidToken]: '无效的认证令牌',
  [ErrorCode.TokenExpired]: '认证令牌已过期',

  // Order
  [ErrorCode.OrderNotFound]: '订单不存在',
  [ErrorCode.OrderInvalidState]: '订单状态无效，无法执行此操作',
  [ErrorCode.OrderAlreadyPaid]: '订单已支付',
  [ErrorCode.OrderItemNotFound]: '订单项不存在',

  // Product
  [ErrorCode.ProductNotFound]: '产品不存在',
  [ErrorCode.ProductOutOfStock]: '产品库存不足',
  [ErrorCode.ProductInvalidPrice]: '产品价格无效',
  [ErrorCode.CategoryNotFound]: '分类不存在',
  [ErrorCode.SpecNotFound]: '规格不存在',

  // Payment
  [ErrorCode.PaymentNotFound]: '支付记录不存在',
  [ErrorCode.PaymentFailed]: '支付失败',
  [ErrorCode.PaymentCancelled]: '支付已取消',

  // System
  [ErrorCode.InternalError]: '系统内部错误',
  [ErrorCode.DatabaseError]: '数据库错误',
  [ErrorCode.TransactionError]: '事务处理错误',
  [ErrorCode.ConfigError]: '配置错误',
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
