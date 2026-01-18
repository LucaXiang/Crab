/**
 * API Response Validator
 *
 * Provides Zod validation wrapper for direct Tauri invoke calls.
 * All API responses should be validated before use to ensure data integrity.
 */

import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import { logger } from '@/utils/logger';

// Re-export base schemas from validators
import {
  currencySchema,
  nonNegativeIntSchema,
  positiveIntSchema,
} from '@/core/domain/validators';

/**
 * API validation error with context
 */
export class ApiValidationError extends Error {
  constructor(
    message: string,
    public readonly apiName: string,
    public readonly originalError?: unknown
  ) {
    super(message);
    this.name = 'ApiValidationError';
  }
}

// ============================================================================
// Validation Helpers
// ============================================================================

/**
 * Validates API response against a schema and returns parsed data
 */
export async function validateApiResponse<T extends z.ZodType>(
  schema: T,
  apiName: string,
  data: unknown
): Promise<z.infer<T>> {
  const result = schema.safeParse(data);
  if (!result.success) {
    const errorMsg = `Invalid response from ${apiName}: ${result.error.message}`;
    logger.error(errorMsg, result.error, { component: 'ApiValidator', apiName });
    throw new ApiValidationError(errorMsg, apiName, result.error);
  }
  return result.data;
}

/**
 * Wraps a Tauri invoke call with validation
 */
export function createValidatedApiCall<T extends z.ZodType>(
  schema: T,
  commandName: string
): () => Promise<z.infer<T>> {
  return async () => {
    const data = await invoke(commandName);
    return validateApiResponse(schema, commandName, data);
  };
}

/**
 * Creates a validated API call with parameters
 */
export function createValidatedApiCallWithParams<T extends z.ZodType, P extends Record<string, unknown>>(
  schema: T,
  commandName: string
): (params: P) => Promise<z.infer<T>> {
  return async (params: P) => {
    const data = await invoke(commandName, params);
    return validateApiResponse(schema, commandName, data);
  };
}

/**
 * Type-safe wrapper for invoke with validation
 */
export async function invokeWithValidation<T extends z.ZodType>(
  schema: T,
  command: string,
  args?: Record<string, unknown>
): Promise<z.infer<T>> {
  const data = await invoke(command, args);
  return validateApiResponse(schema, command, data);
}

// ============================================================================
// Price Adjustment Schemas
// ============================================================================

/**
 * Adjustment config time range schema
 */
const adjustmentConfigTimeRangeSchema = z.object({
  startTime: z.string(),
  endTime: z.string(),
  daysOfWeek: z.array(z.number()).optional(),
});

/**
 * Adjustment config quantity range schema
 */
const adjustmentConfigQuantityRangeSchema = z.object({
  minQuantity: z.number(),
  maxQuantity: z.number().optional(),
});

/**
 * Adjustment config conditions schema
 */
const adjustmentConfigConditionsSchema = z.object({
  minOrderAmount: z.number().optional(),
  maxOrderAmount: z.number().optional(),
  applyToDiscountedItems: z.boolean().optional(),
});

/**
 * Adjustment config schema
 */
export const adjustmentConfigSchema = z.object({
  timeRange: adjustmentConfigTimeRangeSchema.optional(),
  quantityRange: adjustmentConfigQuantityRangeSchema.optional(),
  conditions: adjustmentConfigConditionsSchema.optional(),
});

/**
 * Adjustment rule schema
 */
export const adjustmentRuleSchema = z.object({
  id: z.number().int().positive(),
  name: z.string().min(1),
  displayName: z.string().optional(),
  receiptName: z.string().optional(),
  ruleType: z.enum(['discount', 'special_price', 'surcharge']),
  adjustmentType: z.enum([
    'percentage_discount',
    'fixed_discount',
    'percentage_surcharge',
    'fixed_surcharge',
  ]),
  adjustmentValue: z.number(),
  scope: z.enum(['global', 'category', 'product', 'order']),
  targetId: z.string().optional(),
  timeMode: z.enum(['onetime', 'recurring', 'permanent']).optional(),
  validFrom: z.number().optional(),
  validTo: z.number().optional(),
  description: z.string().optional(),
  priority: z.number().optional(),
  status: z.enum(['active', 'inactive']),
  config: adjustmentConfigSchema.optional(),
  createdAt: z.string().optional(),
  updatedAt: z.string().optional(),
});

/**
 * Adjustment rule list schema
 */
export const adjustmentRuleListSchema = z.array(adjustmentRuleSchema);

// ============================================================================
// Validated Price Adjustment API
// ============================================================================

