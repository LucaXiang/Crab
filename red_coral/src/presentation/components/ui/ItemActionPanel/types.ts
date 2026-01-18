export interface ItemActionPanelProps {
  t: (key: string, params?: any) => string;
  quantity: number;
  discount: number;
  basePrice: number;
  optionsModifier: number;
  onQuantityChange: (val: number) => void;
  onDiscountChange: (val: number, authorizer?: { id: string; username: string }) => void;
  onConfirm: () => void;
  onCancel?: () => void;
  onDelete?: (authorizer?: { id: string; username: string }) => void;
  confirmLabel?: string;
  showDelete?: boolean;
}

export type EditMode = 'STANDARD' | 'QTY' | 'DISC' | 'PRICE';
