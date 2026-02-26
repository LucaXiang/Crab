import React, { useEffect } from 'react';
import { Printer } from 'lucide-react';
import { FormField } from './FormField';
import { usePrintDestinationStore } from '@/core/stores/resources';
import type { PrintPurpose } from '@/core/domain/types/api';

export interface KitchenPrinterSelectorProps {
  value: number[];
  onChange: (value: number[]) => void;
  /** Filter by purpose (default: show all) */
  purpose?: PrintPurpose;
  label?: string;
  t: (key: string) => string;
}

/**
 * Multi-select print destination selector (toggle chips)
 */
export const KitchenPrinterSelector: React.FC<KitchenPrinterSelectorProps> = ({
  value,
  onChange,
  purpose,
  label,
  t
}) => {
  const allItems = usePrintDestinationStore((state) => state.items);
  const isLoaded = usePrintDestinationStore((state) => state.isLoaded);
  const fetchAll = usePrintDestinationStore((state) => state.fetchAll);

  useEffect(() => {
    if (!isLoaded) fetchAll();
  }, [isLoaded, fetchAll]);
  const items = purpose ? allItems.filter((p) => p.purpose === purpose) : allItems;

  const handleToggle = (id: number) => {
    if (value.includes(id)) {
      onChange(value.filter((v) => v !== id));
    } else {
      onChange([...value, id]);
    }
  };

  if (!isLoaded) {
    return (
      <FormField label={label || t('settings.kitchen_printer')}>
        <p className="text-sm text-gray-400 py-2">...</p>
      </FormField>
    );
  }

  if (items.length === 0) {
    return (
      <FormField label={label || t('settings.kitchen_printer')}>
        <p className="text-sm text-gray-400 py-2">{t('settings.product.print.no_printers')}</p>
      </FormField>
    );
  }

  return (
    <FormField label={label || t('settings.kitchen_printer')}>
      <div className="flex flex-wrap gap-2">
        {items.map((p) => {
          const selected = value.includes(p.id);
          return (
            <button
              key={p.id}
              type="button"
              onClick={() => handleToggle(p.id)}
              className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all border ${
                selected
                  ? 'bg-teal-50 border-teal-300 text-teal-700'
                  : 'bg-white border-gray-200 text-gray-500 hover:border-gray-300 hover:bg-gray-50'
              }`}
            >
              <Printer size={14} className={selected ? 'text-teal-500' : 'text-gray-400'} />
              {p.name}
            </button>
          );
        })}
      </div>
    </FormField>
  );
};