/**
 * Fetch all price adjustment rules with validation
 */
export async function fetchAdjustmentRules(): Promise<z.infer<typeof adjustmentRuleSchema>[]> {
  return invokeWithValidation(adjustmentRuleListSchema, 'fetch_price_adjustment_rules');
}

/**
 * Get a single price adjustment rule with validation
 */
export async function getAdjustmentRule(id: number): Promise<z.infer<typeof adjustmentRuleSchema>> {
  return invokeWithValidation(adjustmentRuleSchema, 'get_price_adjustment_rule', { id });
}

/**
 * Create a new price adjustment rule (no validation on response)
 */
export async function createAdjustmentRule(
  params: Record<string, unknown>
): Promise<z.infer<typeof adjustmentRuleSchema>> {
  const data = await invoke('create_price_adjustment_rule', params);
  return validateApiResponse(adjustmentRuleSchema, 'create_price_adjustment_rule', data);
}

/**
 * Update a price adjustment rule (no validation on response)
 */
export async function updateAdjustmentRule(
  params: Record<string, unknown>
): Promise<z.infer<typeof adjustmentRuleSchema>> {
  const data = await invoke('update_price_adjustment_rule', params);
  return validateApiResponse(adjustmentRuleSchema, 'update_price_adjustment_rule', data);
}

/**
 * Toggle a price adjustment rule active status
 */
export async function toggleAdjustmentRule(id: number): Promise<void> {
  return invoke('toggle_price_adjustment_rule', { id });
}

/**
 * Delete a price adjustment rule
 */
export async function deleteAdjustmentRule(id: number): Promise<void> {
  return invoke('delete_price_adjustment_rule', { id });
}

/**
 * Get applicable price adjustment rules for a product with validation
 */
export async function getApplicableAdjustmentRules(
  productId: string
): Promise<z.infer<typeof adjustmentRuleSchema>[]> {
  return invokeWithValidation(adjustmentRuleListSchema, 'get_applicable_price_adjustment_rules', { productId });
}

/**
 * Get global price adjustment rules with validation
 */
export async function getGlobalAdjustmentRules(): Promise<z.infer<typeof adjustmentRuleSchema>[]> {
  return invokeWithValidation(adjustmentRuleListSchema, 'get_global_price_adjustment_rules');
}

// ============================================================================
// Re-export Types
// ============================================================================

export type AdjustmentRule = z.infer<typeof adjustmentRuleSchema>;
export type AdjustmentConfig = z.infer<typeof adjustmentConfigSchema>;

// ============================================================================
// Statistics Schemas
// ============================================================================

/**
 * Overview statistics schema (matches Rust OverviewStats)
 */
export const overviewStatsSchema = z.object({
  todayRevenue: currencySchema,
  todayOrders: nonNegativeIntSchema,
  todayCustomers: nonNegativeIntSchema,
  averageOrderValue: currencySchema,
  cashRevenue: currencySchema,
  cardRevenue: currencySchema,
  otherRevenue: currencySchema,
  voidedOrders: nonNegativeIntSchema,
  voidedAmount: currencySchema,
  totalDiscount: currencySchema,
  avgGuestSpend: currencySchema,
  avgDiningTime: z.number().optional(),
});

/**
 * Revenue trend point schema (matches Rust RevenueTrendPoint)
 */
export const revenueTrendPointSchema = z.object({
  time: z.string().min(1),
  value: currencySchema,
});

/**
 * Category sale schema (matches Rust CategorySale)
 */
export const categorySaleSchema = z.object({
  name: z.string().min(1),
  value: currencySchema,
  color: z.string().min(1),
});

/**
 * Top product schema (matches Rust TopProduct)
 */
export const topProductSchema = z.object({
  name: z.string().min(1),
  sales: nonNegativeIntSchema,
});

/**
 * Full statistics response schema (matches Rust StatisticsResponse)
 */
export const statisticsResponseSchema = z.object({
  overview: overviewStatsSchema,
  revenueTrend: z.array(revenueTrendPointSchema),
  categorySales: z.array(categorySaleSchema),
  topProducts: z.array(topProductSchema),
});

// ============================================================================
// Validated Statistics API
// ============================================================================

// StatisticsResponse type definition
export interface StatisticsResponse {
  overview: {
    todayRevenue: number;
    todayOrders: number;
    todayCustomers: number;
    averageOrderValue: number;
    cashRevenue: number;
    cardRevenue: number;
    otherRevenue: number;
    voidedOrders: number;
    voidedAmount: number;
    totalDiscount: number;
    avgGuestSpend: number;
    avgDiningTime?: number;
  };
  revenueTrend: Array<{ time: string; value: number }>;
  categorySales: Array<{ name: string; value: number; color: string }>;
  topProducts: Array<{ name: string; sales: number }>;
}

