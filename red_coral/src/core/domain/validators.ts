/**
 * Zod Validation Schemas
 *
 * Base validation schemas used by apiValidator.
 * Only contains schemas that are actually used.
 */

import { z } from 'zod';

/**
 * Currency amount schema (positive number)
 * Validates monetary values
 */
export const currencySchema = z.number().positive().finite();

/**
 * Positive integer schema
 */
export const positiveIntSchema = z.number().int().positive();

/**
 * Non-negative integer schema
 */
export const nonNegativeIntSchema = z.number().int().min(0);
