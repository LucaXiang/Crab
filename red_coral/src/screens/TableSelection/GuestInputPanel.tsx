import React from 'react';
import { Table, CartItem } from '../../types';

// Sub-components
import {
  GuestInputHeader,
  GuestCountInput,
  TimelinePreview,
  ConfirmButton,
} from './components';

interface GuestInputPanelProps {
  selectedTable: Table;
  isOccupied: boolean;
  guestInput: string;
  enableIndividualMode: boolean;
  cart: CartItem[];
  onGuestInputChange: (value: string) => void;
  onIndividualModeToggle: () => void;
  onConfirm: () => void;
  onBack: () => void;
  onManage?: () => void;
}

export const GuestInputPanel: React.FC<GuestInputPanelProps> = React.memo(
  ({
    selectedTable,
    isOccupied,
    guestInput,
    cart,
    onGuestInputChange,
    onConfirm,
    onBack,
    onManage,
  }) => {
    return (
      <div className="flex-1 flex flex-col md:flex-row overflow-hidden relative">
        {/* LEFT: Input / Preview */}
        <div className="flex-1 flex flex-col min-h-0 bg-white relative">
          <GuestInputHeader
            selectedTable={selectedTable}
            isOccupied={isOccupied}
            onBack={onBack}
            onManage={onManage}
          />

          <div className="flex-1 overflow-y-auto p-0 flex flex-col items-center justify-center">
            {isOccupied ? (
              <div className="w-full h-full">
                <TimelinePreview
                  isOccupied={isOccupied}
                  cart={cart}
                  guestInput={guestInput}
                />
              </div>
            ) : (
              <GuestCountInput
                guestInput={guestInput}
                onGuestInputChange={onGuestInputChange}
              />
            )}
          </div>

          <ConfirmButton
            isOccupied={isOccupied}
            guestInput={guestInput}
            onConfirm={onConfirm}
          />
        </div>
      </div>
    );
  }
);
