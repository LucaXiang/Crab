
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CashPaymentModal } from './CashPaymentModal';

vi.mock('@/hooks/useI18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
    locale: 'en',
    setLocale: () => {}
  })
}));

vi.mock('@/presentation/components/ui/Numpad', () => ({
  Numpad: ({ onNumber }: { onNumber: (value: string) => void }) => (
    <button onClick={() => onNumber('1')}>numpad-1</button>
  )
}));

describe('CashPaymentModal', () => {
  it('calls onConfirm when confirm button clicked with sufficient amount', () => {
    const handleConfirm = vi.fn();

    render(
      <CashPaymentModal
        amountDue={10}
        isOpen={true}
        isProcessing={false}
        onConfirm={handleConfirm}
        onCancel={() => {}}
      />
    );

    const confirmButton = screen.getByText('checkout.confirmPayment');
    fireEvent.click(confirmButton);

    expect(handleConfirm).toHaveBeenCalledTimes(1);
    expect(handleConfirm).toHaveBeenCalledWith(10);
  });
});

