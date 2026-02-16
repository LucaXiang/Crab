import React from 'react';
import { TrendingUp } from 'lucide-react';
import { HeldOrder } from '@/core/domain/types';
import { applyOrderSurcharge } from '@/core/stores/order/commands';
import { PriceAdjustmentModal } from './PriceAdjustmentModal';
import type { AdjustmentConfig } from './PriceAdjustmentModal';

const SURCHARGE_CONFIG: AdjustmentConfig = {
  type: 'surcharge',
  icon: TrendingUp,
  color: 'purple',
  i18nPrefix: 'checkout.order_surcharge',
  getExisting: (order) => ({
    percent: order.order_manual_surcharge_percent,
    fixed: order.order_manual_surcharge_fixed,
  }),
  applyFn: async (orderId, { percent, fixed, authorizer }) => {
    await applyOrderSurcharge(orderId, {
      surchargePercent: percent,
      surchargeAmount: fixed,
      authorizer,
    });
  },
};

interface OrderSurchargeModalProps {
  isOpen: boolean;
  order: HeldOrder;
  onClose: () => void;
}

export const OrderSurchargeModal: React.FC<OrderSurchargeModalProps> = (props) => (
  <PriceAdjustmentModal {...props} config={SURCHARGE_CONFIG} />
);
