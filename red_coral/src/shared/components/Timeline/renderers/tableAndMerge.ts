import type {
  OrderMergedPayload,
  OrderMovedPayload,
  OrderMovedOutPayload,
  OrderMergedOutPayload,
  TableReassignedPayload,
} from '@/core/domain/types/orderEvent';
import { ArrowRight, ArrowLeft } from 'lucide-react';
import type { EventRenderer } from './types';

export const OrderMergedRenderer: EventRenderer<OrderMergedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.items && payload.items.length > 0) {
      const itemCount = payload.items.reduce((sum, item) => sum + item.quantity, 0);
      details.push(`${t('timeline.labels.items')}: ${itemCount}`);
    }

    return {
      title: t('timeline.order_merged'),
      summary: payload.source_table_name ? `${t('timeline.from')} ${payload.source_table_name}` : '',
      details,
      icon: ArrowLeft,
      colorClass: 'bg-purple-500',
      timestamp: event.timestamp,
    };
  }
};

export const OrderMovedRenderer: EventRenderer<OrderMovedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.items && payload.items.length > 0) {
      const itemCount = payload.items.reduce((sum, item) => sum + item.quantity, 0);
      details.push(`${t('timeline.labels.items')}: ${itemCount}`);
    }

    return {
      title: t('timeline.table_moved'),
      summary: (payload.source_table_name && payload.target_table_name)
        ? `${payload.source_table_name} → ${payload.target_table_name}`
        : '',
      details,
      icon: ArrowRight,
      colorClass: 'bg-indigo-500',
      timestamp: event.timestamp,
    };
  }
};

export const OrderMovedOutRenderer: EventRenderer<OrderMovedOutPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.reason) {
      details.push(`${t('timeline.labels.reason')}: ${payload.reason}`);
    }

    return {
      title: t('timeline.moved_out'),
      summary: payload.target_table_name ? `${t('timeline.to')} ${payload.target_table_name}` : '',
      details,
      icon: ArrowRight,
      colorClass: 'bg-indigo-600',
      timestamp: event.timestamp,
    };
  }
};

export const OrderMergedOutRenderer: EventRenderer<OrderMergedOutPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.reason) {
      details.push(`${t('timeline.labels.reason')}: ${payload.reason}`);
    }

    return {
      title: t('timeline.merged_out'),
      summary: payload.target_table_name ? `${t('timeline.to')} ${payload.target_table_name}` : '',
      details,
      icon: ArrowRight,
      colorClass: 'bg-purple-600',
      timestamp: event.timestamp,
    };
  }
};

export const TableReassignedRenderer: EventRenderer<TableReassignedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.target_zone_name) {
      details.push(`${t('timeline.labels.zone')}: ${payload.target_zone_name}`);
    }
    if (payload.items && payload.items.length > 0) {
      const itemCount = payload.items.reduce((sum, item) => sum + item.quantity, 0);
      details.push(`${t('timeline.labels.items')}: ${itemCount}`);
    }

    return {
      title: t('timeline.table_reassigned'),
      summary: (payload.source_table_name && payload.target_table_name)
        ? `${payload.source_table_name} → ${payload.target_table_name}`
        : '',
      details,
      icon: ArrowRight,
      colorClass: 'bg-blue-600',
      timestamp: event.timestamp,
    };
  }
};
