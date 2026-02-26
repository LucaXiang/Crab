/**
 * MemberDetailMode - 全屏会员详情页
 *
 * 三栏布局：
 * - 左侧: OrderSidebar
 * - 中间: 会员组折扣规则、集章进度 & 兑换操作
 * - 右侧: 会员基本信息 + 取消关联
 */

import React, { useState, useCallback, useEffect, useMemo } from 'react';
import { ArrowLeft, Crown, UserX, Stamp, X, Tag, Percent, Globe, Layers, Package } from 'lucide-react';
import { HeldOrder } from '@/core/domain/types';
import type { MemberStampProgressDetail, MarketingGroupDetail, MgDiscountRule, ProductScope } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { unlinkMember, redeemStamp, cancelStampRedemption } from '@/core/stores/order/commands';
import { CommandFailedError } from '@/core/stores/order/commands/sendCommand';
import { commandErrorMessage } from '@/utils/error/commandError';
import { getMemberDetail } from '@/features/member/mutations';
import { getMarketingGroupDetail } from '@/features/marketing-group/mutations';
import { useCategoryStore } from '@/features/category/store';
import { useTagStore } from '@/features/tag/store';
import { useProductStore } from '@/core/stores/resources';
import { StampRewardPickerModal } from './payment/StampRewardPickerModal';
import { StampRedeemModal } from './payment/StampRedeemModal';
import { getMatchingItems, getDesignatedMatchingItems } from '@/utils/stampMatching';

interface MemberDetailModeProps {
  order: HeldOrder;
  totalPaid: number;
  remaining: number;
  onBack: () => void;
  onManageTable?: () => void;
}

