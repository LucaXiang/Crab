import React from 'react';
import { FormField } from './FormField';
import { selectClass } from './FormField';
import { useKitchenPrinterStore } from '@/core/stores/product/useKitchenPrinterStore';

export interface KitchenPrinterSelectorProps {
  value: string | number | null | undefined;
  onChange: (value: string | null) => void;
  label?: string;
  t: (key: string) => string;
}

/**
 * Reusable kitchen printer selector component
 * Automatically fetches and displays available kitchen printers
 */
export const KitchenPrinterSelector: React.FC<KitchenPrinterSelectorProps> = ({
  value,
  onChange,
  label,
  t
}) => {
  const { kitchenPrinters } = useKitchenPrinterStore();

  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const val = e.target.value;
    onChange(val || null);
  };

  return (
    <FormField label={label || t('settings.kitchenPrinter')}>
      <div className="relative">
        <select
          value={value || ''}
          onChange={handleChange}
          className={selectClass}
        >
          <option value="">{t('common.default')}</option>
          {kitchenPrinters.map((p) => (
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
