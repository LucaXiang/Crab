/**
 * UI Stores
 */

// UI Store (路由、模态框、动画、POS 过滤)
export { useUIStore } from './useUIStore';
export { useScreen } from './useUIStore';
export { useViewMode } from './useUIStore';
export { useAnimations } from './useUIStore';
export { useModalStates } from './useUIStore';
export { useUIActions } from './useUIStore';
export { useSelectedCategory } from './useUIStore';
export { useSearchQuery } from './useUIStore';
export { usePOSUIActions } from './useUIStore';

// Printer Store (打印机配置)
export {
  usePrinterStore,
  useReceiptPrinter,
  useKitchenPrinter,
  useLabelPrinter,
  useCashDrawerPrinter,
  useActiveLabelTemplateId,
  usePrinterActions,
  useAutoOpenCashDrawerAfterReceipt,
} from '../printer';

// UI Scale Store
export { useUIScaleStore, useUIScale, useSetUIScale, initUIScale } from './useUIScaleStore';