export const MemberDetailMode: React.FC<MemberDetailModeProps> = ({
  order,
  totalPaid,
  remaining,
  onBack,
  onManageTable,
}) => {
  const { t } = useI18n();
  const categories = useCategoryStore(s => s.items);
  const tags = useTagStore(s => s.items);
  const products = useProductStore(s => s.items);
  const [isProcessing, setIsProcessing] = useState(false);
  const [stampProgress, setStampProgress] = useState<MemberStampProgressDetail[]>([]);
  const [groupDetail, setGroupDetail] = useState<MarketingGroupDetail | null>(null);
  const [rewardPickerActivity, setRewardPickerActivity] = useState<MemberStampProgressDetail | null>(null);
  const [stampRedeemActivity, setStampRedeemActivity] = useState<MemberStampProgressDetail | null>(null);

  const getTargetName = useCallback((scope: ProductScope, targetId: number | null): string | null => {
    if (targetId == null) return null;
    switch (scope) {
      case 'CATEGORY': return categories.find(c => c.id === targetId)?.name || String(targetId);
      case 'TAG': return tags.find(t => t.id === targetId)?.name || String(targetId);
      case 'PRODUCT': return products.find(p => p.id === targetId)?.name || String(targetId);
      default: return null;
    }
  }, [categories, tags, products]);

  // Fetch member stamp progress + marketing group detail
  useEffect(() => {
    if (order.member_id) {
      getMemberDetail(order.member_id)
        .then((detail) => setStampProgress(detail.stamp_progress))
        .catch(() => setStampProgress([]));
    }
    if (order.marketing_group_id) {
      getMarketingGroupDetail(order.marketing_group_id)
        .then(setGroupDetail)
        .catch(() => setGroupDetail(null));
    }
  }, [order.member_id, order.marketing_group_id]);

  const handleUnlink = useCallback(async () => {
    setIsProcessing(true);
    try {
      await unlinkMember(order.order_id);
      toast.success(t('checkout.member.unlinked'));
      onBack();
    } catch {
      toast.error(t('checkout.member.unlink_failed'));
    } finally {
      setIsProcessing(false);
    }
  }, [order.order_id, onBack, t]);

  // Stamp redeem handlers (same logic as SelectModePage)
  const refreshStampProgress = useCallback(async () => {
    if (order.member_id) {
      try {
        const detail = await getMemberDetail(order.member_id);
        setStampProgress(detail.stamp_progress);
      } catch { /* ignore */ }
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

  // Active discount rules
  const discountRules = useMemo(() =>
    groupDetail?.discount_rules.filter(r => r.is_active) ?? [],
    [groupDetail]
  );

  return (
    <div className="h-full flex bg-gray-50/50 backdrop-blur-xl">
      <OrderSidebar
        order={order}
        totalPaid={totalPaid}
        remaining={remaining}
        onManage={onManageTable}
      />

      <div className="flex-1 flex flex-col h-full overflow-hidden relative">
        {/* Background Decor */}
        <div className="absolute top-[-20%] left-[-10%] w-[600px] h-[600px] bg-primary-100/50 rounded-full mix-blend-multiply filter blur-[100px] opacity-50 pointer-events-none" />
        <div className="absolute bottom-[-20%] right-[-10%] w-[600px] h-[600px] bg-violet-100/50 rounded-full mix-blend-multiply filter blur-[100px] opacity-50 pointer-events-none" />

        {/* Header */}
        <div className="p-6 bg-white/80 backdrop-blur-md border-b border-gray-200/50 shadow-sm flex justify-between items-center z-10 shrink-0">
          <h3 className="font-bold text-gray-800 text-2xl flex items-center gap-3">
            <div className="p-2 bg-primary-500 rounded-xl text-white shadow-lg shadow-primary-500/30">
              <Crown size={24} />
            </div>
            {t('checkout.member.detail_title')}
          </h3>
          <button
            onClick={onBack}
            className="px-5 py-2.5 bg-white border border-gray-200 hover:bg-gray-50 hover:border-gray-300 text-gray-700 rounded-xl font-medium flex items-center gap-2 transition-all shadow-sm"
          >
            <ArrowLeft size={20} /> {t('common.action.back')}
          </button>
        </div>

        <div className="flex-1 flex overflow-hidden z-10">
          {/* Center: Rules + Stamp Progress */}
          <div className="flex-1 flex flex-col border-r border-gray-200/60 bg-white/50 backdrop-blur-sm min-w-0">
            <div className="flex-1 overflow-y-auto p-6 space-y-8 custom-scrollbar">

              {/* Discount Rules Section */}
              {discountRules.length > 0 && (
                <div>
                  <h4 className="text-sm font-bold uppercase tracking-wider text-gray-500 mb-4 flex items-center gap-2">
                    <Tag size={14} />
                    {t('checkout.member.discount_rules')}
                  </h4>
                  <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
                    {discountRules.map((rule) => (
                      <DiscountRuleCard key={rule.id} rule={rule} targetName={getTargetName(rule.product_scope, rule.target_id)} t={t} />
                    ))}
                  </div>
                </div>
              )}

              {/* Stamp Progress Section */}
              {stampProgress.length > 0 && (
                <div>
                  <h4 className="text-sm font-bold uppercase tracking-wider text-gray-500 mb-4 flex items-center gap-2">
                    <Stamp size={14} />
                    {t('checkout.member.stamp_progress')}
                  </h4>
                  <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                    {stampProgress.map((sp) => {
                      const alreadyRedeemed = order.stamp_redemptions?.some(
                        (r) => r.stamp_activity_id === sp.stamp_activity_id
                      );
                      const orderBonus = order.items
                        .filter((item) => !item.is_comped && sp.stamp_targets.some((st) =>
                          st.target_type === 'PRODUCT' ? st.target_id === item.id
                          : st.target_type === 'CATEGORY' ? st.target_id === item.category_id
                          : false
                        ))
                        .reduce((sum, item) => sum + item.quantity, 0);
                      const effectiveStamps = sp.current_stamps + orderBonus;
                      const canRedeem = effectiveStamps >= sp.stamps_required && !alreadyRedeemed;
                      const progressPercent = Math.min(100, Math.round((effectiveStamps / sp.stamps_required) * 100));

                      return (
                        <div
                          key={sp.stamp_activity_id}
                          className={`relative rounded-2xl p-5 transition-all ${
                            alreadyRedeemed
                              ? 'bg-violet-50 border-2 border-violet-200'
                              : canRedeem
                                ? 'bg-violet-500 text-white shadow-lg shadow-violet-500/20 cursor-pointer hover:shadow-xl hover:scale-[1.01]'
                                : 'bg-white border border-gray-200 shadow-sm'
                          }`}
                          onClick={() => { if (canRedeem) setStampRedeemActivity(sp); }}
                        >
                          {/* Cancel button for redeemed */}
                          {alreadyRedeemed && (
                            <button
                              onClick={(e) => { e.stopPropagation(); handleCancelStampRedemption(sp.stamp_activity_id); }}
                              disabled={isProcessing}
                              className="absolute top-3 right-3 p-1.5 rounded-full bg-violet-200 hover:bg-violet-300 text-violet-700 transition-colors disabled:opacity-50"
                              title={t('checkout.stamp.cancel')}
                            >
                              <X size={14} />
                            </button>
                          )}

                          <div className="flex items-center gap-3 mb-3">
                            <Stamp size={20} className={alreadyRedeemed ? 'text-violet-500' : canRedeem ? '' : 'text-gray-400'} />
                            <span className={`font-bold text-lg ${alreadyRedeemed ? 'text-violet-700' : canRedeem ? '' : 'text-gray-800'}`}>
                              {sp.stamp_activity_name}
                            </span>
                          </div>

                          {/* Progress */}
                          <div className="flex items-baseline gap-2 mb-2">
                            <span className={`text-3xl font-black tabular-nums ${alreadyRedeemed ? 'text-violet-600' : canRedeem ? '' : 'text-gray-800'}`}>
                              {effectiveStamps}
                            </span>
                            <span className={`text-sm ${alreadyRedeemed ? 'text-violet-400' : canRedeem ? 'text-white/70' : 'text-gray-400'}`}>
                              / {sp.stamps_required}
                            </span>
                            {orderBonus > 0 && (
                              <span className={`text-sm font-medium ${alreadyRedeemed ? 'text-violet-400' : canRedeem ? 'text-white/70' : 'text-gray-400'}`}>
                                (+{orderBonus})
                              </span>
                            )}
                          </div>

                          {/* Progress bar */}
                          <div className={`w-full h-2 rounded-full overflow-hidden ${alreadyRedeemed ? 'bg-violet-200' : canRedeem ? 'bg-white/20' : 'bg-gray-100'}`}>
                            <div
                              className={`h-full rounded-full transition-all ${alreadyRedeemed ? 'bg-violet-400' : canRedeem ? 'bg-white' : 'bg-violet-400'}`}
                              style={{ width: `${progressPercent}%` }}
                            />
                          </div>

                          {/* Status */}
                          {alreadyRedeemed ? (
                            <div className="mt-3 text-sm font-medium text-violet-500">
                              {t('checkout.stamp.already_redeemed')}
                            </div>
                          ) : canRedeem ? (
                            <div className="mt-3 text-sm font-medium text-white/80">
                              {t('checkout.stamp.redeem')}
                            </div>
                          ) : (
                            <div className="mt-3 text-sm text-gray-400">
                              {sp.stamps_required - effectiveStamps} {t('checkout.member.stamps_remaining')}
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}

              {/* Empty state */}
              {discountRules.length === 0 && stampProgress.length === 0 && (
                <div className="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
                  <Crown size={48} className="opacity-30" />
                  <p className="text-lg font-medium">{t('checkout.member.no_rules')}</p>
                </div>
              )}
            </div>
          </div>

          {/* Right: Member Info Panel */}
          <div className="w-[400px] flex flex-col bg-white border-l border-gray-200 shadow-xl z-20">
            {/* Member Header */}
            <div className="p-6 bg-primary-50 border-b border-primary-100">
              <div className="text-2xl font-bold text-gray-800">{order.member_name}</div>
              {order.marketing_group_name && (
                <div className="mt-2 inline-flex items-center gap-1.5 text-sm bg-violet-100 text-violet-700 px-3 py-1 rounded-full font-medium">
                  <Crown size={14} />
                  {order.marketing_group_name}
                </div>
              )}
            </div>

            {/* Stats */}
            <div className="flex-1 overflow-y-auto p-6 space-y-6">
              {/* MG Discount on this order */}
              {order.mg_discount_amount > 0 && (
                <div className="p-4 bg-red-50 rounded-xl border border-red-100">
                  <div className="text-sm font-medium text-gray-500 mb-1">{t('checkout.member.order_mg_discount')}</div>
                  <div className="text-2xl font-bold text-red-500">-{formatCurrency(order.mg_discount_amount)}</div>
                </div>
              )}

              {/* Placeholder for future stats */}
              <div className="p-4 bg-gray-50 rounded-xl border border-gray-100 space-y-3">
                <div className="text-sm font-medium text-gray-500">{t('checkout.member.points_balance')}</div>
                <div className="text-2xl font-bold text-gray-800 tabular-nums">--</div>
              </div>
            </div>

            {/* Unlink Button */}
            <div className="p-6 border-t border-gray-200">
              <button
                onClick={handleUnlink}
                disabled={isProcessing}
                className="w-full py-3.5 bg-gray-100 hover:bg-red-50 text-gray-500 hover:text-red-600 rounded-xl font-bold transition-colors flex items-center justify-center gap-2 disabled:opacity-50"
              >
                <UserX size={20} />
                {t('checkout.member.unlink')}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Stamp Redeem Modals */}
      {stampRedeemActivity && (() => {
        const spa = stampRedeemActivity;
        const isDesignated = spa.reward_strategy === 'DESIGNATED';
        const matchingItems = isDesignated
          ? getDesignatedMatchingItems(order.items, spa)
          : getMatchingItems(order.items, spa);
        const oBonus = order.items
          .filter((item) => !item.is_comped && spa.stamp_targets.some((st) =>
            st.target_type === 'PRODUCT' ? st.target_id === item.id
            : st.target_type === 'CATEGORY' ? st.target_id === item.category_id
            : false
          ))
          .reduce((sum, item) => sum + item.quantity, 0);
        const excess = (spa.current_stamps + oBonus) > spa.stamps_required;
        const eff = spa.current_stamps + oBonus;
        return (
          <StampRedeemModal
            isOpen
            activity={spa}
            matchingItems={matchingItems}
            hasExcess={excess}
            effectiveStamps={eff}
            orderBonus={oBonus}
            isProcessing={isProcessing}
            onMatchRedeem={() => handleMatchRedeem(spa)}
            onSelectRedeem={() => { setStampRedeemActivity(null); setRewardPickerActivity(spa); }}
            onDirectRedeem={() => handleDirectRedeem(spa.stamp_activity_id)}
            onClose={() => setStampRedeemActivity(null)}
          />
        );
      })()}

      {rewardPickerActivity && (
        <StampRewardPickerModal
          isOpen={!!rewardPickerActivity}
          activityName={rewardPickerActivity.stamp_activity_name}
          rewardTargets={rewardPickerActivity.reward_targets}
          rewardQuantity={rewardPickerActivity.reward_quantity}
          onSelect={(productId) => handleSelectionRedeem(rewardPickerActivity.stamp_activity_id, productId)}
          onClose={() => setRewardPickerActivity(null)}
        />
      )}
    </div>
  );
};

/** Scope icons */
const SCOPE_ICONS: Record<string, React.ElementType> = {
  GLOBAL: Globe,
  CATEGORY: Layers,
  TAG: Tag,
  PRODUCT: Package,
};

/** Discount rule card */
function DiscountRuleCard({ rule, targetName, t }: { rule: MgDiscountRule; targetName: string | null; t: (key: string) => string }) {
  const scopeLabel = rule.product_scope === 'GLOBAL' ? t('checkout.member.scope_global')
    : rule.product_scope === 'CATEGORY' ? t('checkout.member.scope_category')
    : rule.product_scope === 'TAG' ? t('checkout.member.scope_tag')
    : t('checkout.member.scope_product');

  const valueLabel = rule.adjustment_type === 'PERCENTAGE'
    ? `${rule.adjustment_value}%`
    : formatCurrency(rule.adjustment_value);

  const ScopeIcon = SCOPE_ICONS[rule.product_scope] || Globe;

  return (
    <div className="p-4 bg-white rounded-xl border border-gray-200 shadow-sm">
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 rounded-lg bg-red-50 flex items-center justify-center shrink-0">
          <Percent size={18} className="text-red-500" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="font-bold text-lg text-gray-800 truncate">{rule.name}</div>
        </div>
        <div className="text-xl font-black text-red-500 shrink-0">-{valueLabel}</div>
      </div>
      <div className="mt-2.5 flex items-center gap-2 text-xs text-gray-500 ml-[52px]">
        <span className="inline-flex items-center gap-1">
          <ScopeIcon size={12} />
          {scopeLabel}
          {targetName && <span className="font-medium text-gray-700"> - {targetName}</span>}
        </span>
      </div>
    </div>
  );
}
