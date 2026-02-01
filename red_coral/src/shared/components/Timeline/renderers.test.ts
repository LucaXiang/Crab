/**
 * Timeline Event Renderers - Unit Tests
 *
 * 测试 Renderer 架构的完整性和鲁棒性
 * 不测试具体的文本输出（依赖翻译配置），只测试结构和错误处理
 */

import { describe, it, expect } from 'vitest';
import { renderEvent } from './renderers';
import type { OrderEvent, EventPayload } from '@/core/domain/types/orderEvent';

// Mock translation function
const mockT = (key: string, params?: Record<string, string | number>): string => {
  if (params) {
    return `${key}:${JSON.stringify(params)}`;
  }
  return key;
};

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

describe('Timeline Renderers - Architecture Tests', () => {
  describe('Renderer Registration', () => {
    it('should have renderer for all main event types', () => {
      const mainEvents: OrderEvent[] = [
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
        createMockEvent('ITEMS_ADDED', {
          type: 'ITEMS_ADDED',
          items: [],
        }),
        createMockEvent('ITEM_REMOVED', {
          type: 'ITEM_REMOVED',
          instance_id: 'inst-1',
          item_name: 'Item',
          reason: null,
          quantity: null,
        }),
        createMockEvent('PAYMENT_ADDED', {
          type: 'PAYMENT_ADDED',
          payment_id: 'pay-1',
          method: 'cash',
          amount: 1000,
          tendered: null,
          change: null,
          note: null,
        }),
        createMockEvent('ORDER_COMPLETED', {
          type: 'ORDER_COMPLETED',
          final_total: 1000,
          receipt_number: 'R1',
          payment_summary: [],
          service_type: 'DINE_IN',
        }),
        createMockEvent('ORDER_VOIDED', {
          type: 'ORDER_VOIDED',
          void_type: 'CANCELLED',
          loss_reason: null,
          loss_amount: null,
          note: null,
        }),
      ];

      mainEvents.forEach(event => {
        // Should not throw
        expect(() => renderEvent(event, mockT)).not.toThrow();

        // Should return valid structure
        const result = renderEvent(event, mockT);
        expect(result).toHaveProperty('title');
        expect(result).toHaveProperty('details');
        expect(result).toHaveProperty('icon');
        expect(result).toHaveProperty('colorClass');
        expect(result).toHaveProperty('timestamp');
      });
    });
  });

  describe('Data Structure Validation', () => {
    it('should return TimelineDisplayData structure', () => {
      const event = createMockEvent('TABLE_OPENED', {
        type: 'TABLE_OPENED',
        table_id: 'T1',
        table_name: 'Table 1',
        zone_id: 'Z1',
        zone_name: 'Main',
        guest_count: 2,
        is_retail: false,
        receipt_number: 'RCP-TEST',
      });

      const result = renderEvent(event, mockT);

      // Validate structure
      expect(typeof result.title).toBe('string');
      expect(Array.isArray(result.details)).toBe(true);
      expect(typeof result.colorClass).toBe('string');
      expect(typeof result.timestamp).toBe('number');
      expect(result.icon).toBeDefined();
    });

    it('should return non-empty title for all events', () => {
      const events: OrderEvent[] = [
        createMockEvent('ITEMS_ADDED', {
          type: 'ITEMS_ADDED',
          items: [],
        }),
        createMockEvent('PAYMENT_ADDED', {
          type: 'PAYMENT_ADDED',
          payment_id: 'pay-1',
          method: 'cash',
          amount: 1000,
          tendered: null,
          change: null,
          note: null,
        }),
      ];

      events.forEach(event => {
        const result = renderEvent(event, mockT);
        expect(result.title.length).toBeGreaterThan(0);
      });
    });
  });

  describe('Edge Cases', () => {
    it('should handle empty payload gracefully', () => {
      const event = createMockEvent('ITEMS_ADDED', {
        type: 'ITEMS_ADDED',
        items: [],
      });

      expect(() => renderEvent(event, mockT)).not.toThrow();
      const result = renderEvent(event, mockT);
      expect(result.details).toEqual([]);
    });

    it('should handle missing optional fields', () => {
      const event = createMockEvent('PAYMENT_ADDED', {
        type: 'PAYMENT_ADDED',
        payment_id: 'pay-1',
        method: 'cash',
        amount: 1000,
        tendered: null,
        change: null,
        note: null,
      });

      expect(() => renderEvent(event, mockT)).not.toThrow();
    });

    it('should preserve timestamp from event', () => {
      const timestamp = Date.now();
      const event = createMockEvent('TABLE_OPENED', {
        type: 'TABLE_OPENED',
        table_id: 'T1',
        table_name: 'Table 1',
        zone_id: 'Z1',
        zone_name: 'Main',
        guest_count: 2,
        is_retail: false,
        receipt_number: 'RCP-TEST',
      }, { timestamp });

      const result = renderEvent(event, mockT);
      expect(result.timestamp).toBe(timestamp);
    });
  });

  describe('Color Classes', () => {
    it('should assign unique color classes to different event types', () => {
      const events: OrderEvent[] = [
        createMockEvent('TABLE_OPENED', { type: 'TABLE_OPENED', table_id: 'T1', table_name: 'T1', zone_id: 'Z1', zone_name: 'Z', guest_count: 2, is_retail: false, receipt_number: 'RCP-TEST' }),
        createMockEvent('ITEMS_ADDED', { type: 'ITEMS_ADDED', items: [] }),
        createMockEvent('PAYMENT_ADDED', { type: 'PAYMENT_ADDED', payment_id: 'p1', method: 'cash', amount: 100, tendered: null, change: null, note: null }),
        createMockEvent('ORDER_COMPLETED', { type: 'ORDER_COMPLETED', final_total: 100, receipt_number: 'R1', payment_summary: [], service_type: 'DINE_IN' }),
        createMockEvent('ORDER_VOIDED', { type: 'ORDER_VOIDED', void_type: 'CANCELLED', loss_reason: null, loss_amount: null, note: null }),
      ];

      const colors = new Set<string>();

      events.forEach(event => {
        const result = renderEvent(event, mockT);
        expect(result.colorClass).toMatch(/^bg-/);
        colors.add(result.colorClass);
      });

      // Should have at least 3 different colors
      expect(colors.size).toBeGreaterThanOrEqual(3);
    });
  });

  describe('Translation Integration', () => {
    it('should call translation function with keys', () => {
      const translationKeys: string[] = [];
      const trackingT = (key: string, params?: Record<string, string | number>) => {
        translationKeys.push(key);
        return mockT(key, params);
      };

      const event = createMockEvent('TABLE_OPENED', {
        type: 'TABLE_OPENED',
        table_id: 'T1',
        table_name: 'Table 1',
        zone_id: 'Z1',
        zone_name: 'Main',
        guest_count: 2,
        is_retail: false,
        receipt_number: 'RCP-TEST',
      });

      renderEvent(event, trackingT);

      // Should have called translation at least once
      expect(translationKeys.length).toBeGreaterThan(0);
      expect(translationKeys.some(k => k.startsWith('timeline.'))).toBe(true);
    });
  });

  describe('Robustness', () => {
    it('should handle renderer lookup without errors', () => {
      // Test that the renderer system doesn't crash on lookups
      const validEvent = createMockEvent('TABLE_OPENED', {
        type: 'TABLE_OPENED',
        table_id: 'T1',
        table_name: 'Table 1',
        zone_id: 'Z1',
        zone_name: 'Main',
        guest_count: 2,
        is_retail: false,
        receipt_number: 'RCP-TEST',
      });

      // Multiple lookups should work
      expect(() => renderEvent(validEvent, mockT)).not.toThrow();
      expect(() => renderEvent(validEvent, mockT)).not.toThrow();
    });

    it('should handle concurrent rendering calls', () => {
      const event = createMockEvent('TABLE_OPENED', {
        type: 'TABLE_OPENED',
        table_id: 'T1',
        table_name: 'Table 1',
        zone_id: 'Z1',
        zone_name: 'Main',
        guest_count: 2,
        is_retail: false,
        receipt_number: 'RCP-TEST',
      });

      // Simulate concurrent calls
      const results = Array.from({ length: 10 }, () => renderEvent(event, mockT));

      // All results should be identical
      results.forEach(result => {
        expect(result.title).toBe(results[0].title);
        expect(result.colorClass).toBe(results[0].colorClass);
      });
    });
  });
});
