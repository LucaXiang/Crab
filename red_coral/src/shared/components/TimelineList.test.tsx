/**
 * TimelineList - Integration Tests
 *
 * 测试 Timeline 组件与 Renderer 的集成，专注于架构而非具体的翻译输出
 */

import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TimelineList } from './TimelineList';
import type { OrderEvent, EventPayload } from '@/core/domain/types/orderEvent';

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

// Helper to create properly typed mock OrderEvent
const createMockEvent = (event_type: string, payload: EventPayload, overrides?: Partial<OrderEvent>): OrderEvent => ({
  event_id: `evt-${event_type}`,
  sequence: 1,
  order_id: 'order-test-1',
  timestamp: Date.now(),
  operator_id: 'op-1',
  operator_name: 'Test Operator',
  command_id: 'cmd-1',
  event_type: event_type as OrderEvent['event_type'],
  payload,
  ...overrides,
});

describe('TimelineList Integration Tests', () => {
  describe('Basic Rendering', () => {
    it('should render without errors when given empty array', () => {
      expect(() => render(<TimelineList events={[]} />)).not.toThrow();
    });

    it('should render timeline items for valid events', () => {
      const events: OrderEvent[] = [
        createMockEvent('TABLE_OPENED', {
          type: 'TABLE_OPENED',
          table_id: 'T1',
          table_name: 'Table 1',
          zone_id: 'Z1',
          zone_name: 'Main',
          guest_count: 2,
          is_retail: false,
          receipt_number: 'RCP-TEST',
        }),
      ];

      const { container } = render(<TimelineList events={events} />);
      // Should render timeline structure with icon, timestamp, and title
      expect(container.querySelector('.rounded-full')).toBeTruthy(); // Icon container
      expect(container.querySelector('.font-mono')).toBeTruthy(); // Timestamp
      expect(container.querySelector('.font-bold')).toBeTruthy(); // Title
    });

    it('should render multiple events', () => {
      const events: OrderEvent[] = [
        createMockEvent('TABLE_OPENED', {
          type: 'TABLE_OPENED',
          table_id: 'T1',
          table_name: 'Table 1',
          zone_id: 'Z1',
          zone_name: 'Main',
          guest_count: 2,
          is_retail: false,
          receipt_number: 'RCP-TEST',
        }, { event_id: 'evt-1', timestamp: Date.now() - 2000, sequence: 1 }),
        createMockEvent('PAYMENT_ADDED', {
          type: 'PAYMENT_ADDED',
          payment_id: 'pay-1',
          method: 'cash',
          amount: 1000,
          tendered: null,
          change: null,
          note: null,
        }, { event_id: 'evt-2', timestamp: Date.now() - 1000, sequence: 2 }),
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
        createMockEvent('ITEMS_ADDED', {
          type: 'ITEMS_ADDED',
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
        }),
      ];

      const { container } = render(<TimelineList events={events} />);
      expect(container.textContent).toContain('Test Pizza');
    });

    it('should render payment added event', () => {
      const events: OrderEvent[] = [
        createMockEvent('PAYMENT_ADDED', {
          type: 'PAYMENT_ADDED',
          payment_id: 'pay-1',
          method: 'cash',
          amount: 5000,
          tendered: null,
          change: null,
          note: 'Test payment note',
        }),
      ];

      const { container } = render(<TimelineList events={events} />);
      expect(container.textContent).toContain('Test payment note');
    });

    it('should render completion event', () => {
      const events: OrderEvent[] = [
        createMockEvent('ORDER_COMPLETED', {
          type: 'ORDER_COMPLETED',
          final_total: 10000,
          receipt_number: 'FAC-2026-001',
          payment_summary: [],
          service_type: 'DINE_IN',
        }),
      ];

      const { container } = render(<TimelineList events={events} />);
      expect(container.textContent).toContain('FAC-2026-001');
    });

    it('should render void event', () => {
      const events: OrderEvent[] = [
        createMockEvent('ORDER_VOIDED', {
          type: 'ORDER_VOIDED',
          void_type: 'CANCELLED',
          loss_reason: null,
          loss_amount: null,
          note: 'Test cancellation',
        }),
      ];

      const { container } = render(<TimelineList events={events} />);
      expect(container.textContent).toContain('Test cancellation');
    });
  });

  describe('Error Handling', () => {
    it('should handle main event types without crashing', () => {
      const mainEventTypes: OrderEvent[] = [
        createMockEvent('TABLE_OPENED', {
          type: 'TABLE_OPENED',
          table_id: 'T1',
          table_name: 'Table 1',
          zone_id: 'Z1',
          zone_name: 'Main',
          guest_count: 2,
          is_retail: false,
          receipt_number: 'RCP-TEST',
        }, { event_id: 'evt-1', sequence: 1 }),
        createMockEvent('ITEMS_ADDED', {
          type: 'ITEMS_ADDED',
          items: [],
        }, { event_id: 'evt-2', sequence: 2 }),
        createMockEvent('PAYMENT_ADDED', {
          type: 'PAYMENT_ADDED',
          payment_id: 'pay-1',
          method: 'cash',
          amount: 1000,
          tendered: null,
          change: null,
          note: null,
        }, { event_id: 'evt-3', sequence: 3 }),
        createMockEvent('ORDER_COMPLETED', {
          type: 'ORDER_COMPLETED',
          final_total: 1000,
          receipt_number: 'FAC-001',
          payment_summary: [],
          service_type: 'DINE_IN',
        }, { event_id: 'evt-4', sequence: 4 }),
      ];

      // Should render without throwing
      expect(() => render(<TimelineList events={mainEventTypes} />)).not.toThrow();
    });

    it('should handle gracefully malformed events', () => {
      // Create a minimally valid event structure with unknown type
      const events: OrderEvent[] = [
        {
          event_id: 'evt-bad',
          sequence: 1,
          order_id: 'order-test-1',
          timestamp: Date.now(),
          operator_id: 'op-1',
          operator_name: 'Test Operator',
          command_id: 'cmd-1',
          event_type: 'UNKNOWN_TYPE' as any,
          payload: {} as any,
        },
      ];

      // Should render without throwing
      expect(() => render(<TimelineList events={events} />)).not.toThrow();
    });
  });

  describe('Performance', () => {
    it('should handle rendering many events efficiently', () => {
      const manyEvents: OrderEvent[] = Array.from({ length: 100 }, (_, i) =>
        createMockEvent('PAYMENT_ADDED', {
          type: 'PAYMENT_ADDED',
          payment_id: `pay-${i}`,
          method: 'cash',
          amount: 1000 + i,
          tendered: null,
          change: null,
          note: null,
        }, {
          event_id: `evt-${i}`,
          timestamp: Date.now() - i * 1000,
          sequence: i + 1,
        })
      );

      const startTime = performance.now();
      render(<TimelineList events={manyEvents} />);
      const renderTime = performance.now() - startTime;

      // Should render in reasonable time (< 1 second)
      expect(renderTime).toBeLessThan(1000);
    });
  });
});
