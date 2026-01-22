import { HeldOrder, Table, CartItem, Zone } from '@/core/domain/types';

export type TableFilter = 'ALL' | 'EMPTY' | 'OCCUPIED' | 'OVERTIME' | 'PRE_PAYMENT';

export interface TableSelectionScreenProps {
  heldOrders: HeldOrder[];
  onSelectTable: (
    table: Table,
    guestCount: number,
    zone?: Zone
  ) => void;
  onClose: () => void;
  onNavigateCheckout?: (tableId: string) => void;
  mode: 'HOLD' | 'RETRIEVE';
  cart?: CartItem[];
  manageTableId?: string;
}

 

 
