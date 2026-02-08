import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { PaymentFlow } from './PaymentFlow';
import type { HeldOrder } from '@/core/domain/types';

vi.mock('@/hooks/useI18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
    locale: 'en',
    setLocale: () => {}
  })
}));

vi.mock('@/stores/order/useCheckoutStore', () => ({
  useRetailServiceType: () => 'dine_in',
  useCheckoutStore: () => () => {}
}));

vi.mock('@/stores/usePaymentStore', () => ({
  usePaymentActions: () => ({
    initSession: vi.fn(),
    addPayment: vi.fn()
  }),
  usePaymentSession: (): null => null,
  usePaymentTotals: () => ({ totalPaid: 0 })
}));

vi.mock('@/core/stores/order/useActiveOrdersStore', () => ({
  useActiveOrdersStore: {
    getState: () => ({
      orders: {},
      getOrder: vi.fn(),
      getOrderByTable: vi.fn(),
    })
  }
}));

vi.mock('@/core/stores/order/useOrderOperations', () => ({
  completeOrder: vi.fn(),
  voidOrder: vi.fn(),
  partialSettle: vi.fn(),
  splitByItems: vi.fn(),
  splitByAmount: vi.fn(),
  startAaSplit: vi.fn(),
  payAaSplit: vi.fn(),
}));

vi.mock('@/presentation/components/auth/EscalatableGate', () => ({
  EscalatableGate: ({ children }: { children: React.ReactNode }) => <>{children}</>
}));

vi.mock('@/presentation/components/OrderSidebar', () => ({
  OrderSidebar: () => <div data-testid="ordersidebar-placeholder" />
}));

vi.mock('@/services/paymentService', () => ({
  processCashPayment: vi.fn(async ({ amount }: { amount: number }) => ({
    method: 'CASH',
    amount,
    tip: 0
  })),
  processCardPayment: vi.fn(async ({ amount }: { amount: number }) => ({
    method: 'CARD',
    amount,
    tip: 0
  })),
  validatePaymentAmount: (paid: number, total: number) => ({
    isValid: paid >= total
  }),
  printOrderReceipt: vi.fn()
}));

vi.mock('@/stores/useCartStore', () => ({
  useCartStore: {
    getState: () => ({})
  }
}));

vi.mock('@/presentation/components/Toast', () => ({
  toast: {
    error: () => {},
    success: () => {}
  }
}));

const baseOrder = {
  order_id: 'ORDER-1',
  total: 100,
  paid_amount: 0,
  remaining_amount: 100,
  is_retail: false,
  zone_name: 'Z',
  zone_id: 1,
  table_name: 'T1',
  table_id: 1,
  guest_count: 2,
  status: 'OPEN',
  items: [],
  payments: [],
  original_total: 100,
  subtotal: 100,
  total_discount: 0,
  total_surcharge: 0,
  tax: 0,
  discount: 0,
  comp_total_amount: 0,
  order_manual_discount_amount: 0,
  order_manual_surcharge_amount: 0,
} as unknown as HeldOrder;

describe('PaymentFlow', () => {
  it('renders and opens cash modal when clicking cash button', async () => {
    render(<PaymentFlow order={baseOrder} onComplete={() => {}} />);

    const cashButton = screen.getByRole('button', { name: /checkout\.method\.cash/i });
    fireEvent.click(cashButton);

    const confirmButton = await screen.findByText('checkout.payment.confirm');
    expect(confirmButton).toBeInTheDocument();
  });
});
