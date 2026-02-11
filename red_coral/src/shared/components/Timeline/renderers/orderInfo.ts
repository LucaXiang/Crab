import type {
  OrderInfoUpdatedPayload,
  RuleSkipToggledPayload,
  OrderDiscountAppliedPayload,
  OrderSurchargeAppliedPayload,
  OrderNoteAddedPayload,
  MemberLinkedPayload,
  MemberUnlinkedPayload,
  StampRedeemedPayload,
  StampRedemptionCancelledPayload,
} from '@/core/domain/types/orderEvent';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { Currency } from '@/utils/currency';
import { Edit3, Tag, UserPlus, UserMinus, Award } from 'lucide-react';
import type { EventRenderer, DetailTag } from './types';

export const OrderInfoUpdatedRenderer: EventRenderer<OrderInfoUpdatedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];
    if (payload.guest_count != null) {
      details.push(`${t('timeline.labels.guests')}: ${payload.guest_count}`);
    }
    if (payload.table_name != null) {
      details.push(`${t('timeline.labels.table')}: ${payload.table_name}`);
    }
    if (payload.is_pre_payment != null) {
      details.push(`${t('timeline.labels.pre_payment')}: ${payload.is_pre_payment ? t('common.yes') : t('common.no')}`);
    }

    return {
      title: t('timeline.order_info_updated'),
      details,
      icon: Edit3,
      colorClass: 'bg-blue-400',
      timestamp: event.timestamp,
    };
  }
};

export const RuleSkipToggledRenderer: EventRenderer<RuleSkipToggledPayload> = {
  render(event, payload, t) {
    const actionLabel = payload.skipped
      ? t('timeline.rule_skipped')
      : t('timeline.rule_applied');

    return {
      title: t('timeline.rule_toggled'),
      summary: `${actionLabel}: ${payload.rule_name}`,
      details: [],
      icon: Tag,
      colorClass: payload.skipped ? 'bg-orange-400' : 'bg-green-400',
      timestamp: event.timestamp,
    };
  }
};

export const OrderDiscountAppliedRenderer: EventRenderer<OrderDiscountAppliedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];
    const detailTags: DetailTag[] = [];
    const isClearing = !payload.discount_percent && !payload.discount_fixed;

    if (payload.discount_percent != null) {
      const computed = payload.discount !== 0 ? ` (-${formatCurrency(payload.discount)})` : '';
      detailTags.push({
        label: t('timeline.labels.discount'),
        value: `${payload.discount_percent}%${computed}`,
        colorClass: 'bg-orange-100 text-orange-700 border-orange-200',
      });
    } else if (payload.discount_fixed != null) {
      detailTags.push({
        label: t('timeline.labels.discount'),
        value: `-${formatCurrency(payload.discount_fixed)}`,
        colorClass: 'bg-orange-100 text-orange-700 border-orange-200',
      });
    }
    details.push(`${t('timeline.labels.subtotal')}: ${formatCurrency(payload.subtotal)}`);
    details.push(`${t('timeline.labels.discount')}: -${formatCurrency(payload.discount)}`);
    details.push(`${t('timeline.labels.total')}: ${formatCurrency(payload.total)}`);

    return {
      title: isClearing ? t('timeline.discount_cleared') : t('timeline.discount_applied'),
      details,
      detailTags,
      icon: Tag,
      colorClass: isClearing ? 'bg-gray-400' : 'bg-orange-400',
      timestamp: event.timestamp,
    };
  }
};

export const OrderSurchargeAppliedRenderer: EventRenderer<OrderSurchargeAppliedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];
    const detailTags: DetailTag[] = [];
    const isClearing = !payload.surcharge_amount && !payload.surcharge_percent;

    if (payload.surcharge_percent != null) {
      const computed = payload.surcharge !== 0 ? ` (+${formatCurrency(payload.surcharge)})` : '';
      detailTags.push({
        label: t('timeline.labels.surcharge'),
        value: `${payload.surcharge_percent}%${computed}`,
        colorClass: 'bg-purple-100 text-purple-700 border-purple-200',
      });
    } else if (payload.surcharge_amount != null) {
      detailTags.push({
        label: t('timeline.labels.surcharge'),
        value: `+${formatCurrency(payload.surcharge_amount)}`,
        colorClass: 'bg-purple-100 text-purple-700 border-purple-200',
      });
    }
    details.push(`${t('timeline.labels.subtotal')}: ${formatCurrency(payload.subtotal)}`);
    details.push(`${t('timeline.labels.surcharge')}: +${formatCurrency(payload.surcharge)}`);
    details.push(`${t('timeline.labels.total')}: ${formatCurrency(payload.total)}`);

    return {
      title: isClearing ? t('timeline.surcharge_cleared') : t('timeline.surcharge_applied'),
      details,
      detailTags,
      icon: Tag,
      colorClass: isClearing ? 'bg-gray-400' : 'bg-purple-400',
      timestamp: event.timestamp,
    };
  }
};

export const OrderNoteAddedRenderer: EventRenderer<OrderNoteAddedPayload> = {
  render(event, payload, t) {
    const isClearing = payload.note === '';
    const details: string[] = [];

    if (!isClearing) {
      details.push(payload.note);
    }
    if (payload.previous_note) {
      details.push(`${t('timeline.labels.previous')}: ${payload.previous_note}`);
    }

    return {
      title: isClearing ? t('timeline.note_cleared') : t('timeline.note_added'),
      details,
      icon: Edit3,
      colorClass: 'bg-blue-400',
      timestamp: event.timestamp,
    };
  }
};

export const MemberLinkedRenderer: EventRenderer<MemberLinkedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.member_linked'),
      summary: payload.member_name,
      details: [
        `${t('timeline.labels.marketing_group')}: ${payload.marketing_group_name}`,
      ],
      icon: UserPlus,
      colorClass: 'bg-teal-400',
      timestamp: event.timestamp,
    };
  }
};

export const MemberUnlinkedRenderer: EventRenderer<MemberUnlinkedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.member_unlinked'),
      summary: payload.previous_member_name,
      details: [],
      icon: UserMinus,
      colorClass: 'bg-gray-400',
      timestamp: event.timestamp,
    };
  }
};

export const StampRedeemedRenderer: EventRenderer<StampRedeemedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.stamp_redeemed'),
      summary: payload.stamp_activity_name,
      details: [
        `${t('timeline.labels.reward_strategy')}: ${payload.reward_strategy}`,
      ],
      icon: Award,
      colorClass: 'bg-amber-400',
      timestamp: event.timestamp,
    };
  }
};

export const StampRedemptionCancelledRenderer: EventRenderer<StampRedemptionCancelledPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.stamp_redemption_cancelled'),
      summary: payload.stamp_activity_name,
      details: [],
      icon: Award,
      colorClass: 'bg-gray-400',
      timestamp: event.timestamp,
    };
  }
};
