import { useState, useEffect } from 'react';

export interface UsePriceInputOptions {
  allowNegative?: boolean;
  minValue?: number;
  maxValue?: number;
  onCommit?: (value: number) => void;
}

export interface UsePriceInputReturn {
  priceInput: string;
  setPriceInput: (val: string) => void;
  handlePriceChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  commitPrice: () => void;
  handlePriceKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void;
}

/**
 * Custom hook for handling price input with validation and formatting
 * Supports decimal numbers with up to 2 decimal places
 *
 * @param initialValue - Initial price value
 * @param options - Configuration options
 * @returns Price input handlers and state
 */
export function usePriceInput(
  initialValue: number,
  options: UsePriceInputOptions = {}
): UsePriceInputReturn {
  const { allowNegative = false, minValue, maxValue, onCommit } = options;

  const [priceInput, setPriceInput] = useState<string>(initialValue.toFixed(2));

  // Update priceInput when initialValue changes
  useEffect(() => {
    setPriceInput(initialValue.toFixed(2));
  }, [initialValue]);

  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    let v = e.target.value;

    // Allow only digits, decimal point, and optionally minus sign
    const regex = allowNegative ? /[^\d.-]/g : /[^\d.]/g;
    v = v.replace(regex, '');

    // Ensure only one decimal point
    const parts = v.split('.');
    if (parts.length > 2) {
      v = parts[0] + '.' + parts.slice(1).join('');
    }

    // Limit decimal places to 2
    if (parts[1] && parts[1].length > 2) {
      v = parts[0] + '.' + parts[1].slice(0, 2);
    }

    // Ensure only one minus sign at the beginning
    if (allowNegative && v.includes('-')) {
      const minusCount = (v.match(/-/g) || []).length;
      if (minusCount > 1 || (v.indexOf('-') > 0)) {
        v = v.replace(/-/g, '');
        if (v.startsWith('.') || v === '') {
          v = '-' + v;
        } else {
          v = '-' + v.replace('-', '');
        }
      }
    }

    setPriceInput(v);
  };

  const commitPrice = () => {
    const val = parseFloat(priceInput);
    let finalValue = isNaN(val) ? 0 : val;

    // Apply min/max constraints
    if (minValue !== undefined && finalValue < minValue) {
      finalValue = minValue;
    }
    if (maxValue !== undefined && finalValue > maxValue) {
      finalValue = maxValue;
    }

    // Update displayed value to formatted number
    setPriceInput(finalValue.toFixed(2));

    // Trigger callback if provided
    onCommit?.(finalValue);
  };

  const handlePriceKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      commitPrice();
      (e.target as HTMLInputElement).blur();
    } else if (e.key === 'Escape') {
      // Reset to initial value
      setPriceInput(initialValue.toFixed(2));
      (e.target as HTMLInputElement).blur();
    }
  };

  return {
    priceInput,
    setPriceInput,
    handlePriceChange,
    commitPrice,
    handlePriceKeyDown
  };
}
