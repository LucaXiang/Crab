/**
 * Zod Validation Schemas
 *
 * Data validation schemas for API responses and user inputs.
 * These schemas ensure data integrity and provide runtime type checking.
 */

import { z } from 'zod';

// ============================================================================
// Common Schemas
// ============================================================================

/**
 * Currency amount schema (positive number)
 * Validates monetary values in euros
 */
export const currencySchema = z.number().positive().finite();

/**
 * Optional currency amount schema
 */
export const optionalCurrencySchema = currencySchema.optional();

/**
 * Positive integer schema
 */
export const positiveIntSchema = z.number().int().positive();

/**
 * Non-negative integer schema
 */
export const nonNegativeIntSchema = z.number().int().min(0);

/**
 * Timestamp schema (Unix timestamp in seconds or milliseconds)
 */
export const timestampSchema = z.number().int();

/**
 * UUID schema (string format)
 */
export const uuidSchema = z.string().uuid();

/**
 * ISO date string schema
 */
export const dateStringSchema = z.string().datetime();

// ============================================================================
// Category Schemas
// ============================================================================

/**
 * Category validation schema
 */
export const categorySchema = z.object({
  name: z.string().min(1),
  sortOrder: positiveIntSchema.optional(),
  kitchenPrinterId: positiveIntSchema.nullable().optional(),
  isKitchenPrintEnabled: z.boolean(),
  isLabelPrintEnabled: z.boolean(),
});

/**
 * Category response array schema
 */
export const categoryListSchema = z.array(categorySchema);

// ============================================================================
// Product Schemas
// ============================================================================

/**
 * Tag schema for product specifications
 */
export const specTagSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  color: z.string().min(1),
});

/**
 * Product specification validation schema
 */
export const productSpecificationSchema = z.object({
  id: z.string().min(1),
  productId: z.string().min(1),
  name: z.string().min(1),
  receiptName: z.string().optional(),
  price: currencySchema,
  externalId: positiveIntSchema.optional(),
  displayOrder: nonNegativeIntSchema,
  isDefault: z.boolean(),
  isRoot: z.boolean(),
  isActive: z.boolean(),
  tags: z.array(specTagSchema).optional(),
});

/**
 * Product validation schema
 */
export const productSchema = z.object({
  id: z.string().min(1),
  uuid: uuidSchema.optional(),
  name: z.string().min(1),
  receiptName: z.string().optional(),
  price: currencySchema,
  image: z.string(),
  category: z.string().min(1),
  externalId: positiveIntSchema,
  taxRate: z.number().int().min(0).max(100),
  sortOrder: positiveIntSchema.optional(),
  kitchenPrinterId: positiveIntSchema.nullable().optional(),
  kitchenPrintName: z.string().optional(),
  isKitchenPrintEnabled: z.number().int().min(-1).max(1).optional(),
  isLabelPrintEnabled: z.number().int().min(-1).max(1).optional(),
  hasMultiSpec: z.boolean().optional(),
});

/**
 * Product list response schema
 */
export const productListSchema = z.object({
  products: z.array(productSchema),
  total: nonNegativeIntSchema,
  page: positiveIntSchema.optional(),
});

/**
 * Fetch products params schema
 */
export const fetchProductsParamsSchema = z.object({
  category: z.string().optional(),
  search: z.string().optional(),
  page: positiveIntSchema.optional(),
  limit: positiveIntSchema.optional(),
}).optional();

// ============================================================================
// Order Schemas
// ============================================================================

/**
 * Order item option schema
 */
export const orderItemOptionSchema = z.object({
  attributeId: z.string().min(1),
  attributeName: z.string().min(1),
  optionId: z.string().min(1),
  optionName: z.string().min(1),
  receiptName: z.string().optional(),
  priceModifier: currencySchema.optional(),
});

/**
 * Order item schema
 */
export const orderItemSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  receiptName: z.string().optional(),
  price: currencySchema,
  quantity: positiveIntSchema,
  discountPercent: z.number().min(0).max(100).optional(),
  surcharge: z.number().min(0).optional(),
  guestId: z.string().optional(),
  originalPrice: currencySchema.optional(),
  selectedOptions: z.array(orderItemOptionSchema).optional(),
});

/**
 * Timeline event schema
 */
