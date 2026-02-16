import { useState, useCallback, useEffect } from 'react';
import type { MemberStampProgressDetail } from '@/core/domain/types/api';
import type { HeldOrder } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { CommandFailedError } from '@/core/stores/order/commands/sendCommand';
import { commandErrorMessage } from '@/utils/error/commandError';
import { redeemStamp, cancelStampRedemption } from '@/core/stores/order/commands';
import { getMemberDetail } from '@/features/member/mutations';
import { getMatchingItems, getDesignatedMatchingItems } from '@/utils/stampMatching';

export function useStampProgress(order: HeldOrder) {
  const { t } = useI18n();
  const [stampProgress, setStampProgress] = useState<MemberStampProgressDetail[]>([]);
  const [rewardPickerActivity, setRewardPickerActivity] = useState<MemberStampProgressDetail | null>(null);
  const [stampRedeemActivity, setStampRedeemActivity] = useState<MemberStampProgressDetail | null>(null);

  useEffect(() => {
    if (order.member_id) {
      getMemberDetail(order.member_id)
        .then((detail) => setStampProgress(detail.stamp_progress))
        .catch(() => setStampProgress([]));
    } else {
      setStampProgress([]);
    }
  }, [order.member_id]);

  const refreshStampProgress = useCallback(async () => {
    if (order.member_id) {
      const detail = await getMemberDetail(order.member_id);
      setStampProgress(detail.stamp_progress);
    }
  }, [order.member_id]);

  const handleMatchRedeem = useCallback(async (sp: MemberStampProgressDetail) => {
    const isDesignated = sp.reward_strategy === 'DESIGNATED';
    const matchingItems = isDesignated
      ? getDesignatedMatchingItems(order.items, sp)
      : getMatchingItems(order.items, sp);
    if (matchingItems.length === 0) return;
    const bestMatch = isDesignated
      ? matchingItems[0]
      : sp.reward_strategy === 'ECONOMIZADOR'
        ? matchingItems.reduce((a, b) => a.original_price <= b.original_price ? a : b)
        : matchingItems.reduce((a, b) => a.original_price >= b.original_price ? a : b);
    try {
      await redeemStamp(order.order_id, sp.stamp_activity_id, null, bestMatch.instance_id);
      toast.success(t('checkout.stamp.redeemed'));
      await refreshStampProgress();
    } catch (e) {
      toast.error(e instanceof CommandFailedError
        ? commandErrorMessage(e.code)
        : t('checkout.stamp.redeem_failed'));
    }
  }, [order.order_id, order.items, refreshStampProgress, t]);

  const handleSelectionRedeem = useCallback(async (activityId: number, productId: number) => {
    setRewardPickerActivity(null);
    try {
      await redeemStamp(order.order_id, activityId, productId);
      toast.success(t('checkout.stamp.redeemed'));
      await refreshStampProgress();
    } catch (e) {
      toast.error(e instanceof CommandFailedError
        ? commandErrorMessage(e.code)
        : t('checkout.stamp.redeem_failed'));
    }
  }, [order.order_id, refreshStampProgress, t]);

  const handleDirectRedeem = useCallback(async (activityId: number) => {
    try {
      await redeemStamp(order.order_id, activityId);
      toast.success(t('checkout.stamp.redeemed'));
      await refreshStampProgress();
    } catch (e) {
      toast.error(e instanceof CommandFailedError
        ? commandErrorMessage(e.code)
        : t('checkout.stamp.redeem_failed'));
    }
  }, [order.order_id, refreshStampProgress, t]);

  const handleCancelStampRedemption = useCallback(async (activityId: number) => {
    try {
      await cancelStampRedemption(order.order_id, activityId);
      toast.success(t('checkout.stamp.cancel_success'));
      await refreshStampProgress();
    } catch {
      toast.error(t('checkout.stamp.cancel_failed'));
    }
  }, [order.order_id, refreshStampProgress, t]);

  return {
    stampProgress,
    rewardPickerActivity,
    setRewardPickerActivity,
    stampRedeemActivity,
    setStampRedeemActivity,
    handleMatchRedeem,
    handleSelectionRedeem,
    handleDirectRedeem,
    handleCancelStampRedemption,
  };
}
