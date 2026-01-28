import type { ItemOption } from '@/core/domain/types';

export interface GroupedAttribute {
  attributeName: string;
  optionNames: string[];
  totalPrice: number;
}


export const groupOptionsByAttribute = (options: ItemOption[]): GroupedAttribute[] => {
  const groups: GroupedAttribute[] = [];

  options.forEach(opt => {
    let group = groups.find(g => g.attributeName === opt.attribute_name);
    if (!group) {
      group = {
        attributeName: opt.attribute_name,
        optionNames: [],
        totalPrice: 0
      };
      groups.push(group);
    }
    group.optionNames.push(opt.option_name);
    group.totalPrice += opt.price_modifier || 0;
  });

  return groups;
};