export const timelineEventSchema = z.object({
  id: z.string().min(1),
  type: z.string().min(1),
  timestamp: timestampSchema,
  title: z.string().min(1),
  summary: z.string().optional(),
  data: z.record(z.string(), z.any()).optional(),
  color: z.string().optional(),
});

/**
 * Discount info schema
 */
export const discountInfoSchema = z.object({
  type: z.enum(['PERCENTAGE', 'FIXED_AMOUNT']),
  value: z.number(),
  amount: currencySchema,
});

/**
 * Surcharge info schema
 */
export const surchargeInfoSchema = z.object({
  type: z.enum(['PERCENTAGE', 'FIXED_AMOUNT']),
  amount: currencySchema,
  total: currencySchema,
  value: z.number().optional(),
  name: z.string().optional(),
});

/**
 * Order schema
 */
export const orderSchema = z.object({
  orderId: positiveIntSchema,
  key: z.string().min(1),
  tableName: z.string().optional(),
  receiptNumber: z.string().optional(),
  status: z.string().min(1),
  startTime: timestampSchema,
  endTime: timestampSchema.optional(),
  guestCount: nonNegativeIntSchema,
  subtotal: currencySchema,
  total: currencySchema,
  discount: discountInfoSchema.optional(),
  surcharge: surchargeInfoSchema.optional(),
  zoneName: z.string().optional(),
  items: z.array(orderItemSchema),
  timeline: z.array(timelineEventSchema).optional(),
});

// ============================================================================
// Statistics Schemas
// ============================================================================

/**
 * Overview stats schema
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
 * Revenue trend point schema
 */
export const revenueTrendPointSchema = z.object({
  time: z.string().min(1),
  value: currencySchema,
});

/**
 * Category sale schema
 */
export const categorySaleSchema = z.object({
  name: z.string().min(1),
  value: currencySchema,
  color: z.string().min(1),
});

/**
 * Top product schema
 */
export const topProductSchema = z.object({
  name: z.string().min(1),
  sales: nonNegativeIntSchema,
});

/**
 * Sales report item schema
 */
export const salesReportItemSchema = z.object({
  orderId: positiveIntSchema,
  receiptNumber: z.string().nullable(),
  date: z.string().min(1),
  total: currencySchema,
  status: z.string().min(1),
});

/**
 * Sales report response schema
 */
export const salesReportResponseSchema = z.object({
  items: z.array(salesReportItemSchema),
  total: nonNegativeIntSchema,
  page: positiveIntSchema,
  pageSize: positiveIntSchema,
  totalPages: positiveIntSchema,
});

/**
 * Statistics response schema
 */
export const statisticsResponseSchema = z.object({
  overview: overviewStatsSchema,
  revenueTrend: z.array(revenueTrendPointSchema),
  categorySales: z.array(categorySaleSchema),
  topProducts: z.array(topProductSchema),
});

// ============================================================================
// Price Adjustment Schemas
// ============================================================================

/**
 * Price adjustment rule schema
 */
export const priceAdjustmentRuleSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  displayName: z.string().min(1),
  receiptName: z.string().min(1),
  ruleType: z.enum(['SURCHARGE', 'DISCOUNT']),
  scope: z.enum(['GLOBAL', 'CATEGORY', 'TAG', 'PRODUCT', 'ZONE']),
  targetId: z.string().optional(),
  zoneId: z.string().optional(),
  adjustmentType: z.enum(['PERCENTAGE', 'FIXED_AMOUNT']),
  adjustmentValue: z.number(),
  priority: nonNegativeIntSchema,
  isActive: z.boolean(),
  description: z.string().optional(),
  timeMode: z.enum(['ALWAYS', 'SCHEDULE', 'ONETIME']),
  startTime: timestampSchema.optional(),
  endTime: timestampSchema.optional(),
  scheduleConfigJson: z.string().optional(),
  createdAt: timestampSchema,
  updatedAt: timestampSchema,
  createdBy: z.string().optional(),
});

/**
 * Price adjustment rule list schema
 */
export const priceAdjustmentRuleListSchema = z.array(priceAdjustmentRuleSchema);

// ============================================================================
// Validation Utility Functions
// ============================================================================

/**
 * Type inference helper - extracts TypeScript types from Zod schemas
 */
