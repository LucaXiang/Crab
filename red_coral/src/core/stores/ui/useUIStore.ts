/**
 * UI Store - 全局 UI 状态管理
 *
 * 职责：路由、模态框、动画、POS 过滤
 * 打印机配置已移至 stores/printer/usePrinterStore.ts
 */

import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { AnimationItem } from '@/presentation/components/CartAnimationOverlay';

type ScreenMode = 'POS' | 'HISTORY' | 'SETTINGS' | 'STATISTICS';
type ViewMode = 'pos' | 'checkout';

interface UIStore {
  // 路由状态
  screen: ScreenMode;
  viewMode: ViewMode;

  // 模态框状态
  showDebugMenu: boolean;
  showTableScreen: boolean;
  showDraftModal: boolean;

  // 动画队列
  animations: AnimationItem[];

  // POS 过滤状态
  selectedCategory: string;
  searchQuery: string;

  // 路由 Actions
  setScreen: (screen: ScreenMode) => void;
  setViewMode: (mode: ViewMode) => void;

  // 模态框 Actions
  setShowDebugMenu: (show: boolean) => void;
  setShowTableScreen: (show: boolean) => void;
  setShowDraftModal: (show: boolean) => void;

  // 动画 Actions
  addAnimation: (animation: AnimationItem) => void;
  removeAnimation: (id: string) => void;

  // POS Actions
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
  animations: [],
  selectedCategory: 'all',
  searchQuery: '',

  // 路由 Actions
  setScreen: (screen) => set({ screen }),
  setViewMode: (mode) => set({ viewMode: mode }),

  // 模态框 Actions
  setShowDebugMenu: (show) => set({ showDebugMenu: show }),
  setShowTableScreen: (show) => set({ showTableScreen: show }),
  setShowDraftModal: (show) => set({ showDraftModal: show }),

  // 动画 Actions
  addAnimation: (animation) =>
    set((state) => {
      if (state.animations.length > 5) return state;
      return { animations: [...state.animations, animation] };
    }),

  removeAnimation: (id) =>
    set((state) => ({
      animations: state.animations.filter(a => a.id !== id)
    })),

  // POS Actions
  setSelectedCategory: (category) => set({ selectedCategory: category }),
  setSearchQuery: (query) => set({ searchQuery: query }),
}));

// ============ Selectors ============

export const useScreen = () => useUIStore((state) => state.screen);
export const useViewMode = () => useUIStore((state) => state.viewMode);
export const useAnimations = () => useUIStore((state) => state.animations);

// POS Selectors
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
    addAnimation: state.addAnimation,
    removeAnimation: state.removeAnimation
  }))
);
