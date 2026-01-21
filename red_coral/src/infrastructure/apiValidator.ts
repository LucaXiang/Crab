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
async function validateApiResponse<T extends z.ZodType>(
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
 * Type-safe wrapper for invoke with validation
 */
async function invokeWithValidation<T extends z.ZodType>(
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

const adjustmentConfigTimeRangeSchema = z.object({
  startTime: z.string(),
  endTime: z.string(),
  daysOfWeek: z.array(z.number()).optional(),
});

const adjustmentConfigQuantityRangeSchema = z.object({
  minQuantity: z.number(),
  maxQuantity: z.number().optional(),
});

const adjustmentConfigConditionsSchema = z.object({
  minOrderAmount: z.number().optional(),
  maxOrderAmount: z.number().optional(),
  applyToDiscountedItems: z.boolean().optional(),
});

export const adjustmentConfigSchema = z.object({
  timeRange: adjustmentConfigTimeRangeSchema.optional(),
  quantityRange: adjustmentConfigQuantityRangeSchema.optional(),
  conditions: adjustmentConfigConditionsSchema.optional(),
});

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

export const adjustmentRuleListSchema = z.array(adjustmentRuleSchema);

// ============================================================================
// Validated Price Adjustment API
// ============================================================================

export async function fetchAdjustmentRules(): Promise<z.infer<typeof adjustmentRuleSchema>[]> {
  return invokeWithValidation(adjustmentRuleListSchema, 'fetch_price_adjustment_rules');
}

export async function getAdjustmentRule(id: number): Promise<z.infer<typeof adjustmentRuleSchema>> {
  return invokeWithValidation(adjustmentRuleSchema, 'get_price_adjustment_rule', { id });
}

export async function createAdjustmentRule(
  params: Record<string, unknown>
): Promise<z.infer<typeof adjustmentRuleSchema>> {
  const data = await invoke('create_price_adjustment_rule', params);
  return validateApiResponse(adjustmentRuleSchema, 'create_price_adjustment_rule', data);
}

export async function updateAdjustmentRule(
  params: Record<string, unknown>
): Promise<z.infer<typeof adjustmentRuleSchema>> {
  const data = await invoke('update_price_adjustment_rule', params);
  return validateApiResponse(adjustmentRuleSchema, 'update_price_adjustment_rule', data);
}

export async function toggleAdjustmentRule(id: number): Promise<void> {
  return invoke('toggle_price_adjustment_rule', { id });
}

export async function deleteAdjustmentRule(id: number): Promise<void> {
  return invoke('delete_price_adjustment_rule', { id });
}

export async function getApplicableAdjustmentRules(
  productId: string
): Promise<z.infer<typeof adjustmentRuleSchema>[]> {
  return invokeWithValidation(adjustmentRuleListSchema, 'get_applicable_price_adjustment_rules', { productId });
}

export async function getGlobalAdjustmentRules(): Promise<z.infer<typeof adjustmentRuleSchema>[]> {
  return invokeWithValidation(adjustmentRuleListSchema, 'get_global_price_adjustment_rules');
}

export type AdjustmentRule = z.infer<typeof adjustmentRuleSchema>;
export type AdjustmentConfig = z.infer<typeof adjustmentConfigSchema>;

// ============================================================================
// Statistics Schemas & API
// ============================================================================

const overviewStatsSchema = z.object({
  today_revenue: currencySchema,
  today_orders: nonNegativeIntSchema,
  today_customers: nonNegativeIntSchema,
  average_order_value: currencySchema,
  cash_revenue: currencySchema,
  card_revenue: currencySchema,
  other_revenue: currencySchema,
  voided_orders: nonNegativeIntSchema,
  voided_amount: currencySchema,
  total_discount: currencySchema,
  avg_guest_spend: currencySchema,
  avg_dining_time: z.number().optional(),
});

const revenueTrendPointSchema = z.object({
  time: z.string().min(1),
  value: currencySchema,
});

const categorySaleSchema = z.object({
  name: z.string().min(1),
  value: currencySchema,
  color: z.string().min(1),
});

const topProductSchema = z.object({
  name: z.string().min(1),
  sales: nonNegativeIntSchema,
});

const statisticsResponseSchema = z.object({
  overview: overviewStatsSchema,
  revenueTrend: z.array(revenueTrendPointSchema),
  categorySales: z.array(categorySaleSchema),
  topProducts: z.array(topProductSchema),
});

export interface StatisticsResponse {
  overview: {
    today_revenue: number;
    today_orders: number;
    today_customers: number;
    average_order_value: number;
    cash_revenue: number;
    card_revenue: number;
    other_revenue: number;
    voided_orders: number;
    voided_amount: number;
    total_discount: number;
    avg_guest_spend: number;
    avg_dining_time?: number;
  };
  revenueTrend: Array<{ time: string; value: number }>;
  categorySales: Array<{ name: string; value: number; color: string }>;
  topProducts: Array<{ name: string; sales: number }>;
}

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
// Sales Report Schemas & API
// ============================================================================

const salesReportItemSchema = z.object({
  order_id: positiveIntSchema,
  receipt_number: z.string().nullable(),
  date: z.string().min(1),
  total: currencySchema,
  status: z.string().min(1),
});

const salesReportResponseSchema = z.object({
  items: z.array(salesReportItemSchema),
  total: nonNegativeIntSchema,
  page: positiveIntSchema,
  pageSize: positiveIntSchema,
  totalPages: positiveIntSchema,
});

export interface SalesReportResponse {
  items: Array<{ order_id: number; receipt_number: string | null; date: string; total: number; status: string }>;
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
