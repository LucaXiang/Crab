import React from 'react';
import { CartItem } from '@/core/domain/types';
import { useLongPress } from '@/hooks/useLongPress';
import { formatCurrency } from '@/utils/currency';
import { calculateOptionsModifier } from '@/utils/pricing';
import { t } from '@/infrastructure/i18n';
import { GroupedOptionsList } from '@/shared/components';
import { Gift } from 'lucide-react';

interface UnpaidItemRowProps {
  item: CartItem;
  onEditItem: (item: CartItem) => void;
  externalId?: number | null;
  bgColor?: string;
  hoverColor?: string;
  accentColor?: string;
}

const UnpaidItemRowInner: React.FC<UnpaidItemRowProps> = ({
  item,
  onEditItem,
  externalId,
  bgColor,
  hoverColor,
  accentColor,
}) => {
  const unpaidQty = item.unpaid_quantity;

  // Price calculations
  const optionsModifier = calculateOptionsModifier(item.selected_options);
  const basePrice = (item.original_price ?? item.price) + optionsModifier;
  const unitPrice = item.unit_price ?? item.price;
  const discountPercent = item.manual_discount_percent || 0;
  const hasDiscount = discountPercent > 0 || basePrice !== unitPrice;

  const isComped = !!item.is_comped;
  const hasMultiSpec = item.selected_specification?.is_multi_spec;
  const hasOptions = item.selected_options && item.selected_options.length > 0;
  const hasNote = item.note && item.note.trim().length > 0;
  const activeRules = (item.applied_rules ?? []).filter(r => !r.skipped);
  const totalRuleDiscount = activeRules
    .filter(r => r.rule_type === 'DISCOUNT')
    .reduce((sum, r) => sum + r.calculated_amount, 0);
  const totalRuleSurcharge = activeRules
    .filter(r => r.rule_type === 'SURCHARGE')
    .reduce((sum, r) => sum + r.calculated_amount, 0);

  const clickHandlers = useLongPress(
    () => {},
    () => {
      if (!isComped) {
        onEditItem(item);
      }
    },
    { delay: 500, isPreventDefault: false }
  );

  return (
    <div
      {...clickHandlers}
      className={`
        group relative border rounded-xl p-4 transition-all duration-200 select-none cursor-pointer
        ${isComped
          ? 'bg-emerald-50/50 border-emerald-200'
          : 'hover:shadow-md'
        }
        ${!isComped && hoverColor ? 'hover:[background-color:var(--hover-bg)]' : ''}
      `}
      style={!isComped ? {
        backgroundColor: bgColor || '#ffffff',
        borderColor: hoverColor || '#e5e7eb',
        '--hover-bg': hoverColor,
      } as React.CSSProperties : undefined}
    >
      <div className="flex items-start justify-between gap-4">
        {/* Left: Item Info */}
        <div className="flex-1 min-w-0">
          {/* Line 1: Product Name */}
          <div className="font-bold text-gray-900 text-lg truncate">
            {item.name}
          </div>

          {/* Line 2: Specification (if multi-spec) */}
          {hasMultiSpec && (
            <div className="text-sm text-gray-600 mt-0.5">
              {t('pos.cart.spec')}: {item.selected_specification!.name}
            </div>
          )}

          {/* Line 3: Attribute Tags (grouped by attribute, one per line) */}
          {hasOptions && <GroupedOptionsList options={item.selected_options!} />}

          {/* Line 4: Note */}
          {hasNote && (
            <div className="text-xs text-blue-600 mt-1 flex items-center gap-1">
              <span>📝</span>
              <span className="truncate">{item.note}</span>
            </div>
          )}

          {/* Line 5: Quantity × Unit Price */}
          <div className="flex items-center gap-2 mt-2 text-sm text-gray-500 tabular-nums">
            <span className="font-medium">x{unpaidQty}</span>
            <span className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: accentColor || '#d1d5db' }} />
            {isComped ? (
              <>
                <span className="line-through text-gray-400">{formatCurrency(item.original_price ?? basePrice)}</span>
                <span className="font-semibold text-emerald-600">{formatCurrency(0)}</span>
              </>
            ) : hasDiscount ? (
              <>
                <span className="line-through text-gray-400">{formatCurrency(basePrice)}</span>
                <span className="font-semibold text-gray-700">{formatCurrency(unitPrice)}</span>
              </>
            ) : (
              <span className="font-semibold text-gray-700">{formatCurrency(unitPrice)}</span>
            )}
          </div>
        </div>

        {/* Right: Total + Badges */}
        <div className="flex flex-col items-end justify-between shrink-0 self-stretch">
          {/* Line Total + Badges */}
          <div className="flex items-center gap-2">
            {isComped && (
              <span className="text-xs bg-emerald-100 text-emerald-700 px-1.5 py-0.5 rounded flex items-center gap-1">
                <Gift size={10} />
                {t('checkout.comp.badge')}
              </span>
            )}
            {discountPercent > 0 && (
              <span className="text-xs bg-orange-100 text-orange-700 px-1.5 py-0.5 rounded font-medium">
                -{discountPercent}%
              </span>
            )}
            {totalRuleDiscount > 0 && (
              <span className="text-xs bg-amber-100 text-amber-700 px-1.5 py-0.5 rounded font-medium">
                -{formatCurrency(totalRuleDiscount)}
              </span>
            )}
            {totalRuleSurcharge > 0 && (
              <span className="text-xs bg-purple-100 text-purple-700 px-1.5 py-0.5 rounded font-medium">
                +{formatCurrency(totalRuleSurcharge)}
              </span>
            )}
            <div className="font-bold text-xl tabular-nums text-gray-900">
              {formatCurrency(unitPrice * unpaidQty)}
            </div>
          </div>

          {/* Instance ID + External ID */}
          <div className="flex items-center gap-2">
            {item.instance_id && (
              <span className="px-1.5 py-0.5 rounded text-[0.625rem] font-bold font-mono border bg-blue-100 text-blue-600 border-blue-200">
                #{item.instance_id.slice(-5)}
              </span>
            )}
            {externalId != null && (
              <div className="text-xs text-white bg-gray-900/85 font-bold font-mono px-2 py-0.5 rounded">
                {externalId}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export const UnpaidItemRow = React.memo(UnpaidItemRowInner);
