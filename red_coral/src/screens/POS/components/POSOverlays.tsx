import React, { Suspense } from 'react';
import { createPortal } from 'react-dom';
import { HeldOrder, CompletedOrder } from '@/types';
import { useI18n } from '@/hooks/useI18n';
import { SectionErrorBoundary } from '@/presentation/components/SectionErrorBoundary';
import { LoadingScreen } from '@/presentation/components/LoadingScreen';

// Lazy load heavy screens
const CheckoutScreen = React.lazy(() => import('@/screens/Checkout').then(m => ({ default: m.CheckoutScreen })));
const HistoryScreen = React.lazy(() => import('@/screens/History').then(m => ({ default: m.HistoryScreen })));
const SettingsScreen = React.lazy(() => import('@/screens/Settings').then(m => ({ default: m.SettingsScreen })));
const StatisticsScreen = React.lazy(() => import('@/screens/Statistics').then(m => ({ default: m.StatisticsScreen })));

interface POSOverlaysProps {
  screen: 'POS' | 'HISTORY' | 'SETTINGS' | 'STATISTICS';
  viewMode: 'pos' | 'checkout';
  checkoutOrder: HeldOrder | null;
  completedOrders?: CompletedOrder[];
  onCheckoutCancel: () => void;
  onCheckoutComplete: () => void;
  onSetScreen: (screen: 'POS' | 'HISTORY' | 'SETTINGS' | 'STATISTICS') => void;
  onManageTable?: () => void;
}

export const POSOverlays = React.memo<POSOverlaysProps>(({
	  screen,
	  viewMode,
	  checkoutOrder,
	  onCheckoutCancel,
	  onCheckoutComplete,
	  onSetScreen,
	  onManageTable,
}) => {
	  const { t } = useI18n();

	  if (typeof document === 'undefined') {
	    return null;
	  }

	  return createPortal(
	    <>
	      {viewMode === 'checkout' && checkoutOrder && (
	        <div className="fixed inset-0 z-40 bg-gray-100 animate-in slide-in-from-right duration-300 shadow-2xl">
	          <Suspense fallback={<LoadingScreen />}>
            <SectionErrorBoundary
              region="checkout_overlay"
              title={t('checkout.error.title')}
              description={t('checkout.error.hint')}
              autoReload={true}
            >
              <CheckoutScreen
                order={checkoutOrder}
	                onCancel={onCheckoutCancel}
	                onComplete={onCheckoutComplete}
	                onManageTable={onManageTable}
	              />
	            </SectionErrorBoundary>
	          </Suspense>
	        </div>
	      )}
	
	      {screen === 'HISTORY' && (
	        <div className="fixed inset-0 z-50 bg-gray-100 shadow-2xl">
	          <Suspense fallback={<LoadingScreen />}>
	            <HistoryScreen
	              isVisible
	              onBack={() => onSetScreen('POS')}
	              onOpenStatistics={() => onSetScreen('STATISTICS')}
	            />
	          </Suspense>
	        </div>
	      )}
	
	      {screen === 'STATISTICS' && (
	        <div className="fixed inset-0 z-50 bg-gray-100 shadow-2xl">
	          <Suspense fallback={<LoadingScreen />}>
	            <StatisticsScreen
	              isVisible
	              onBack={() => onSetScreen('POS')}
	            />
	          </Suspense>
	        </div>
	      )}
	
	      {screen === 'SETTINGS' && (
	        <div className="fixed inset-0 z-50 bg-gray-100 shadow-2xl">
	          <Suspense fallback={<LoadingScreen />}>
	            <SettingsScreen onBack={() => onSetScreen('POS')} />
	          </Suspense>
	        </div>
	      )}
	    </>,
	    document.body
	  );
});

POSOverlays.displayName = 'POSOverlays';
