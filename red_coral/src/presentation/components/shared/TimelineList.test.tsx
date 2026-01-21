/**
 * TimelineList - Integration Tests
 *
 * 测试 Timeline 组件与 Renderer 的集成，专注于架构而非具体的翻译输出
 */

import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TimelineList } from './TimelineList';
import type { OrderEvent } from '@/core/domain/types/orderEvent';

// Mock translation
vi.mock('@/hooks/useI18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (params) {
        return `${key}:${JSON.stringify(params)}`;
      }
      return key;
    },
  }),
}));

describe('TimelineList Integration Tests', () => {
  describe('Basic Rendering', () => {
    it('should render without errors when given empty array', () => {
      expect(() => render(<TimelineList events={[]} />)).not.toThrow();
    });

    it('should render timeline items for valid events', () => {
      const events: OrderEvent[] = [
        {
          event_id: 'evt-1',
          event_type: 'TABLE_OPENED',
          timestamp: Date.now(),
          payload: {
            table_id: 'T1',
            table_name: 'Table 1',
            zone_id: 'Z1',
            zone_name: 'Main',
            guest_count: 2,
            is_retail: false,
          },
          sequence: 1,
        },
      ];

      const { container } = render(<TimelineList events={events} />);
      // Should render timeline structure with icon, timestamp, and title
      expect(container.querySelector('.rounded-full')).toBeTruthy(); // Icon container
      expect(container.querySelector('.font-mono')).toBeTruthy(); // Timestamp
      expect(container.querySelector('.font-bold')).toBeTruthy(); // Title
    });

    it('should render multiple events', () => {
      const events: OrderEvent[] = [
        {
          event_id: 'evt-1',
          event_type: 'TABLE_OPENED',
          timestamp: Date.now() - 2000,
          payload: {
            table_id: 'T1',
            table_name: 'Table 1',
            zone_id: 'Z1',
            zone_name: 'Main',
            guest_count: 2,
            is_retail: false,
          },
          sequence: 1,
        },
        {
          event_id: 'evt-2',
          event_type: 'PAYMENT_ADDED',
          timestamp: Date.now() - 1000,
          payload: {
            payment_id: 'pay-1',
            method: 'CASH',
            amount: 1000,
            tendered: null,
            change: null,
            note: null,
          },
          sequence: 2,
        },
      ];

      const { container } = render(<TimelineList events={events} />);
      // Should have multiple timeline items (check for timestamp elements)
      const timeElements = container.querySelectorAll('.font-mono');
      expect(timeElements.length).toBe(2);
      // Should have multiple icon containers
      const iconElements = container.querySelectorAll('.rounded-full');
      expect(iconElements.length).toBe(2);
    });
  });

  describe('Event Type Support', () => {
    it('should render items added event', () => {
      const events: OrderEvent[] = [
        {
          event_id: 'evt-1',
          event_type: 'ITEMS_ADDED',
          timestamp: Date.now(),
          payload: {
            items: [
              {
                id: 'item-1',
                name: 'Test Pizza',
                price: 1000,
                original_price: 1000,
                quantity: 2,
                unpaid_quantity: 2,
                instance_id: 'inst-1',
                selected_options: [],
              },
            ],
          },
          sequence: 1,
        },
      ];

      const { container } = render(<TimelineList events={events} />);
      expect(container.textContent).toContain('Test Pizza');
    });

    it('should render payment added event', () => {
      const events: OrderEvent[] = [
        {
          event_id: 'evt-1',
          event_type: 'PAYMENT_ADDED',
          timestamp: Date.now(),
          payload: {
            payment_id: 'pay-1',
            method: 'CASH',
            amount: 5000,
            tendered: null,
            change: null,
            note: 'Test payment note',
          },
          sequence: 1,
        },
      ];

      const { container } = render(<TimelineList events={events} />);
      expect(container.textContent).toContain('Test payment note');
    });

    it('should render completion event', () => {
      const events: OrderEvent[] = [
        {
          event_id: 'evt-1',
          event_type: 'ORDER_COMPLETED',
          timestamp: Date.now(),
          payload: {
            final_total: 10000,
            receipt_number: 'FAC-2026-001',
          },
          sequence: 1,
        },
      ];

      const { container } = render(<TimelineList events={events} />);
      expect(container.textContent).toContain('FAC-2026-001');
    });

    it('should render void event', () => {
      const events: OrderEvent[] = [
        {
          event_id: 'evt-1',
          event_type: 'ORDER_VOIDED',
          timestamp: Date.now(),
          payload: {
            reason: 'Test cancellation',
          },
          sequence: 1,
        },
      ];

      const { container } = render(<TimelineList events={events} />);
      expect(container.textContent).toContain('Test cancellation');
    });
  });

  describe('Error Handling', () => {
    it('should handle main event types without crashing', () => {
      const mainEventTypes: OrderEvent[] = [
        {
          event_id: 'evt-1',
          event_type: 'TABLE_OPENED',
          timestamp: Date.now(),
          payload: {
            table_id: 'T1',
            table_name: 'Table 1',
            zone_id: 'Z1',
            zone_name: 'Main',
            guest_count: 2,
            is_retail: false,
          },
          sequence: 1,
        },
        {
          event_id: 'evt-2',
          event_type: 'ITEMS_ADDED',
          timestamp: Date.now(),
          payload: { items: [] },
          sequence: 2,
        },
        {
          event_id: 'evt-3',
          event_type: 'PAYMENT_ADDED',
          timestamp: Date.now(),
          payload: {
            payment_id: 'pay-1',
            method: 'CASH',
            amount: 1000,
            tendered: null,
            change: null,
            note: null,
          },
          sequence: 3,
        },
        {
          event_id: 'evt-4',
          event_type: 'ORDER_COMPLETED',
          timestamp: Date.now(),
          payload: {
            final_total: 1000,
            receipt_number: 'FAC-001',
          },
          sequence: 4,
        },
      ];

      // Should render without throwing
      expect(() => render(<TimelineList events={mainEventTypes} />)).not.toThrow();
    });

    it('should handle gracefully malformed events', () => {
      const events: OrderEvent[] = [
        {
          event_id: 'evt-bad',
          event_type: 'UNKNOWN_TYPE' as any,
          timestamp: Date.now(),
          payload: {} as any,
          sequence: 1,
        },
      ];

      // Should render without throwing
      expect(() => render(<TimelineList events={events} />)).not.toThrow();
    });
  });

  describe('Performance', () => {
    it('should handle rendering many events efficiently', () => {
      const manyEvents: OrderEvent[] = Array.from({ length: 100 }, (_, i) => ({
        event_id: `evt-${i}`,
        event_type: 'PAYMENT_ADDED',
        timestamp: Date.now() - i * 1000,
        payload: {
          payment_id: `pay-${i}`,
          method: 'CASH',
          amount: 1000 + i,
          tendered: null,
          change: null,
          note: null,
        },
        sequence: i + 1,
      }));

      const startTime = performance.now();
      render(<TimelineList events={manyEvents} />);
      const renderTime = performance.now() - startTime;

      // Should render in reasonable time (< 1 second)
      expect(renderTime).toBeLessThan(1000);
    });
  });
});
