export interface GroupedAttribute {
  attributeName: string;
  optionNames: string[];
  totalPrice: number;
}

export const groupOptionsByAttribute = (options: { attributeName: string; optionName: string; priceModifier: number }[]): GroupedAttribute[] => {
  const groups: GroupedAttribute[] = [];
  
  options.forEach(opt => {
    let group = groups.find(g => g.attributeName === opt.attributeName);
    if (!group) {
      group = {
        attributeName: opt.attributeName,
        optionNames: [],
        totalPrice: 0
      };
      groups.push(group);
    }
    group.optionNames.push(opt.optionName);
    group.totalPrice += opt.priceModifier;
  });
  
  return groups;
};
