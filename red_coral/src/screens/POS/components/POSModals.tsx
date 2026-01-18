import React, { Suspense } from 'react';
import { DraftListModal } from '@/presentation/components/DraftListModal';
import { TableSelectionScreen } from '@/screens/TableSelection';
import { HeldOrder, DraftOrder, CartItem } from '@/types';

interface POSModalsProps {
  // Draft Modal
  showDraftModal: boolean;
  draftOrders: DraftOrder[];
  onCloseDraftModal: () => void;
  onRestoreDraft: (id: string) => void;
  onDeleteDraft: (id: string) => void;

  // Table Selection Modal
  showTableScreen: boolean;
  heldOrders: HeldOrder[];
  cart: CartItem[];
  onSelectTable: (table: any, ...args: any[]) => void;
  onCloseTableScreen: () => void;
  manageTableId?: string | null;
  onNavigateCheckout: (tableId: string) => void;
}

export const POSModals = React.memo<POSModalsProps>(({
	  showDraftModal,
	  draftOrders,
	  onCloseDraftModal,
	  onRestoreDraft,
	  onDeleteDraft,
	  showTableScreen,
	  heldOrders,
	  cart,
	  onSelectTable,
	  onCloseTableScreen,
	  manageTableId,
	  onNavigateCheckout,
}) => {
	  return (
	    <>
	      {showDraftModal && (
	        <DraftListModal
	          draftOrders={draftOrders}
	          onClose={onCloseDraftModal}
	          onRestore={onRestoreDraft}
	          onDelete={onDeleteDraft}
	        />
	      )}
	
	      {showTableScreen && (
	        <Suspense fallback={null}>
	          <TableSelectionScreen
	            heldOrders={heldOrders}
	            onSelectTable={onSelectTable}
	            onClose={onCloseTableScreen}
	            onNavigateCheckout={onNavigateCheckout}
	            mode={cart.length > 0 ? 'HOLD' : 'RETRIEVE'}
	            cart={cart}
	            manageTableId={manageTableId || undefined}
	          />
	        </Suspense>
	      )}
	    </>
	  );
});

POSModals.displayName = 'POSModals';
