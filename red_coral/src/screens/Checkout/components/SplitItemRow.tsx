import React from 'react';
import { CartItemSnapshot } from '@/core/domain/types';
import { useProductById } from '@/features/product';
import { useImageUrl } from '@/core/hooks';
import { formatCurrency } from '@/utils/currency';
import { t } from '@/infrastructure/i18n';
import DefaultImage from '@/assets/reshot.svg';
import { ImageOff } from 'lucide-react';

interface SplitItemRowProps {
  item: CartItemSnapshot;
}

export const SplitItemRow: React.FC<SplitItemRowProps> = ({ item }) => {
  // Get product to get image
  const product = useProductById(item.id);
  const [imageUrl] = useImageUrl(product?.image);
  const imageSrc = imageUrl || DefaultImage;

  // Server-authoritative: use backend-computed values
  const unitPrice = item.unit_price ?? item.price;
  const lineTotal = item.line_total ?? (unitPrice * item.quantity);

  return (
    <div className="flex items-center gap-3 py-2 select-none">
      {/* Image */}
      <div className="w-12 h-12 shrink-0 rounded-lg overflow-hidden bg-gray-100 border border-gray-200">
        {product?.image ? (
          <img
            src={imageSrc}
            alt={item.name}
            className="w-full h-full object-cover"
            onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }}
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-gray-300">
            <ImageOff size={20} />
          </div>
        )}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="font-medium text-gray-700 truncate">{item.name}</div>
        {item.selected_specification?.is_multi_spec && (
          <div className="text-xs text-gray-500">{t('pos.cart.spec')}: {item.selected_specification.name}</div>
        )}
        {item.selected_options && item.selected_options.length > 0 && (() => {
          const grouped = new Map<string, typeof item.selected_options>();
          for (const opt of item.selected_options!) {
            const key = opt.attribute_name;
            if (!grouped.has(key)) grouped.set(key, []);
            grouped.get(key)!.push(opt);
          }
          return (
            <div className="flex flex-col gap-0.5 mt-0.5">
              {[...grouped.entries()].map(([attrName, opts]) => (
                <span key={attrName} className="text-xs text-gray-500">
                  {attrName}: {opts!.map((opt, i) => (
                    <React.Fragment key={i}>
                      {i > 0 && ', '}
                      {opt.option_name}
                      {opt.price_modifier != null && opt.price_modifier !== 0 && (
                        <span className={opt.price_modifier > 0 ? 'text-orange-600 ml-0.5' : 'text-green-600 ml-0.5'}>
                          {opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)}
                        </span>
                      )}
                    </React.Fragment>
                  ))}
                </span>
              ))}
            </div>
          );
        })()}
        {item.manual_discount_percent != null && item.manual_discount_percent > 0 && (
          <span className="text-xs bg-rose-100 text-rose-600 px-1.5 py-0.5 rounded">
            -{item.manual_discount_percent}%
          </span>
        )}
      </div>

      {/* Price */}
      <div className="text-right shrink-0 tabular-nums">
        <div className="text-sm text-gray-500">
          x{item.quantity} @ {formatCurrency(unitPrice)}
        </div>
        <div className="font-bold text-gray-700">{formatCurrency(lineTotal)}</div>
      </div>
    </div>
  );
};
