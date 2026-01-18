import { ItemAttributeSelection } from '@/core/domain/types';

export interface GroupedAttribute {
  attributeName: string;
  optionNames: string[];
  totalPrice: number;
}

// Re-export for backward compatibility
export type { ItemAttributeSelection };

export const groupOptionsByAttribute = (options: ItemAttributeSelection[]): GroupedAttribute[] => {
  const groups: GroupedAttribute[] = [];

  options.forEach(opt => {
    let group = groups.find(g => g.attributeName === opt.name);
    if (!group) {
      group = {
        attributeName: opt.name,
        optionNames: [],
        totalPrice: 0
      };
      groups.push(group);
    }
    group.optionNames.push(opt.value);
    group.totalPrice += opt.price_modifier || 0;
  });

  return groups;
};
