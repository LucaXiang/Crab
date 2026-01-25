/**
 * API Response Validator
 *
 * Provides Zod validation wrapper for API calls.
 * All API responses should be validated before use to ensure data integrity.
 */

import { invokeApi } from '@/infrastructure/api/tauri-client';
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
 * Validates data against a schema and returns parsed data
 */
function validateData<T extends z.ZodType>(
  schema: T,
  apiName: string,
  data: unknown
): z.infer<T> {
  const result = schema.safeParse(data);
  if (!result.success) {
    const errorMsg = `Invalid response from ${apiName}: ${result.error.message}`;
    logger.error(errorMsg, result.error, { component: 'ApiValidator', apiName });
    throw new ApiValidationError(errorMsg, apiName, result.error);
  }
  return result.data;
}

/**
 * Type-safe wrapper for invokeApi with Zod validation
 */
async function invokeWithValidation<T extends z.ZodType>(
  schema: T,
  command: string,
  args?: Record<string, unknown>
): Promise<z.infer<T>> {
  const data = await invokeApi(command, args);
  return validateData(schema, command, data);
}

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
  order_id: z.string(),
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
  items: Array<{ order_id: string; receipt_number: string | null; date: string; total: number; status: string }>;
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
