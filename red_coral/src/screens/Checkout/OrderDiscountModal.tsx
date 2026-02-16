import React from 'react';
import { Percent } from 'lucide-react';
import { HeldOrder } from '@/core/domain/types';
import { applyOrderDiscount } from '@/core/stores/order/commands';
import { PriceAdjustmentModal } from './PriceAdjustmentModal';
import type { AdjustmentConfig } from './PriceAdjustmentModal';

const DISCOUNT_CONFIG: AdjustmentConfig = {
  type: 'discount',
  icon: Percent,
  color: 'orange',
  i18nPrefix: 'checkout.order_discount',
  getExisting: (order) => ({
    percent: order.order_manual_discount_percent,
    fixed: order.order_manual_discount_fixed,
  }),
  applyFn: async (orderId, { percent, fixed, authorizer }) => {
    await applyOrderDiscount(orderId, {
      discountPercent: percent,
      discountFixed: fixed,
      authorizer,
    });
  },
};

interface OrderDiscountModalProps {
  isOpen: boolean;
  order: HeldOrder;
  onClose: () => void;
}

export const OrderDiscountModal: React.FC<OrderDiscountModalProps> = (props) => (
  <PriceAdjustmentModal {...props} config={DISCOUNT_CONFIG} />
);
