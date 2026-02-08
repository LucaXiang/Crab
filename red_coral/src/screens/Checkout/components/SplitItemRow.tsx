import React from 'react';
import { CartItemSnapshot } from '@/core/domain/types';
import { useProductById } from '@/features/product';
import { useImageUrl } from '@/core/hooks';
import { formatCurrency } from '@/utils/currency';
import { t } from '@/infrastructure/i18n';
import DefaultImage from '@/assets/reshot.svg';
import { GroupedOptionsList } from '@/shared/components';

interface SplitItemRowProps {
  item: CartItemSnapshot;
}

export const SplitItemRow: React.FC<SplitItemRowProps> = ({ item }) => {
  // Get product to get image
  const product = useProductById(Number(item.id));
  const [imageUrl] = useImageUrl(product?.image);
  const imageSrc = imageUrl || DefaultImage;

  // For split items, always compute from unit_price × quantity
  // (item.line_total may reflect the original item's full total, not the split portion)
  const unitPrice = item.unit_price;
  const lineTotal = unitPrice * item.quantity;

  return (
    <div className="flex items-center gap-3 py-2 select-none">
      {/* Image */}
      <div className="w-12 h-12 shrink-0 rounded-lg overflow-hidden bg-gray-100 border border-gray-200">
        <img
          src={imageSrc}
          alt={item.name}
          className="w-full h-full object-cover"
          onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }}
        />
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="font-medium text-gray-700 truncate">{item.name}</div>
        {item.selected_specification?.is_multi_spec && (
          <div className="text-xs text-gray-500">{t('pos.cart.spec')}: {item.selected_specification.name}</div>
        )}
        {item.selected_options && item.selected_options.length > 0 && (
          <GroupedOptionsList options={item.selected_options} className="flex flex-col gap-0.5 mt-0.5" itemClassName="text-xs text-gray-500" />
        )}
        {item.manual_discount_percent != null && item.manual_discount_percent > 0 && (
          <span className="text-xs bg-rose-100 text-rose-600 px-1.5 py-0.5 rounded">
            -{item.manual_discount_percent}%
          </span>
        )}
      </div>

      {/* Price */}
      <div className="text-right shrink-0 tabular-nums">
        <div className="text-sm text-gray-500">
          x{item.quantity} · {formatCurrency(unitPrice)}
        </div>
        <div className="font-bold text-gray-700">{formatCurrency(lineTotal)}</div>
      </div>
    </div>
  );
};
