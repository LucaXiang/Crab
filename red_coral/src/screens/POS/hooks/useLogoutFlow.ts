import { useCallback, useState } from 'react';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { voidOrder } from '@/core/stores/order/commands';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useShiftStore } from '@/core/stores/shift';
import { useI18n } from '@/hooks/useI18n';

interface ExitDialog {
  open: boolean;
  title: string;
  description: string;
  isBlocking: boolean;
}

export function useLogoutFlow() {
  const { t } = useI18n();
  const logout = useAuthStore((state) => state.logout);
  const { currentShift, clearShift } = useShiftStore();

  const [exitDialog, setExitDialog] = useState<ExitDialog>({ open: false, title: '', description: '', isBlocking: false });
  const [showCloseShiftModal, setShowCloseShiftModal] = useState(false);

  const handleLogout = useCallback(() => {
    clearShift();
    logout();
  }, [logout, clearShift]);

  const handleRequestExit = useCallback(async () => {
    const store = useActiveOrdersStore.getState();
    const active = store.getActiveOrders();
    const retailActive = active.filter((o) => o.is_retail === true);

    // Void retail orders
    for (const snapshot of retailActive) {
      try {
        await voidOrder(snapshot.order_id, { voidType: 'CANCELLED', note: 'Retail session cancelled on logout' });
      } catch {
        // Ignore errors - best effort cleanup
      }
    }

    // Check for remaining non-retail orders
    const remaining = store
      .getActiveOrders()
      .filter((o) => o.is_retail !== true);

    if (remaining && remaining.length > 0) {
      const names = remaining.map((o) => o.table_name || o.order_id).slice(0, 5).join('、');
      const moreText = remaining.length > 5 ? ` ${t('app.logout.and_more', { count: remaining.length })}` : '';
      setExitDialog({
        open: true,
        title: t('app.logout.blocked'),
        description:
          (t('app.logout.description')) + `\n${names}${moreText}\n\n` +
          (t('app.logout.hint')),
        isBlocking: true,
      });
    } else {
      if (currentShift) {
        setShowCloseShiftModal(true);
      } else {
        handleLogout();
      }
    }
  }, [t, handleLogout, currentShift]);

  const handleCloseShiftSuccess = useCallback(() => {
    setShowCloseShiftModal(false);
    handleLogout();
  }, [handleLogout]);

  const handleDismissExitDialog = useCallback(() => {
    setExitDialog((d) => ({ ...d, open: false }));
  }, []);

  const handleConfirmExitDialog = useCallback(() => {
    setExitDialog((d) => ({ ...d, open: false }));
    if (!exitDialog.isBlocking) {
      handleLogout();
    }
  }, [exitDialog.isBlocking, handleLogout]);

  return {
    exitDialog,
    showCloseShiftModal,
    currentShift,
    handleRequestExit,
    handleCloseShiftSuccess,
    handleDismissExitDialog,
    handleConfirmExitDialog,
    setShowCloseShiftModal,
  };
}
