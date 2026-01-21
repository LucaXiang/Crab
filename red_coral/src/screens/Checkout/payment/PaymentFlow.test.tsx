import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { PaymentFlow } from './PaymentFlow';

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
  usePaymentSession: () => null,
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
  splitOrder: vi.fn(),
}));

vi.mock('@/stores/order/useReceiptStore', () => ({
  useReceiptStore: {
    getState: () => ({
      generateReceiptNumber: () => 'FAC-TEST-001'
    })
  }
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
    getState: () => ({
      setReceiptNumber: () => {}
    })
  }
}));

vi.mock('@/presentation/components/Toast', () => ({
  toast: {
    error: () => {},
    success: () => {}
  }
}));

const baseOrder: any = {
  key: 'ORDER-1',
  total: 100,
  paidAmount: 0,
  is_retail: false,
  zone_name: 'Z',
  tableName: 'T1',
  guestCount: 2,
  startTime: Date.now(),
  items: [],
  timeline: []
};

describe('PaymentFlow', () => {
  it('renders and opens cash modal when clicking cash button', async () => {
    render(<PaymentFlow order={baseOrder} onComplete={() => {}} />);

    const cashButton = screen.getByRole('button', { name: /checkout\.method\.cash/i });
    fireEvent.click(cashButton);

    const confirmButton = await screen.findByText('checkout.payment.confirm');
    expect(confirmButton).toBeInTheDocument();
  });
});
