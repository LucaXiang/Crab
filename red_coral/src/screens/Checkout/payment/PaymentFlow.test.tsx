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

vi.mock('@/core/stores/order/useCheckoutStore', () => ({
  useRetailServiceType: () => 'dineIn',
  setRetailServiceType: () => {},
  toBackendServiceType: (v: string) => v,
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

vi.mock('@/core/stores/order/commands', () => ({
  completeOrder: vi.fn(),
  voidOrder: vi.fn(),
  partialSettle: vi.fn(),
  splitByItems: vi.fn(),
  splitByAmount: vi.fn(),
  startAaSplit: vi.fn(),
  payAaSplit: vi.fn(),
  cancelPayment: vi.fn(),
  updateOrderInfo: vi.fn(),
}));

vi.mock('@/core/services/order/paymentService', () => ({
  openCashDrawer: vi.fn(),
}));

vi.mock('@/presentation/components/auth/EscalatableGate', () => ({
  EscalatableGate: ({ children }: { children: React.ReactNode }) => <>{children}</>
}));

vi.mock('@/presentation/components/OrderSidebar', () => ({
  OrderSidebar: () => <div data-testid="ordersidebar-placeholder" />
}));

vi.mock('@/presentation/components/Toast', () => ({
  toast: {
    error: () => {},
    success: () => {}
  }
}));

vi.mock('@/features/product', () => ({
  useProductStore: (selector: (s: { items: never[] }) => unknown) => selector({ items: [] }),
}));

vi.mock('@/features/category', () => ({
  useCategoryStore: (selector: (s: { items: never[] }) => unknown) => selector({ items: [] }),
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
