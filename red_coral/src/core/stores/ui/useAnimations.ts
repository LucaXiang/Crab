/**
 * Animation Store for cart item animations
 */

import { create } from 'zustand';

interface AnimationItem {
  id: string;
  type: string;
  data?: unknown;
  image?: string;
  startRect?: DOMRect;
  targetX?: number;
  targetY?: number;
  timestamp: number;
}

interface AnimationState {
  animationQueue: AnimationItem[];
  addAnimation: (item: Omit<AnimationItem, 'id' | 'timestamp'>) => void;
  removeAnimation: (id: string) => void;
  clearAnimation: () => void;
}

export const useAnimations = create<AnimationState>((set) => ({
  animationQueue: [],

  addAnimation: (item) => {
    const id = `anim-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    set((state) => ({
      animationQueue: [...state.animationQueue, { ...item, id, timestamp: Date.now() }],
    }));
  },

  removeAnimation: (id) => {
    set((state) => ({
      animationQueue: state.animationQueue.filter((item) => item.id !== id),
    }));
  },

  clearAnimation: () => {
    set({ animationQueue: [] });
  },
}));
