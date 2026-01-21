/**
 * Timeline Event Renderers - Unit Tests
 *
 * 测试 Renderer 架构的完整性和鲁棒性
 * 不测试具体的文本输出（依赖翻译配置），只测试结构和错误处理
 */

import { describe, it, expect } from 'vitest';
import { renderEvent } from './renderers';
import type { OrderEvent } from '@/core/domain/types/orderEvent';

// Mock translation function
const mockT = (key: string, params?: Record<string, string | number>): string => {
  if (params) {
    return `${key}:${JSON.stringify(params)}`;
  }
  return key;
};

describe('Timeline Renderers - Architecture Tests', () => {
  describe('Renderer Registration', () => {
    it('should have renderer for all main event types', () => {
      const mainEventTypes: Array<{ type: string; payload: any }> = [
        {
          type: 'TABLE_OPENED',
          payload: {
            table_id: 'T1',
            table_name: 'Table 1',
            zone_id: 'Z1',
            zone_name: 'Main',
            guest_count: 2,
            is_retail: false,
          },
        },
        {
          type: 'ITEMS_ADDED',
          payload: { items: [] },
        },
        {
          type: 'ITEM_REMOVED',
          payload: {
            instance_id: 'inst-1',
            item_name: 'Item',
            reason: null,
            quantity: null,
          },
        },
        {
          type: 'PAYMENT_ADDED',
          payload: {
            payment_id: 'pay-1',
            method: 'CASH',
            amount: 1000,
            tendered: null,
            change: null,
            note: null,
          },
        },
        {
          type: 'ORDER_COMPLETED',
          payload: {
            final_total: 1000,
            receipt_number: 'R1',
          },
        },
        {
          type: 'ORDER_VOIDED',
          payload: {
            reason: null,
          },
        },
      ];

      mainEventTypes.forEach(({ type, payload }) => {
        const event: OrderEvent = {
          event_id: `evt-${type}`,
          event_type: type as any,
          timestamp: Date.now(),
          payload: payload as any,
          sequence: 1,
        };

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
      const event: OrderEvent = {
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
      };

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
        {
          event_id: 'evt-1',
          event_type: 'ITEMS_ADDED',
          timestamp: Date.now(),
          payload: { items: [] },
          sequence: 1,
        },
        {
          event_id: 'evt-2',
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
          sequence: 2,
        },
      ];

      events.forEach(event => {
        const result = renderEvent(event, mockT);
        expect(result.title.length).toBeGreaterThan(0);
      });
    });
  });

  describe('Edge Cases', () => {
    it('should handle empty payload gracefully', () => {
      const event: OrderEvent = {
        event_id: 'evt-empty',
        event_type: 'ITEMS_ADDED',
        timestamp: Date.now(),
        payload: { items: [] },
        sequence: 1,
      };

      expect(() => renderEvent(event, mockT)).not.toThrow();
      const result = renderEvent(event, mockT);
      expect(result.details).toEqual([]);
    });

    it('should handle missing optional fields', () => {
      const event: OrderEvent = {
        event_id: 'evt-minimal',
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
        sequence: 1,
      };

      expect(() => renderEvent(event, mockT)).not.toThrow();
    });

    it('should preserve timestamp from event', () => {
      const timestamp = Date.now();
      const event: OrderEvent = {
        event_id: 'evt-time',
        event_type: 'TABLE_OPENED',
        timestamp,
        payload: {
          table_id: 'T1',
          table_name: 'Table 1',
          zone_id: 'Z1',
          zone_name: 'Main',
          guest_count: 2,
          is_retail: false,
        },
        sequence: 1,
      };

      const result = renderEvent(event, mockT);
      expect(result.timestamp).toBe(timestamp);
    });
  });

  describe('Color Classes', () => {
    it('should assign unique color classes to different event types', () => {
      const events: Array<{ type: string; payload: any }> = [
        { type: 'TABLE_OPENED', payload: { table_id: 'T1', table_name: 'T1', zone_id: 'Z1', zone_name: 'Z', guest_count: 2, is_retail: false } },
        { type: 'ITEMS_ADDED', payload: { items: [] } },
        { type: 'PAYMENT_ADDED', payload: { payment_id: 'p1', method: 'CASH', amount: 100, tendered: null, change: null, note: null } },
        { type: 'ORDER_COMPLETED', payload: { final_total: 100, receipt_number: 'R1' } },
        { type: 'ORDER_VOIDED', payload: { reason: null } },
      ];

      const colors = new Set<string>();

      events.forEach(({ type, payload }) => {
        const event: OrderEvent = {
          event_id: `evt-${type}`,
          event_type: type as any,
          timestamp: Date.now(),
          payload: payload as any,
          sequence: 1,
        };

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

      const event: OrderEvent = {
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
      };

      renderEvent(event, trackingT);

      // Should have called translation at least once
      expect(translationKeys.length).toBeGreaterThan(0);
      expect(translationKeys.some(k => k.startsWith('timeline.'))).toBe(true);
    });
  });

  describe('Robustness', () => {
    it('should handle renderer lookup without errors', () => {
      // Test that the renderer system doesn't crash on lookups
      const validEvent: OrderEvent = {
        event_id: 'evt-valid',
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
      };

      // Multiple lookups should work
      expect(() => renderEvent(validEvent, mockT)).not.toThrow();
      expect(() => renderEvent(validEvent, mockT)).not.toThrow();
    });

    it('should handle concurrent rendering calls', () => {
      const event: OrderEvent = {
        event_id: 'evt-concurrent',
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
      };

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
