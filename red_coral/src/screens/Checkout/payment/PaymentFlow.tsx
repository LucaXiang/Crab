import React, { useState, useEffect } from 'react';
import { HeldOrder } from '@/core/domain/types';
import { CompItemMode } from '../CompItemMode';
import { OrderDetailMode } from '../OrderDetailMode';
import { SelectModePage } from './SelectModePage';
import { ItemSplitPage } from './ItemSplitPage';
import { AmountSplitPage } from './AmountSplitPage';
import { PaymentRecordsPage } from './PaymentRecordsPage';

interface PaymentFlowProps {
  order: HeldOrder;
  onComplete: () => void;
  onCancel?: () => void;
  onVoid?: () => void;
  onManageTable?: () => void;
}

type PaymentMode = 'SELECT' | 'ITEM_SPLIT' | 'AMOUNT_SPLIT' | 'PAYMENT_RECORDS' | 'COMP' | 'ORDER_DETAIL';

export const PaymentFlow: React.FC<PaymentFlowProps> = ({ order, onComplete, onCancel, onVoid, onManageTable }) => {
  const [mode, setMode] = useState<PaymentMode>('SELECT');

  // Reset to SELECT when order changes
  useEffect(() => {
    setMode('SELECT');
  }, [order.order_id]);

  const totalPaid = order.paid_amount;
  const remaining = order.remaining_amount;

  switch (mode) {
    case 'SELECT':
      return (
        <SelectModePage
          order={order}
          onComplete={onComplete}
          onCancel={onCancel}
          onVoid={onVoid}
          onManageTable={onManageTable}
          onNavigate={setMode}
        />
      );
    case 'ITEM_SPLIT':
      return (
        <ItemSplitPage
          order={order}
          onBack={() => setMode('SELECT')}
          onComplete={onComplete}
          onManageTable={onManageTable}
        />
      );
    case 'AMOUNT_SPLIT':
      return (
        <AmountSplitPage
          order={order}
          onBack={() => setMode('SELECT')}
          onComplete={onComplete}
          onManageTable={onManageTable}
        />
      );
    case 'PAYMENT_RECORDS':
      return (
        <PaymentRecordsPage
          order={order}
          onBack={() => setMode('SELECT')}
          onManageTable={onManageTable}
        />
      );
    case 'COMP':
      return (
        <CompItemMode
          order={order}
          totalPaid={totalPaid}
          remaining={remaining}
          onBack={() => setMode('SELECT')}
          onManageTable={onManageTable}
        />
      );
    case 'ORDER_DETAIL':
      return (
        <OrderDetailMode
          order={order}
          totalPaid={totalPaid}
          remaining={remaining}
          onBack={() => setMode('SELECT')}
          onManageTable={onManageTable}
        />
      );
  }
};