export type Infer<T extends z.ZodType> = z.infer<T>;

/**
 * Validates data against a schema
 * @param schema - Zod schema to validate against
 * @param data - Data to validate
 * @returns Object with success boolean and either parsed data or error
 */
export function validateData<T extends z.ZodType>(
  schema: T,
  data: unknown
): { success: true; data: z.infer<T> } | { success: false; error: z.ZodError } {
  const result = schema.safeParse(data);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, error: result.error };
}

/**
 * Parses and validates API response
 * Throws error if validation fails
 */
export function parseApiResponse<T extends z.ZodType>(
  schema: T,
  data: unknown,
  context?: string
): z.infer<T> {
  const result = schema.safeParse(data);
  if (!result.success) {
    const errorMsg = context
      ? `Invalid ${context}: ${result.error.message}`
      : `Invalid data: ${result.error.message}`;
    throw new Error(errorMsg);
  }
  return result.data;
}

/**
 * Creates a validator function for form inputs
 */
export function createFormValidator<T extends z.ZodType>(
  schema: T
): (data: unknown) => { success: boolean; data?: z.infer<T>; errors?: Record<string, string> } {
  return (data: unknown) => {
    const result = schema.safeParse(data);
    if (result.success) {
      return { success: true, data: result.data };
    }
    const errors: Record<string, string> = {};
    for (const issue of result.error.issues) {
      const path = issue.path.join('.');
      errors[path] = issue.message;
    }
    return { success: false, errors };
  };
}

// ============================================================================
// Re-export Types for convenience
// ============================================================================

export type Category = Infer<typeof categorySchema>;
export type Product = Infer<typeof productSchema>;
export type ProductSpecification = Infer<typeof productSpecificationSchema>;
export type Order = Infer<typeof orderSchema>;
export type OrderItem = Infer<typeof orderItemSchema>;
export type TimelineEvent = Infer<typeof timelineEventSchema>;
export type OverviewStats = Infer<typeof overviewStatsSchema>;
export type RevenueTrendPoint = Infer<typeof revenueTrendPointSchema>;
export type CategorySale = Infer<typeof categorySaleSchema>;
export type TopProduct = Infer<typeof topProductSchema>;
export type PriceAdjustmentRule = Infer<typeof priceAdjustmentRuleSchema>;

// ============================================================================
// Table & Zone Schemas
// ============================================================================

/**
 * Zone validation schema
 */
export const zoneSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  surchargeType: z.enum(['percentage', 'fixed']).optional(),
  surchargeAmount: currencySchema.optional(),
});

/**
 * Zone array schema
 */
export const zoneListSchema = z.array(zoneSchema);

/**
 * Table validation schema (base)
 */
export const tableSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  zoneId: z.string().min(1),
  capacity: nonNegativeIntSchema,
});

/**
 * Table with zone name (from JOIN query)
 */
export const tableWithZoneSchema = tableSchema.extend({
  zoneName: z.string().optional(),
});

/**
 * Table response schema (from API) - status is required
 */
export const tableResponseSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  zoneId: z.string().min(1),
  capacity: nonNegativeIntSchema,
  status: z.string(),
  zoneName: z.string().optional(),
});

/**
 * Table list schema
 */
export const tableListSchema = z.array(tableWithZoneSchema);

/**
 * Kitchen printer validation schema
 */
export const kitchenPrinterSchema = z.object({
  id: positiveIntSchema,
  name: z.string().min(1),
  printerName: z.string().optional(),
  description: z.string().optional(),
  connectionType: z.string().optional(),
  connectionInfo: z.string().optional(),
  isDefault: z.number().int().optional(),
});

/**
 * Kitchen printer list schema
 */
export const kitchenPrinterListSchema = z.array(kitchenPrinterSchema);

// ============================================================================
// Attribute Schemas
// ============================================================================

/**
 * Attribute template validation schema
 */
export const attributeTemplateSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  type: z.enum(['SINGLE_REQUIRED', 'SINGLE_OPTIONAL', 'MULTI_REQUIRED', 'MULTI_OPTIONAL']),
  displayOrder: nonNegativeIntSchema,
  isActive: z.boolean(),
  showOnReceipt: z.boolean(),
  receiptName: z.string().optional(),
  kitchenPrinterId: positiveIntSchema.nullable().optional(),
  isGlobal: z.boolean().optional(),
});

