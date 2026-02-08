export interface ItemActionPanelProps {
  t: (key: string, params?: Record<string, string | number>) => string;
  quantity: number;
  discount: number;
  basePrice: number;
  optionsModifier: number;
  onQuantityChange: (val: number) => void;
  onDiscountChange: (val: number, authorizer?: { id: number; name: string }) => void;
  onBasePriceChange?: (val: number) => void;
  onConfirm: () => void;
  onCancel?: () => void;
  onDelete?: (authorizer?: { id: number; name: string }) => void;
  confirmLabel?: string;
  showDelete?: boolean;
}

export type EditMode = 'STANDARD' | 'QTY' | 'DISC' | 'PRICE' | 'BASE_PRICE';
