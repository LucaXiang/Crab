import { Currency } from './currency';

/**
 * Calculates the remaining amount to be paid
 */
export const calculateRemaining = (total: number, paid: number): number => {
  return Currency.max(0, Currency.sub(total, paid)).toNumber();
};

/**
 * Checks if the order is paid in full
 */
export const isPaidInFull = (total: number, paid: number): boolean => {
  return Currency.gte(paid, total);
};
