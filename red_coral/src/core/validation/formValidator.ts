/**
 * Form Validation Utilities
 *
 * Provides form validation using Zod schemas with React integration.
 */

import { z } from 'zod';

// ============================================================================
// Common Form Validation Schemas
// ============================================================================

/**
 * Product form validation schema
 */
export const productFormSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  receiptName: z.string().optional(),
  price: z.number().min(0, 'Price must be positive'),
  category: z.string().min(1, 'Category is required'),
  image: z.string().min(1, 'Image is required'),
  externalId: z.number().int().positive().optional(),
  taxRate: z.number().min(0).max(1),
  kitchenPrinterId: z.number().optional().nullable(),
  kitchenPrintName: z.string().optional(),
  isKitchenPrintEnabled: z.number().min(-1).max(1).optional(),
  isLabelPrintEnabled: z.number().min(-1).max(1).optional(),
});

export type ProductFormData = z.infer<typeof productFormSchema>;

/**
 * Category form validation schema
 */
export const categoryFormSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  sortOrder: z.number().int().min(0).optional(),
  kitchenPrinterId: z.number().optional().nullable(),
  isKitchenPrintEnabled: z.boolean().optional(),
  isLabelPrintEnabled: z.boolean().optional(),
});

export type CategoryFormData = z.infer<typeof categoryFormSchema>;

/**
 * Table form validation schema
 */
export const tableFormSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  zoneId: z.string().min(1, 'Zone is required'),
  capacity: z.number().int().min(1, 'Capacity must be at least 1'),
});

export type TableFormData = z.infer<typeof tableFormSchema>;

/**
 * Zone form validation schema
 */
export const zoneFormSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  surchargeType: z.enum(['percentage', 'fixed']).optional(),
  surchargeAmount: z.number().min(0).optional(),
});

export type ZoneFormData = z.infer<typeof zoneFormSchema>;

/**
 * User form validation schema
 */
export const userFormSchema = z.object({
  username: z.string().min(3, 'Username must be at least 3 characters'),
  password: z.string().min(6, 'Password must be at least 6 characters').optional(),
  roleId: z.number().int().positive('Role is required'),
  name: z.string().min(1, 'Name is required'),
  isActive: z.boolean().optional(),
});

export type UserFormData = z.infer<typeof userFormSchema>;

/**
 * Kitchen printer form validation schema
 */
export const kitchenPrinterFormSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  connectionType: z.string().min(1, 'Connection type is required'),
  connectionInfo: z.string().min(1, 'Connection info is required'),
  isDefault: z.number().min(0).max(1).optional(),
});

export type KitchenPrinterFormData = z.infer<typeof kitchenPrinterFormSchema>;

/**
 * Attribute template form validation schema
 */
export const attributeTemplateFormSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  type: z.string().optional(),
  displayOrder: z.number().int().min(0).optional(),
  showOnReceipt: z.boolean().optional(),
  receiptName: z.string().optional(),
  kitchenPrinterId: z.number().optional().nullable(),
  isGlobal: z.boolean().optional(),
});

export type AttributeTemplateFormData = z.infer<typeof attributeTemplateFormSchema>;

// ============================================================================
// Validation Helper Functions
// ============================================================================

/**
 * Validate form data and return errors
 */
export function validateForm(
  schema: z.ZodObject<z.ZodRawShape>,
  data: unknown
): { valid: boolean; errors: Record<string, string> } {
  const result = schema.safeParse(data);
  if (result.success) {
    return { valid: true, errors: {} };
  }

  const errors: Record<string, string> = {};
  for (const issue of result.error?.issues || []) {
    const path = issue.path[0] as string;
    if (path) {
      errors[path] = issue.message;
    }
  }

  return { valid: false, errors };
}

/**
 * Check if a value is valid
 */
export function isValid(
  schema: z.ZodObject<z.ZodRawShape>,
  value: unknown
): boolean {
  return schema.safeParse(value).success;
}

/**
 * Get first error message for a field
 */
export function getFieldError(
  schema: z.ZodObject<z.ZodRawShape>,
  data: unknown,
  field: string
): string | null {
  const result = schema.safeParse(data);
  if (result.success) return null;

  for (const issue of result.error?.issues || []) {
    if (issue.path[0] === field) {
      return issue.message;
    }
  }
  return null;
}
