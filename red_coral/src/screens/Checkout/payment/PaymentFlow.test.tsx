import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { PaymentFlow } from './PaymentFlow';
import type { HeldOrder } from '@/core/domain/types';

// --- Core infrastructure mocks (cut off tauri-client import tree) ---

vi.mock('@/infrastructure/api/tauri-client', () => {
  const proxy = new Proxy({}, { get: () => vi.fn() });
  return {
    invokeApi: vi.fn(),
    getApi: vi.fn(() => proxy),
    createTauriClient: vi.fn(() => proxy),
    TauriApiClient: vi.fn(() => proxy),
  };
});

vi.mock('@/hooks/useI18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
    locale: 'en',
    setLocale: () => {}
  })
}));

vi.mock('@/utils/logger', () => ({
  logger: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), debug: vi.fn() },
}));

vi.mock('@/infrastructure/i18n', () => ({
  t: (key: string) => key,
  i18n: { t: (key: string) => key, getLocale: () => 'zh-CN', setLocale: vi.fn(), subscribe: vi.fn(() => () => {}), getAllTranslations: () => ({}) },
  getLocale: () => 'zh-CN',
  setLocale: vi.fn(),
  initLocale: vi.fn(),
  DEFAULT_LOCALE: 'zh-CN',
  SUPPORTED_LOCALES: ['zh-CN', 'es-ES'],
}));

// --- Store mocks ---

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
  redeemStamp: vi.fn(),
  cancelStampRedemption: vi.fn(),
}));

vi.mock('@/core/stores/order/commands/sendCommand', () => ({
  CommandFailedError: class CommandFailedError extends Error {
    constructor(msg: string) { super(msg); this.name = 'CommandFailedError'; }
  },
  sendCommand: vi.fn(),
}));

vi.mock('@/core/services/order/paymentService', () => ({
  openCashDrawer: vi.fn(),
}));

// --- Feature mocks ---

vi.mock('@/features/product', () => ({
  useProductStore: (selector: (s: { items: never[] }) => unknown) => selector({ items: [] }),
}));

vi.mock('@/features/category', () => ({
  useCategoryStore: (selector: (s: { items: never[] }) => unknown) => selector({ items: [] }),
}));

vi.mock('@/features/member/mutations', () => ({
  getMemberDetail: vi.fn(),
  listMembers: vi.fn(),
  searchMembers: vi.fn(),
}));

// --- Heavy sub-component mocks (prevent deep import trees) ---

vi.mock('@/presentation/components/auth/EscalatableGate', () => ({
  EscalatableGate: ({ children }: { children: React.ReactNode }) => <>{children}</>
}));

vi.mock('@/presentation/components/OrderSidebar', () => ({
  OrderSidebar: () => <div data-testid="ordersidebar-placeholder" />
}));

vi.mock('@/presentation/components/Toast', () => ({
  toast: { error: vi.fn(), success: vi.fn() }
}));

vi.mock('../OrderDiscountModal', () => ({
  OrderDiscountModal: () => null,
}));

vi.mock('../OrderSurchargeModal', () => ({
  OrderSurchargeModal: () => null,
}));

vi.mock('../MemberLinkModal', () => ({
  MemberLinkModal: () => null,
}));

vi.mock('./StampRewardPickerModal', () => ({
  StampRewardPickerModal: () => null,
}));

vi.mock('./StampRedeemModal', () => ({
  StampRedeemModal: () => null,
}));

vi.mock('./PaymentSuccessModal', () => ({
  PaymentSuccessModal: () => null,
}));

vi.mock('./PaymentRecordsPage', () => ({
  PaymentRecordsPage: () => null,
}));

vi.mock('../CompItemMode', () => ({
  CompItemMode: () => null,
}));

vi.mock('../OrderDetailMode', () => ({
  OrderDetailMode: () => null,
}));

vi.mock('../MemberDetailMode', () => ({
  MemberDetailMode: () => null,
}));

vi.mock('./ItemSplitPage', () => ({
  ItemSplitPage: () => null,
}));

vi.mock('./AmountSplitPage', () => ({
  AmountSplitPage: () => null,
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
