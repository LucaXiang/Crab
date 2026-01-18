export interface GroupedAttribute {
  attributeName: string;
  optionNames: string[];
  totalPrice: number;
}

export interface ItemAttributeSelection {
  attributeId: number;
  optionId: number;
  name: string;
  value: string;
  priceModifier?: number;
}

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
    group.totalPrice += opt.priceModifier || 0;
  });

  return groups;
};
