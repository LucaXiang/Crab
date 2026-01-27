import Decimal from 'decimal.js';

// Configure Decimal to handle adequate precision
// Unified rounding strategy: ROUND_HALF_UP (四舍五入)
// Matches backend rust_decimal::RoundingStrategy::MidpointAwayFromZero
Decimal.set({ precision: 20, rounding: Decimal.ROUND_HALF_UP });

export const Currency = {
  /**
   * Convert value to Decimal
   */
  toDecimal(v: number | string | Decimal): Decimal {
    return new Decimal(v);
  },

  /**
   * Add: a + b
   */
  add(a: number | string | Decimal, b: number | string | Decimal): Decimal {
    return new Decimal(a).plus(b);
  },

  /**
   * Subtract: a - b
   */
  sub(a: number | string | Decimal, b: number | string | Decimal): Decimal {
    return new Decimal(a).minus(b);
  },

  /**
   * Multiply: a * b
   */
  mul(a: number | string | Decimal, b: number | string | Decimal): Decimal {
    return new Decimal(a).times(b);
  },

  /**
   * Divide: a / b
   */
  div(a: number | string | Decimal, b: number | string | Decimal): Decimal {
    return new Decimal(a).div(b);
  },

  /**
   * Round to 2 decimal places using ROUND_HALF_UP (四舍五入)
   * e.g. 1.235 -> 1.24, 1.234 -> 1.23
   * @deprecated Use round2() for clarity. Kept for backward compatibility.
   */
  floor2(n: number | string | Decimal): Decimal {
    return new Decimal(n).toDecimalPlaces(2, Decimal.ROUND_HALF_UP);
  },

  /**
   * Standard rounding to 2 decimal places
   * Used for final totals if needed
   */
  round2(n: number | string | Decimal): Decimal {
    return new Decimal(n).toDecimalPlaces(2, Decimal.ROUND_HALF_UP);
  },

  /**
   * Check if a > b
   */
  gt(a: number | string | Decimal, b: number | string | Decimal): boolean {
    return new Decimal(a).greaterThan(b);
  },

  /**
   * Check if a >= b
   */
  gte(a: number | string | Decimal, b: number | string | Decimal): boolean {
    return new Decimal(a).greaterThanOrEqualTo(b);
  },

  /**
   * Check if a < b
   */
  lt(a: number | string | Decimal, b: number | string | Decimal): boolean {
    return new Decimal(a).lessThan(b);
  },

  /**
   * Check if a <= b
   */
  lte(a: number | string | Decimal, b: number | string | Decimal): boolean {
    return new Decimal(a).lessThanOrEqualTo(b);
  },

  /**
   * Check if a == b
   */
  eq(a: number | string | Decimal, b: number | string | Decimal): boolean {
    return new Decimal(a).equals(b);
  },

  /**
   * Return max(a, b)
   */
  max(a: number | string | Decimal, b: number | string | Decimal): Decimal {
    return Decimal.max(a, b);
  },

  /**
   * Return min(a, b)
   */
  min(a: number | string | Decimal, b: number | string | Decimal): Decimal {
    return Decimal.min(a, b);
  },

  /**
   * Absolute value
   */
  abs(n: number | string | Decimal): Decimal {
    return new Decimal(n).abs();
  }
};
