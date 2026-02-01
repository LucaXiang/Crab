import React from 'react';
import { formatCurrency } from '@/utils/currency/formatCurrency';

interface SelectedOption {
  attribute_name: string;
  option_name: string;
  price_modifier?: number | null;
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
          {attrName}: {opts.map((opt, i) => (
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
        </div>
      ))}
    </div>
  );
};
