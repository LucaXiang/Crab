import { create } from 'zustand';

const UI_SCALE_KEY = 'ui-scale';
const DEFAULT_SCALE = 1;
const MIN_SCALE = 0.9;
const MAX_SCALE = 1.3;

// 应用缩放到 CSS 变量
const applyScale = (scale: number): void => {
  document.documentElement.style.setProperty('--ui-scale', scale.toString());
};

// 从 localStorage 读取初始值
const getInitialScale = (): number => {
  if (typeof window === 'undefined' || typeof localStorage?.getItem !== 'function') {
    return DEFAULT_SCALE;
  }
  const stored = localStorage.getItem(UI_SCALE_KEY);
  if (stored) {
    const parsed = parseFloat(stored);
    if (!isNaN(parsed) && parsed >= MIN_SCALE && parsed <= MAX_SCALE) {
      return parsed;
    }
  }
  return DEFAULT_SCALE;
};

interface UIScaleState {
  scale: number; // 0.9 ~ 1.3
  setScale: (scale: number) => void;
}

export const useUIScaleStore = create<UIScaleState>((set) => ({
  scale: getInitialScale(),

  setScale: (scale: number) => {
    // 限制范围
    const clampedScale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, scale));

    // 持久化到 localStorage
    if (typeof window !== 'undefined') {
      localStorage.setItem(UI_SCALE_KEY, clampedScale.toString());
    }

    // 应用到 CSS 变量
    applyScale(clampedScale);

    set({ scale: clampedScale });
  },
}));

// 初始化函数 - 在应用启动时调用
export const initUIScale = (): void => {
  const scale = getInitialScale();
  applyScale(scale);
};

// Selector hooks
export const useUIScale = () => useUIScaleStore((state) => state.scale);
export const useSetUIScale = () => useUIScaleStore((state) => state.setScale);
