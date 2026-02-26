import { useState, useEffect, useCallback } from 'react';
import type { PriceRule, PriceRuleUpdate, ProductScope, RuleType, AdjustmentType } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';
import { useZoneStore } from '@/features/zone/store';
import { useCategoryStore } from '@/features/category/store';
import { useTagStore } from '@/features/tag/store';
import { useProductStore } from '@/core/stores/resources';
import { createTauriClient } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { Globe, ShoppingCart, Armchair } from 'lucide-react';

const getApi = () => createTauriClient();

export function useRuleEditor(rule: PriceRule, onRuleUpdated: () => void) {
  const { t } = useI18n();
  const zones = useZoneStore(state => state.items);
  const categories = useCategoryStore(state => state.items);
  const tags = useTagStore(state => state.items);
  const products = useProductStore(state => state.items);

  const [isEditing, setIsEditing] = useState(false);
  const [editData, setEditData] = useState<Partial<PriceRuleUpdate>>({});
  const [saving, setSaving] = useState(false);

  const [showZonePicker, setShowZonePicker] = useState(false);
  const [showTargetPicker, setShowTargetPicker] = useState(false);
  const [showTimeEditor, setShowTimeEditor] = useState(false);

  useEffect(() => {
    setIsEditing(false);
    setEditData({});
  }, [rule.id]);

  // Get current values (from editData if editing, otherwise from rule)
  const current = {
    ruleType: ((isEditing ? editData.rule_type : undefined) ?? rule.rule_type) as RuleType,
    adjustmentType: ((isEditing ? editData.adjustment_type : undefined) ?? rule.adjustment_type) as AdjustmentType,
    adjustmentValue: (isEditing ? editData.adjustment_value : undefined) ?? rule.adjustment_value,
    productScope: ((isEditing ? editData.product_scope : undefined) ?? rule.product_scope) as ProductScope,
    targetId: (isEditing ? editData.target_id : undefined) ?? rule.target_id,
    zoneScope: (isEditing ? editData.zone_scope : undefined) ?? rule.zone_scope,
    isStackable: (isEditing ? editData.is_stackable : undefined) ?? rule.is_stackable,
    isExclusive: (isEditing ? editData.is_exclusive : undefined) ?? rule.is_exclusive,
    isActive: (isEditing ? editData.is_active : undefined) ?? rule.is_active,
    activeDays: (isEditing ? editData.active_days : undefined) ?? rule.active_days,
    activeStartTime: (isEditing ? editData.active_start_time : undefined) ?? rule.active_start_time,
    activeEndTime: (isEditing ? editData.active_end_time : undefined) ?? rule.active_end_time,
    validFrom: (isEditing ? editData.valid_from : undefined) ?? rule.valid_from,
    validUntil: (isEditing ? editData.valid_until : undefined) ?? rule.valid_until,
    name: (isEditing ? editData.name : undefined) ?? rule.name,
  };

  const isDiscount = current.ruleType === 'DISCOUNT';

  const getZoneName = useCallback((zoneScope: string): string => {
    if (zoneScope === 'all') return t('settings.price_rule.zone.all');
    if (zoneScope === 'retail') return t('settings.price_rule.zone.retail');
    const zone = zones.find(z => String(z.id) === zoneScope);
    return zone?.name || zoneScope;
  }, [zones, t]);

  const getZoneIcon = useCallback((zoneScope: string): React.ElementType => {
    if (zoneScope === 'all') return Globe;
    if (zoneScope === 'retail') return ShoppingCart;
    return Armchair;
  }, []);

  const getTargetName = useCallback((scope: ProductScope, targetId: number | null | undefined): string | null => {
    if (targetId == null) return null;
    switch (scope) {
      case 'CATEGORY': return categories.find(c => c.id === targetId)?.name || String(targetId);
      case 'TAG': return tags.find(t => t.id === targetId)?.name || String(targetId);
      case 'PRODUCT': return products.find(p => p.id === targetId)?.name || String(targetId);
      default: return null;
    }
  }, [categories, tags, products]);

  const formatAdjustment = (): string => {
    const sign = isDiscount ? '-' : '+';
    if (current.adjustmentType === 'PERCENTAGE') return `${sign}${current.adjustmentValue}%`;
    return `${sign}€${current.adjustmentValue.toFixed(2)}`;
  };

  const getStackingMode = (): 'exclusive' | 'non_stackable' | 'stackable' => {
    if (current.isExclusive) return 'exclusive';
    if (current.isStackable) return 'stackable';
    return 'non_stackable';
  };

  const getStackingLabel = (): string => t(`settings.price_rule.stacking.${getStackingMode()}`);

  const formatTimeSummary = (): string => {
    const parts: string[] = [];
    if (current.activeDays && current.activeDays.length > 0 && current.activeDays.length < 7) {
      const dayKeys = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'] as const;
      parts.push(current.activeDays.map(d => t(`calendar.days.${dayKeys[d]}`)).join(''));
    }
    if (current.activeStartTime && current.activeEndTime) {
      parts.push(`${current.activeStartTime}-${current.activeEndTime}`);
    }
    if (current.validFrom || current.validUntil) {
      const from = current.validFrom ? new Date(current.validFrom).toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' }) : '';
      const until = current.validUntil ? new Date(current.validUntil).toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' }) : '';
      if (from && until) parts.push(`${from}~${until}`);
      else if (from) parts.push(`${from}起`);
      else if (until) parts.push(`至${until}`);
    }
    return parts.length > 0 ? parts.join(' ') : t('settings.price_rule.time.always');
  };

  const updateEditData = (updates: Partial<PriceRuleUpdate>) => {
    setEditData(prev => ({ ...prev, ...updates }));
  };

  const handleStartEdit = () => {
    setEditData({
      name: rule.name,
      rule_type: rule.rule_type,
      adjustment_type: rule.adjustment_type,
      adjustment_value: rule.adjustment_value,
      product_scope: rule.product_scope,
      target_id: rule.target_id ?? undefined,
      zone_scope: rule.zone_scope,
      is_stackable: rule.is_stackable,
      is_exclusive: rule.is_exclusive,
      is_active: rule.is_active,
      active_days: rule.active_days ?? undefined,
      active_start_time: rule.active_start_time ?? undefined,
      active_end_time: rule.active_end_time ?? undefined,
      valid_from: rule.valid_from ?? undefined,
      valid_until: rule.valid_until ?? undefined,
    });
    setIsEditing(true);
  };

  const handleCancelEdit = () => {
    setIsEditing(false);
    setEditData({});
  };

  const handleSave = async () => {
    if (!rule.id) return;
    setSaving(true);
    try {
      await getApi().updatePriceRule(rule.id, editData);
      toast.success(t('settings.price_rule.message.updated'));
      setIsEditing(false);
      setEditData({});
      onRuleUpdated();
    } catch (e) {
      logger.error('Failed to update price rule', e);
      toast.error(t('common.message.save_failed'));
    } finally {
      setSaving(false);
    }
  };

  const handleRuleTypeChange = (type: RuleType) => updateEditData({ rule_type: type });
  const handleAdjustmentTypeChange = (type: AdjustmentType) => updateEditData({ adjustment_type: type });

  const handleProductScopeChange = (scope: ProductScope) => {
    updateEditData({ product_scope: scope, target_id: undefined });
    if (scope !== 'GLOBAL') setShowTargetPicker(true);
  };

  const handleStackingModeChange = (mode: 'exclusive' | 'non_stackable' | 'stackable') => {
    switch (mode) {
      case 'exclusive': updateEditData({ is_exclusive: true, is_stackable: false }); break;
      case 'stackable': updateEditData({ is_exclusive: false, is_stackable: true }); break;
      case 'non_stackable': updateEditData({ is_exclusive: false, is_stackable: false }); break;
    }
  };

  return {
    isEditing,
    saving,
    current,
    isDiscount,
    showZonePicker,
    setShowZonePicker,
    showTargetPicker,
    setShowTargetPicker,
    showTimeEditor,
    setShowTimeEditor,
    getZoneName,
    getZoneIcon,
    getTargetName,
    formatAdjustment,
    getStackingMode,
    getStackingLabel,
    formatTimeSummary,
    updateEditData,
    handleStartEdit,
    handleCancelEdit,
    handleSave,
    handleRuleTypeChange,
    handleAdjustmentTypeChange,
    handleProductScopeChange,
    handleStackingModeChange,
  };
}
