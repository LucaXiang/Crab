import { create } from 'zustand';

type VirtualKeyboardMode = 'always' | 'never' | 'auto';
type KeyboardLayout = 'text' | 'number';

interface VirtualKeyboardState {
  mode: VirtualKeyboardMode;
  visible: boolean;
  activeElement: HTMLElement | null;
  layout: KeyboardLayout;

  setMode: (mode: VirtualKeyboardMode) => void;
  show: (element: HTMLElement) => void;
  hide: () => void;
  isEnabled: () => boolean;
}

function detectTouchScreen(): boolean {
  return navigator.maxTouchPoints > 0 || matchMedia('(pointer: coarse)').matches;
}

function resolveLayout(el: HTMLElement): KeyboardLayout {
  if (el instanceof HTMLInputElement) {
    const t = el.type;
    const im = el.inputMode;
    if (t === 'number' || im === 'decimal' || im === 'numeric') return 'number';
  }
  return 'text';
}

// Persist mode to localStorage manually (simple, no middleware needed since it's one field)
const STORAGE_KEY = 'virtual-keyboard-mode';
function loadMode(): VirtualKeyboardMode {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === 'always' || v === 'never' || v === 'auto') return v;
  } catch { /* ignore */ }
  return 'auto';
}

export const useVirtualKeyboardStore = create<VirtualKeyboardState>()((set, get) => ({
  mode: loadMode(),
  visible: false,
  activeElement: null,
  layout: 'text',

  setMode: (mode) => {
    try { localStorage.setItem(STORAGE_KEY, mode); } catch { /* ignore */ }
    set({ mode });
    // If switching to 'never', hide immediately
    if (mode === 'never') set({ visible: false, activeElement: null });
  },

  show: (element) => {
    if (!get().isEnabled()) return;
    set({ visible: true, activeElement: element, layout: resolveLayout(element) });
  },

  hide: () => set({ visible: false, activeElement: null }),

  isEnabled: () => {
    const { mode } = get();
    if (mode === 'always') return true;
    if (mode === 'never') return false;
    return detectTouchScreen();
  },
}));

// Selectors
export const useVirtualKeyboardVisible = () =>
  useVirtualKeyboardStore((s) => s.visible);

export const useVirtualKeyboardLayout = () =>
  useVirtualKeyboardStore((s) => s.layout);

export const useVirtualKeyboardMode = () =>
  useVirtualKeyboardStore((s) => s.mode);
