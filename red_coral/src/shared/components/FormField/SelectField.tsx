import React from 'react';
import { FormField, selectClass } from './FormField';

interface SelectOption {
  value: string | number;
  label: string;
}

export interface SelectFieldProps {
  label: string;
  value: string | number | undefined | null;
  onChange: (value: string | number) => void;
  options: SelectOption[];
  required?: boolean;
  placeholder?: string;
  className?: string;
  disabled?: boolean;
}

/**
 * Reusable select field component with dropdown icon
 * Wraps FormField and provides consistent styling across forms
 */
export const SelectField: React.FC<SelectFieldProps> = ({
  label,
  value,
  onChange,
  options,
  required,
  placeholder,
  className,
  disabled
}) => {
  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const val = e.target.value;
    // Try to parse as number if original value was number
    const numVal = Number(val);
    onChange(isNaN(numVal) || val === '' ? val : numVal);
  };

  return (
    <FormField label={label} required={required}>
      <div className="relative">
        <select
          value={value ?? ''}
          onChange={handleChange}
          className={className || selectClass}
          disabled={disabled}
        >
          {placeholder && (
            <option value="">
              {placeholder}
            </option>
          )}
          {options.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-3">
          <svg
            className="h-4 w-4 text-gray-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </div>
    </FormField>
  );
};