/**
 * Attribute option validation schema
 */
export const attributeOptionSchema = z.object({
  id: z.string().min(1),
  attributeId: z.string().min(1),
  name: z.string().min(1),
  receiptName: z.string().optional(),
  kitchenPrintName: z.string().optional(),
  valueCode: z.string().optional(),
  priceModifier: currencySchema,
  isDefault: z.boolean(),
  displayOrder: nonNegativeIntSchema,
  isActive: z.boolean(),
});

/**
 * Product attribute binding schema
 */
export const productAttributeSchema = z.object({
  id: z.string().min(1),
  productId: z.string().min(1),
  attributeId: z.string().min(1),
  isRequired: z.boolean(),
  displayOrder: nonNegativeIntSchema,
  defaultOptionIds: z.array(z.string()).optional(),
});

/**
 * Category attribute binding schema
 */
export const categoryAttributeSchema = z.object({
  id: z.string().min(1),
  category: z.string().min(1),
  attributeId: z.string().min(1),
  isRequired: z.boolean(),
  displayOrder: nonNegativeIntSchema,
  defaultOptionIds: z.array(z.string()).optional(),
});

/**
 * Attribute option for product/category attributes
 */
const attributeOptionSimpleSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  priceModifier: currencySchema.optional(),
  isDefault: z.boolean().optional(),
  isActive: z.boolean().optional(),
});

/**
 * Product attribute info (without template metadata)
 */
const productAttributeInfoSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  type: z.enum(['SINGLE_REQUIRED', 'SINGLE_OPTIONAL', 'MULTI_REQUIRED', 'MULTI_OPTIONAL']),
  displayOrder: nonNegativeIntSchema,
  isActive: z.boolean(),
  options: z.array(attributeOptionSimpleSchema),
  defaultOptionIds: z.array(z.string()).optional(),
});

/**
 * Product with attributes response schema
 */
export const productWithAttributesSchema = z.object({
  product: z.object({
    id: z.string().min(1),
    name: z.string().min(1),
    price: currencySchema,
    categoryId: z.string().min(1),
    image: z.string().optional(),
  }),
  attributes: z.array(productAttributeInfoSchema),
  bindings: z.array(z.object({
    id: z.string().min(1),
    attributeId: z.string().min(1),
    isRequired: z.boolean(),
    displayOrder: nonNegativeIntSchema,
    defaultOptionIds: z.array(z.string()).optional(),
  })).optional(),
});

/**
 * Category attributes response schema
 */
export const categoryAttributesSchema = z.object({
  categoryId: z.string().min(1),
  attributes: z.array(productAttributeInfoSchema),
  bindings: z.array(z.object({
    id: z.string().min(1),
    attributeId: z.string().min(1),
    isRequired: z.boolean(),
    displayOrder: nonNegativeIntSchema,
    defaultOptionIds: z.array(z.string()).optional(),
  })).optional(),
});

// ============================================================================
// Auth Schemas
// ============================================================================

/**
 * Role validation schema
 */
export const roleSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  displayName: z.string().min(1),
  description: z.string().optional(),
  isSystem: z.boolean(),
  createdAt: timestampSchema,
  updatedAt: timestampSchema,
});

/**
 * User validation schema
 */
export const userSchema = z.object({
  id: z.string().min(1),
  username: z.string().min(1),
  displayName: z.string().min(1),
  role: z.string().min(1),
  avatar: z.string().optional(),
  isActive: z.boolean(),
  createdAt: timestampSchema,
  updatedAt: timestampSchema,
  createdBy: z.string().optional(),
  lastLogin: timestampSchema.optional(),
});

/**
 * User session validation schema
 */
export const userSessionSchema = z.object({
  user: userSchema,
  token: z.string().optional(),
});

// ============================================================================
// Fetch Responses
// ============================================================================

/**
 * Fetch tables response schema
 */
export const fetchTablesResponseSchema = z.object({
  tables: z.array(tableWithZoneSchema),
  total: nonNegativeIntSchema,
});

/**
 * Fetch kitchen printers response schema
 */
export const fetchKitchenPrintersResponseSchema = z.object({
  printers: z.array(kitchenPrinterSchema),
});