// ============================================================================
// Auth Schemas
// ============================================================================

/**
 * Role permissions schema
 */
export const rolePermissionsSchema = z.object({
  role: z.string().min(1),
  permissions: z.array(z.string()),
});

/**
 * User session schema (matches Rust UserSession)
 */
export const userSessionSchema = z.object({
  user: z.object({
    id: z.string().min(1),
    username: z.string().min(1),
    displayName: z.string().min(1),
    role: z.string().min(1),
    avatar: z.string().optional(),
    isActive: z.boolean(),
    createdAt: z.number(),
    updatedAt: z.number(),
    createdBy: z.string().optional(),
    lastLogin: z.number().optional(),
  }),
  token: z.string().optional(),
});

// ============================================================================
// Validated Auth API
// ============================================================================

/**
 * Authenticate user with validation
 */
export async function authenticateUser(
  username: string,
  password: string
): Promise<z.infer<typeof userSessionSchema>> {
  return invokeWithValidation(
    userSessionSchema,
    'authenticate_user',
    { username, password }
  );
}

/**
 * Get role permissions with validation
 */
export async function getRolePermissions(role: string): Promise<string[]> {
  const result = await invokeWithValidation(
    rolePermissionsSchema,
    'get_role_permissions',
    { role }
  );
  return result.permissions;
}

// ============================================================================
// User Schemas
// ============================================================================

/**
 * User schema (matches Rust User)
 */
export const userSchema = z.object({
  id: z.string().min(1),
  uuid: z.string().min(1),
  username: z.string().min(1),
  displayName: z.string().min(1),
  role: z.string().min(1),
  isActive: z.boolean(),
  createdAt: z.number(),
  updatedAt: z.number(),
  createdBy: z.string().optional(),
  lastLogin: z.number().optional(),
  avatar: z.string().optional(),
});

/**
 * User list schema
 */
export const userListSchema = z.array(userSchema);

/**
 * Fetch all users with validation
 */
export async function fetchAllUsers(): Promise<z.infer<typeof userSchema>[]> {
  return invokeWithValidation(userListSchema, 'fetch_all_users');
}

/**
 * Create user with validation
 */
export async function createUser(
  input: Record<string, unknown>,
  creatorId: string
): Promise<z.infer<typeof userSchema>> {
  return invokeWithValidation(userSchema, 'create_user', { input, creatorId });
}

/**
 * Update user with validation
 */
export async function updateUser(
  userId: string,
  input: Record<string, unknown>
): Promise<z.infer<typeof userSchema>> {
  return invokeWithValidation(userSchema, 'update_user', { userId, input });
}

/**
 * Fetch statistics with validation
 */
export async function getStatistics(
  timeRange: string,
  startDate?: string,
  endDate?: string
): Promise<StatisticsResponse> {
  const params: Record<string, unknown> = { timeRange };
  if (startDate) params.startDate = startDate;
  if (endDate) params.endDate = endDate;

  return invokeWithValidation(statisticsResponseSchema, 'get_statistics', params) as Promise<StatisticsResponse>;
}

// ============================================================================
// Sales Report Schemas
// ============================================================================

/**
 * Sales report item schema (matches Rust SalesReportItem)
 */
export const salesReportItemSchema = z.object({
  orderId: positiveIntSchema,
  receiptNumber: z.string().nullable(),
  date: z.string().min(1),
  total: currencySchema,
  status: z.string().min(1),
});

/**
 * Sales report response schema (matches Rust SalesReportResponse)
 */
export const salesReportResponseSchema = z.object({
  items: z.array(salesReportItemSchema),
  total: nonNegativeIntSchema,
  page: positiveIntSchema,
  pageSize: positiveIntSchema,
  totalPages: positiveIntSchema,
});

/**
 * Fetch sales report with validation
 */
export interface SalesReportResponse {
  items: Array<{ orderId: number; receiptNumber: string | null; date: string; total: number; status: string }>;
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export async function getSalesReport(
  timeRange: string,
  page: number = 1,
  startDate?: string,
  endDate?: string
): Promise<SalesReportResponse> {
  const params: Record<string, unknown> = { timeRange, page };
  if (startDate) params.startDate = startDate;
  if (endDate) params.endDate = endDate;

  return invokeWithValidation(salesReportResponseSchema, 'get_sales_report', params) as Promise<SalesReportResponse>;
}
