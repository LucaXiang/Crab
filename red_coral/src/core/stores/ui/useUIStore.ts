import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { AnimationItem } from '@/presentation/components/CartAnimationOverlay';

type ScreenMode = 'POS' | 'HISTORY' | 'SETTINGS' | 'STATISTICS';
type ViewMode = 'pos' | 'checkout';

interface UIStore {
  // State
  screen: ScreenMode;
  viewMode: ViewMode;
  showDebugMenu: boolean;
  showTableScreen: boolean;
  showDraftModal: boolean;
  receiptPrinter: string | null;
  kitchenPrinter: string | null;
  isKitchenPrintEnabled: boolean;
  labelPrinter: string | null;
  isLabelPrintEnabled: boolean;
  activeLabelTemplateId: string | null;
  animations: AnimationItem[];

  // POS UI State
  selectedCategory: string; // 'all' for all categories
  searchQuery: string;

  // Actions
  setScreen: (screen: ScreenMode) => void;
  setViewMode: (mode: ViewMode) => void;
  setShowDebugMenu: (show: boolean) => void;
  setShowTableScreen: (show: boolean) => void;
  setShowDraftModal: (show: boolean) => void;
  setReceiptPrinter: (name: string | null) => void;
  setKitchenPrinter: (name: string | null) => void;
  setIsKitchenPrintEnabled: (enabled: boolean) => void;
  setLabelPrinter: (name: string | null) => void;
  setIsLabelPrintEnabled: (enabled: boolean) => void;
  setActiveLabelTemplateId: (id: string | null) => void;
  addAnimation: (animation: AnimationItem) => void;
  removeAnimation: (id: string) => void;

  // POS UI Actions
  setSelectedCategory: (category: string) => void;
  setSearchQuery: (query: string) => void;
}

export const useUIStore = create<UIStore>((set) => ({
  // Initial State
  screen: 'POS',
  viewMode: 'pos',
  showDebugMenu: false,
  showTableScreen: false,
  showDraftModal: false,
  receiptPrinter: (typeof window !== 'undefined' && typeof localStorage?.getItem === 'function' ? (localStorage.getItem('printer_receipt') || localStorage.getItem('printerName')) : null) || null,
  kitchenPrinter: (typeof window !== 'undefined' && typeof localStorage?.getItem === 'function' ? localStorage.getItem('printer_kitchen') : null) || null,
  isKitchenPrintEnabled: (typeof window !== 'undefined' && typeof localStorage?.getItem === 'function' ? localStorage.getItem('kitchen_print_enabled') !== 'false' : true),
  labelPrinter: (typeof window !== 'undefined' && typeof localStorage?.getItem === 'function' ? localStorage.getItem('printer_label') : null) || null,
  isLabelPrintEnabled: (typeof window !== 'undefined' && typeof localStorage?.getItem === 'function' ? localStorage.getItem('label_print_enabled') !== 'false' : true),
  activeLabelTemplateId: (typeof window !== 'undefined' && typeof localStorage?.getItem === 'function' ? localStorage.getItem('active_label_template_id') : null) || null,
  animations: [],

  // POS UI State
  selectedCategory: 'all',
  searchQuery: '',

  // Actions
  setScreen: (screen: ScreenMode) => set({ screen }),
  setViewMode: (mode: ViewMode) => set({ viewMode: mode }),
  setShowDebugMenu: (show: boolean) => set({ showDebugMenu: show }),
  setShowTableScreen: (show: boolean) => set({ showTableScreen: show }),
  setShowDraftModal: (show: boolean) => set({ showDraftModal: show }),
  
  setReceiptPrinter: (name: string | null) => {
    if (typeof window !== 'undefined') {
      if (name) {
        localStorage.setItem('printer_receipt', name);
        localStorage.setItem('printerName', name); // Keep legacy sync for safety
      } else {
        localStorage.removeItem('printer_receipt');
        localStorage.removeItem('printerName');
      }
    }
    set({ receiptPrinter: name });
  },

  setKitchenPrinter: (name: string | null) => {
    if (typeof window !== 'undefined') {
      if (name) localStorage.setItem('printer_kitchen', name);
      else localStorage.removeItem('printer_kitchen');
    }
    set({ kitchenPrinter: name });
  },

  setIsKitchenPrintEnabled: (enabled: boolean) => {
    if (typeof window !== 'undefined') {
      localStorage.setItem('kitchen_print_enabled', String(enabled));
    }
    set({ isKitchenPrintEnabled: enabled });
  },

  setLabelPrinter: (name: string | null) => {
    if (typeof window !== 'undefined') {
      if (name) localStorage.setItem('printer_label', name);
      else localStorage.removeItem('printer_label');
    }
    set({ labelPrinter: name });
  },

  setIsLabelPrintEnabled: (enabled: boolean) => {
    if (typeof window !== 'undefined') {
      localStorage.setItem('label_print_enabled', String(enabled));
    }
    set({ isLabelPrintEnabled: enabled });
  },

  setActiveLabelTemplateId: (id: string | null) => {
    if (typeof window !== 'undefined') {
      if (id) localStorage.setItem('active_label_template_id', id);
      else localStorage.removeItem('active_label_template_id');
    }
    set({ activeLabelTemplateId: id });
  },

  addAnimation: (animation: AnimationItem) =>
    set((state) => {
        // Prevent adding too many concurrent animations
        if (state.animations.length > 5) return state;
        return {
            animations: [...state.animations, animation]
        };
    }),

  removeAnimation: (id: string) =>
    set((state) => ({
      animations: state.animations.filter(a => a.id !== id)
    })),

  // POS UI Actions
  setSelectedCategory: (category: string) => set({ selectedCategory: category }),
  setSearchQuery: (query: string) => set({ searchQuery: query }),
}));

