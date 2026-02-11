/**
 * Timeline Event Renderers
 *
 * 职责分离设计：
 * - 每个 OrderEvent Payload 有独立的 Renderer
 * - Renderer 负责将事件数据转换为 UI 展示数据
 * - 通过注册表映射，无需 switch case
 */

import type { OrderEvent, OrderEventType } from '@/core/domain/types/orderEvent';
import { Tag } from 'lucide-react';

// Re-export types for consumers
export type { TimelineTag, DetailTag, TimelineDisplayData, EventRenderer, TranslateFn } from './types';

// Renderer imports
import { TableOpenedRenderer, OrderCompletedRenderer, OrderVoidedRenderer } from './orderLifecycle';
import { ItemsAddedRenderer, ItemModifiedRenderer, ItemRemovedRenderer, ItemCompedRenderer, ItemUncompedRenderer } from './itemOperations';
import { PaymentAddedRenderer, PaymentCancelledRenderer } from './payments';
import { ItemSplitRenderer, AmountSplitRenderer, AaSplitStartedRenderer, AaSplitPaidRenderer, AaSplitCancelledRenderer } from './splits';
import { OrderMergedRenderer, OrderMovedRenderer, OrderMovedOutRenderer, OrderMergedOutRenderer, TableReassignedRenderer } from './tableAndMerge';
import { OrderInfoUpdatedRenderer, RuleSkipToggledRenderer, OrderDiscountAppliedRenderer, OrderSurchargeAppliedRenderer, OrderNoteAddedRenderer, MemberLinkedRenderer, MemberUnlinkedRenderer, StampRedeemedRenderer, StampRedemptionCancelledRenderer } from './orderInfo';

import type { EventRenderer as EventRendererType } from './types';
import type { TranslateFn } from './types';
import type { TimelineDisplayData } from './types';

// eslint-disable-next-line @typescript-eslint/no-explicit-any -- heterogeneous registry requires existential type
export const EVENT_RENDERERS: Record<OrderEventType, EventRendererType<any>> = {
  TABLE_OPENED: TableOpenedRenderer,
  ITEMS_ADDED: ItemsAddedRenderer,
  ITEM_MODIFIED: ItemModifiedRenderer,
  ITEM_REMOVED: ItemRemovedRenderer,
  ITEM_COMPED: ItemCompedRenderer,
  ITEM_UNCOMPED: ItemUncompedRenderer,
  PAYMENT_ADDED: PaymentAddedRenderer,
  PAYMENT_CANCELLED: PaymentCancelledRenderer,
  ITEM_SPLIT: ItemSplitRenderer,
  AMOUNT_SPLIT: AmountSplitRenderer,
  AA_SPLIT_STARTED: AaSplitStartedRenderer,
  AA_SPLIT_PAID: AaSplitPaidRenderer,
  AA_SPLIT_CANCELLED: AaSplitCancelledRenderer,
  ORDER_COMPLETED: OrderCompletedRenderer,
  ORDER_VOIDED: OrderVoidedRenderer,
  ORDER_MERGED: OrderMergedRenderer,
  ORDER_MOVED: OrderMovedRenderer,
  ORDER_MOVED_OUT: OrderMovedOutRenderer,
  ORDER_MERGED_OUT: OrderMergedOutRenderer,
  TABLE_REASSIGNED: TableReassignedRenderer,
  ORDER_INFO_UPDATED: OrderInfoUpdatedRenderer,
  RULE_SKIP_TOGGLED: RuleSkipToggledRenderer,
  ORDER_DISCOUNT_APPLIED: OrderDiscountAppliedRenderer,
  ORDER_SURCHARGE_APPLIED: OrderSurchargeAppliedRenderer,
  ORDER_NOTE_ADDED: OrderNoteAddedRenderer,
  MEMBER_LINKED: MemberLinkedRenderer,
  MEMBER_UNLINKED: MemberUnlinkedRenderer,
  STAMP_REDEEMED: StampRedeemedRenderer,
  STAMP_REDEMPTION_CANCELLED: StampRedemptionCancelledRenderer,
};

/**
 * 渲染单个事件
 */
export function renderEvent(event: OrderEvent, t: TranslateFn): TimelineDisplayData {
  const renderer = EVENT_RENDERERS[event.event_type];

  if (!renderer) {
    return {
      title: event.event_type,
      summary: '',
      details: [],
      icon: Tag,
      colorClass: 'bg-gray-400',
      timestamp: event.timestamp,
    };
  }

  return renderer.render(event, event.payload, t);
}
