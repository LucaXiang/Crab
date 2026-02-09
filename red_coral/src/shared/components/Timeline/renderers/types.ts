import type { OrderEvent } from '@/core/domain/types/orderEvent';
import type { LucideIcon } from 'lucide-react';

export type TranslateFn = (key: string, params?: Record<string, string | number>) => string;

export interface TimelineTag {
  text: string;
  type: 'item' | 'payment';
}

export interface DetailTag {
  label: string;
  value: string;
  colorClass: string;
}

export interface TimelineDisplayData {
  title: string;
  summary?: string;
  details: string[];
  detailTags?: DetailTag[];
  icon: LucideIcon | React.ComponentType;
  colorClass: string;
  customColor?: string;
  timestamp: number;
  isHidden?: boolean;
  tags?: TimelineTag[];
}

/**
 * Event Renderer Interface (类似 Rust trait)
 */
export interface EventRenderer<T> {
  render(event: OrderEvent, payload: T, t: TranslateFn): TimelineDisplayData;
}