// ============ Granular Selectors (Performance Optimization) ============

export const useScreen = () => useUIStore((state) => state.screen);
export const useViewMode = () => useUIStore((state) => state.viewMode);
export const useSelectedPrinter = () => useUIStore((state) => state.receiptPrinter);
export const useReceiptPrinter = () => useUIStore((state) => state.receiptPrinter);
export const useKitchenPrinter = () => useUIStore((state) => state.kitchenPrinter);
export const useIsKitchenPrintEnabled = () => useUIStore((state) => state.isKitchenPrintEnabled);
export const useLabelPrinter = () => useUIStore((state) => state.labelPrinter);
export const useIsLabelPrintEnabled = () => useUIStore((state) => state.isLabelPrintEnabled);
export const useActiveLabelTemplateId = () => useUIStore((state) => state.activeLabelTemplateId);
export const useAnimations = () => useUIStore((state) => state.animations);

// POS UI Selectors
export const useSelectedCategory = () => useUIStore((state) => state.selectedCategory);
export const useSearchQuery = () => useUIStore((state) => state.searchQuery);
export const usePOSUIActions = () => useUIStore(
  useShallow((state) => ({
    setSelectedCategory: state.setSelectedCategory,
    setSearchQuery: state.setSearchQuery,
  }))
);

export const useModalStates = () => useUIStore(
  useShallow((state) => ({
    showDebugMenu: state.showDebugMenu,
    showTableScreen: state.showTableScreen,
    showDraftModal: state.showDraftModal
  }))
);

export const useUIActions = () => useUIStore(
  useShallow((state) => ({
    setScreen: state.setScreen,
    setViewMode: state.setViewMode,
    setShowDebugMenu: state.setShowDebugMenu,
    setShowTableScreen: state.setShowTableScreen,
    setShowDraftModal: state.setShowDraftModal,
    setSelectedPrinter: state.setReceiptPrinter, // Deprecated alias
    setReceiptPrinter: state.setReceiptPrinter,
    setKitchenPrinter: state.setKitchenPrinter,
    setIsKitchenPrintEnabled: state.setIsKitchenPrintEnabled,
    setLabelPrinter: state.setLabelPrinter,
    setIsLabelPrintEnabled: state.setIsLabelPrintEnabled,
    setActiveLabelTemplateId: state.setActiveLabelTemplateId,
    addAnimation: state.addAnimation,
    removeAnimation: state.removeAnimation
  }))
);
