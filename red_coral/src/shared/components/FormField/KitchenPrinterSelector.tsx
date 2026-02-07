import React from 'react';
import { FormField } from './FormField';
import { selectClass } from './FormField';
import { usePrintDestinationStore } from '@/core/stores/resources';

export interface KitchenPrinterSelectorProps {
  value: number | null | undefined;
  onChange: (value: number | null) => void;
  label?: string;
  t: (key: string) => string;
}

/**
 * Reusable print destination selector component
 * Automatically fetches and displays available print destinations
 */
export const KitchenPrinterSelector: React.FC<KitchenPrinterSelectorProps> = ({
  value,
  onChange,
  label,
  t
}) => {
  const items = usePrintDestinationStore((state) => state.items);

  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const val = e.target.value;
    onChange(val ? Number(val) : null);
  };

  return (
    <FormField label={label || t('settings.kitchen_printer')}>
      <div className="relative">
        <select
          value={value || ''}
          onChange={handleChange}
          className={selectClass}
        >
          <option value="">{t('common.label.default')}</option>
          {items.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
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
