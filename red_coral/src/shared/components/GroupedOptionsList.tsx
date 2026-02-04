import React from 'react';
import { formatCurrency } from '@/utils/currency/formatCurrency';

interface SelectedOption {
  attribute_name: string;
  option_name: string;
  price_modifier?: number | null;
  quantity?: number;
}

interface GroupedOptionsListProps {
  options: SelectedOption[];
  className?: string;
  itemClassName?: string;
}

/**
 * 按属性名分组显示选中的属性选项
 * 用于购物车、结账、拆单等多处菜品选项展示
 */
export const GroupedOptionsList: React.FC<GroupedOptionsListProps> = ({
  options,
  className = 'flex flex-col gap-0.5 mt-1',
  itemClassName = 'text-xs text-gray-600',
}) => {
  const grouped = new Map<string, SelectedOption[]>();
  for (const opt of options) {
    const key = opt.attribute_name;
    if (!grouped.has(key)) grouped.set(key, []);
    grouped.get(key)!.push(opt);
  }

  return (
    <div className={className}>
      {[...grouped.entries()].map(([attrName, opts]) => (
        <div key={attrName} className={`${itemClassName} truncate`}>
          {attrName}: {opts.map((opt, i) => {
            const qty = opt.quantity ?? 1;
            const totalPrice = (opt.price_modifier ?? 0) * qty;
            return (
              <React.Fragment key={i}>
                {i > 0 && ', '}
                {opt.option_name}
                {qty > 1 && (
                  <span className="text-orange-600 ml-0.5">×{qty}</span>
                )}
                {totalPrice !== 0 && (
                  <span className={totalPrice > 0 ? 'text-orange-600 ml-0.5' : 'text-green-600 ml-0.5'}>
                    {totalPrice > 0 ? '+' : ''}{formatCurrency(totalPrice)}
                  </span>
                )}
              </React.Fragment>
            );
          })}
        </div>
      ))}
    </div>
  );
};
