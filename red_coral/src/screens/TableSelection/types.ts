import { HeldOrder, Table, CartItem, Zone } from '../../types';

export type TableFilter = 'ALL' | 'EMPTY' | 'OCCUPIED' | 'OVERTIME' | 'PRE_PAYMENT';

export interface TableSelectionScreenProps {
  heldOrders: HeldOrder[];
  onSelectTable: (
    table: Table,
    guestCount: number,
    enableIndividualMode?: boolean,
    zone?: Zone
  ) => void;
  onClose: () => void;
  onNavigateCheckout?: (tableId: string) => void;
  mode: 'HOLD' | 'RETRIEVE';
  cart?: CartItem[];
  manageTableId?: string;
}

 

 
