/**
 * UI Stores
 */

// UI Store (路由、模态框、动画、POS 过滤)
export { useScreen } from './useUIStore';
export { useViewMode } from './useUIStore';
export { useAnimations } from './useUIStore';
export { useModalStates } from './useUIStore';
export { useUIActions } from './useUIStore';
export { useSelectedCategory } from './useUIStore';
export { usePOSUIActions } from './useUIStore';

// Printer Store (打印机配置)
export {
  useReceiptPrinter,
  useKitchenPrinter,
  useLabelPrinter,
  useCashDrawerPrinter,
  useActiveLabelTemplateId,
  usePrinterActions,
  useAutoOpenCashDrawerAfterReceipt,
} from '../printer';

// UI Scale Store
export { useUIScale, useSetUIScale, initUIScale } from './useUIScaleStore';

// Virtual Keyboard Store
export {
  useVirtualKeyboardStore,
  useVirtualKeyboardMode,
} from './useVirtualKeyboardStore';
