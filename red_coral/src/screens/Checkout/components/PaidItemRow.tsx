import React from 'react';
import { CartItem } from '@/core/domain/types';
import { formatCurrency } from '@/utils/currency';

interface PaidItemRowProps {
  item: CartItem;
  paidQty: number;
  surchargeExempt?: boolean;
  t: (key: string) => string;
}

export const PaidItemRow: React.FC<PaidItemRowProps> = ({
  item,
  paidQty,
  surchargeExempt,
  t,
}) => {
  // Price calculations
  const optionsModifier = (item.selected_options ?? []).reduce((sum, opt) => sum + (opt.price_modifier ?? 0), 0);
  const basePrice = (item.original_price ?? item.price) + optionsModifier;
  const unitPrice = surchargeExempt
    ? (item.unit_price ?? item.price) - (item.surcharge ?? 0)
    : (item.unit_price ?? item.price);
  const discountPercent = item.manual_discount_percent || 0;
  const hasDiscount = discountPercent > 0 || basePrice !== unitPrice;

  const hasMultiSpec = item.selected_specification?.is_multi_spec;
  const hasOptions = item.selected_options && item.selected_options.length > 0;
  const hasNote = item.note && item.note.trim().length > 0;

  return (
    <div className="group relative bg-gray-50/80 border border-gray-200 rounded-xl p-4">
      <div className="flex items-start justify-between gap-4">
        {/* Left: Item Info */}
        <div className="flex-1 min-w-0">
          {/* Line 1: Product Name */}
          <div className="font-bold text-gray-700 text-lg truncate">
            {item.name}
          </div>

          {/* Line 2: Specification (if multi-spec) */}
          {hasMultiSpec && (
            <div className="text-sm text-gray-500 mt-0.5">
              {item.selected_specification!.name}
            </div>
          )}

          {/* Line 3: Attribute Tags */}
          {hasOptions && (
            <div className="flex flex-wrap gap-1 mt-1">
              {item.selected_options!.map((opt, idx) => (
                <span
                  key={idx}
                  className="text-xs bg-gray-200 text-gray-500 px-1.5 py-0.5 rounded"
                >
                  {opt.attribute_name}:{opt.option_name}
                  {opt.price_modifier != null && opt.price_modifier !== 0 && (
                    <span className={opt.price_modifier > 0 ? 'text-orange-500 ml-0.5' : 'text-green-500 ml-0.5'}>
                      {opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)}
                    </span>
                  )}
                </span>
              ))}
            </div>
          )}

          {/* Line 4: Note */}
          {hasNote && (
            <div className="text-xs text-gray-500 mt-1 flex items-center gap-1">
              <span>📝</span>
              <span className="truncate">{item.note}</span>
            </div>
          )}

          {/* Line 5: Paid Quantity × Unit Price */}
          <div className="flex items-center gap-2 mt-2 text-sm text-gray-500">
            <span className="font-medium text-green-600">Paid x{paidQty}</span>
            <span className="w-1 h-1 bg-gray-300 rounded-full" />
            {hasDiscount ? (
              <>
                <span className="line-through text-gray-400">{formatCurrency(basePrice)}</span>
                <span className="font-semibold text-gray-600">{formatCurrency(unitPrice)}</span>
              </>
            ) : (
              <span className="font-semibold text-gray-600">{formatCurrency(unitPrice)}</span>
            )}
          </div>
        </div>

        {/* Right: Total + External ID */}
        <div className="flex flex-col items-end gap-2 shrink-0">
          {/* Badges + Line Total */}
          <div className="flex items-center gap-2">
            {discountPercent > 0 && (
              <span className="text-xs bg-gray-200 text-gray-600 px-1.5 py-0.5 rounded">
                -{discountPercent}%
              </span>
            )}
            <div className="font-bold text-xl text-gray-700">
              {formatCurrency(unitPrice * paidQty)}
            </div>
          </div>

          {/* External ID */}
          {item.selected_specification?.external_id && (
            <div className="text-xs text-white bg-gray-900/85 font-bold font-mono px-2 py-0.5 rounded">
              {item.selected_specification.external_id}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
