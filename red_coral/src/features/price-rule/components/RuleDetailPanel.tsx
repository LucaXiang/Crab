import React, { useState, useEffect } from 'react';
import {
  Percent,
  Pencil,
  Trash2,
  X,
  Save,
  Globe,
  Package,
  Tag,
  Layers,
  ShoppingCart,
  Armchair,
  Settings,
  ChevronRight,
  Calendar,
} from 'lucide-react';
import type { PriceRule, PriceRuleUpdate, ProductScope, RuleType, AdjustmentType } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';
import { useZoneStore } from '@/features/zone/store';
import { useCategoryStore } from '@/features/category/store';
import { useTagStore } from '@/features/tag/store';
import { useProductStore } from '@/core/stores/resources';
import { TimeVisualization } from './TimeVisualization';
import { ZonePicker } from './ZonePicker';
import { TargetPicker } from './TargetPicker';
import { TimeConditionEditor } from './TimeConditionEditor';
import { createTauriClient } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';

interface RuleDetailPanelProps {
  rule: PriceRule | null;
  onRuleUpdated: () => void;
  onDeleteRule: (rule: PriceRule) => void;
}

const getApi = () => createTauriClient();

// Scope icons
const PRODUCT_SCOPE_ICONS: Record<string, React.ElementType> = {
  GLOBAL: Globe,
  CATEGORY: Layers,
  TAG: Tag,
  PRODUCT: Package,
};

export const RuleDetailPanel: React.FC<RuleDetailPanelProps> = ({
  rule,
  onRuleUpdated,
  onDeleteRule,
}) => {
  const { t } = useI18n();
  const zones = useZoneStore(state => state.items);
  const categories = useCategoryStore(state => state.items);
  const tags = useTagStore(state => state.items);
  const products = useProductStore(state => state.items);

  const [isEditing, setIsEditing] = useState(false);
  const [editData, setEditData] = useState<Partial<PriceRuleUpdate>>({});
  const [saving, setSaving] = useState(false);

  // Picker states
  const [showZonePicker, setShowZonePicker] = useState(false);
  const [showTargetPicker, setShowTargetPicker] = useState(false);
  const [showTimeEditor, setShowTimeEditor] = useState(false);

  // Reset edit state when rule changes
  useEffect(() => {
    setIsEditing(false);
    setEditData({});
  }, [rule?.id]);

  if (!rule) {
    return (
      <div className="flex-1 flex items-center justify-center bg-white">
        <div className="text-center text-gray-400">
          <Settings size={48} className="mx-auto mb-3 opacity-50" />
          <p>{t('settings.price_rule.hint.select_rule')}</p>
        </div>
      </div>
    );
  }

  // Get current values (from editData if editing, otherwise from rule)
  const currentRuleType = (isEditing ? editData.rule_type : undefined) ?? rule.rule_type;
  const currentAdjustmentType = (isEditing ? editData.adjustment_type : undefined) ?? rule.adjustment_type;
  const currentAdjustmentValue = (isEditing ? editData.adjustment_value : undefined) ?? rule.adjustment_value;
  const currentProductScope = (isEditing ? editData.product_scope : undefined) ?? rule.product_scope;
  const currentTarget = (isEditing ? editData.target_id : undefined) ?? rule.target_id;
  const currentZoneScope = (isEditing ? editData.zone_scope : undefined) ?? rule.zone_scope;
  const currentIsStackable = (isEditing ? editData.is_stackable : undefined) ?? rule.is_stackable;
  const currentIsExclusive = (isEditing ? editData.is_exclusive : undefined) ?? rule.is_exclusive;
  const currentIsActive = (isEditing ? editData.is_active : undefined) ?? rule.is_active;
  const currentActiveDays = (isEditing ? editData.active_days : undefined) ?? rule.active_days;
  const currentActiveStartTime = (isEditing ? editData.active_start_time : undefined) ?? rule.active_start_time;
  const currentActiveEndTime = (isEditing ? editData.active_end_time : undefined) ?? rule.active_end_time;
  const currentValidFrom = (isEditing ? editData.valid_from : undefined) ?? rule.valid_from;
  const currentValidUntil = (isEditing ? editData.valid_until : undefined) ?? rule.valid_until;

  const isDiscount = currentRuleType === 'DISCOUNT';
  const ProductScopeIcon = PRODUCT_SCOPE_ICONS[currentProductScope] || Globe;

  // Get zone display name
  const getZoneName = (zoneScope: string): string => {
    if (zoneScope === 'all') return t('settings.price_rule.zone.all');
    if (zoneScope === 'retail') return t('settings.price_rule.zone.retail');
    const zone = zones.find(z => String(z.id) === zoneScope);
    return zone?.name || zoneScope;
  };

  // Get zone icon
  const getZoneIcon = (zoneScope: string): React.ElementType => {
    if (zoneScope === 'all') return Globe;
    if (zoneScope === 'retail') return ShoppingCart;
    return Armchair;
  };

  // Get target display name
  const getTargetName = (scope: ProductScope, targetId: number | null | undefined): string | null => {
    if (targetId == null) return null;

    switch (scope) {
      case 'CATEGORY': {
        const cat = categories.find(c => c.id === targetId);
        return cat?.name || String(targetId);
      }
      case 'TAG': {
        const tag = tags.find(t => t.id === targetId);
        return tag?.name || String(targetId);
      }
      case 'PRODUCT': {
        const product = products.find(p => p.id === targetId);
        return product?.name || String(targetId);
      }
      default:
        return null;
    }
  };

  // Format adjustment
  const formatAdjustment = (): string => {
    const sign = isDiscount ? '-' : '+';
    if (currentAdjustmentType === 'PERCENTAGE') {
      return `${sign}${currentAdjustmentValue}%`;
    }
    return `${sign}€${currentAdjustmentValue.toFixed(2)}`;
  };

  // Get stacking mode
  const getStackingMode = (): 'exclusive' | 'non_stackable' | 'stackable' => {
    if (currentIsExclusive) return 'exclusive';
    if (currentIsStackable) return 'stackable';
    return 'non_stackable';
  };

  const getStackingLabel = (): string => {
    return t(`settings.price_rule.stacking.${getStackingMode()}`);
  };

  // Format time summary for display
  const formatTimeSummary = (): string => {
    const parts: string[] = [];

    if (currentActiveDays && currentActiveDays.length > 0 && currentActiveDays.length < 7) {
      const dayNames = ['日', '一', '二', '三', '四', '五', '六'];
      const days = currentActiveDays.map(d => dayNames[d]).join('');
      parts.push(`周${days}`);
    }

    if (currentActiveStartTime && currentActiveEndTime) {
      parts.push(`${currentActiveStartTime}-${currentActiveEndTime}`);
    }

    if (currentValidFrom || currentValidUntil) {
      const from = currentValidFrom
        ? new Date(currentValidFrom).toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' })
        : '';
      const until = currentValidUntil
        ? new Date(currentValidUntil).toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' })
        : '';
      if (from && until) {
        parts.push(`${from}~${until}`);
      } else if (from) {
        parts.push(`${from}起`);
      } else if (until) {
        parts.push(`至${until}`);
      }
    }

    return parts.length > 0 ? parts.join(' ') : t('settings.price_rule.time.always');
  };

  // Start editing
  const handleStartEdit = () => {
    setEditData({
      display_name: rule.display_name,
      rule_type: rule.rule_type,
      adjustment_type: rule.adjustment_type,
      adjustment_value: rule.adjustment_value,
      product_scope: rule.product_scope,
      target_id: rule.target_id,
      zone_scope: rule.zone_scope,
      is_stackable: rule.is_stackable,
      is_exclusive: rule.is_exclusive,
      is_active: rule.is_active,
      active_days: rule.active_days,
      active_start_time: rule.active_start_time,
      active_end_time: rule.active_end_time,
      valid_from: rule.valid_from,
      valid_until: rule.valid_until,
    });
    setIsEditing(true);
  };

  // Cancel editing
  const handleCancelEdit = () => {
    setIsEditing(false);
    setEditData({});
  };

  // Save changes
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
      console.error(e);
      toast.error(t('common.message.save_failed'));
    } finally {
      setSaving(false);
    }
  };

  // Update edit data
  const updateEditData = (updates: Partial<PriceRuleUpdate>) => {
    setEditData(prev => ({ ...prev, ...updates }));
  };

  // Handle rule type change
  const handleRuleTypeChange = (type: RuleType) => {
    updateEditData({ rule_type: type });
  };

  // Handle adjustment type change
  const handleAdjustmentTypeChange = (type: AdjustmentType) => {
    updateEditData({ adjustment_type: type });
  };

  // Handle product scope change
  const handleProductScopeChange = (scope: ProductScope) => {
    if (scope === 'GLOBAL') {
      updateEditData({ product_scope: scope, target_id: undefined });
    } else {
      // Clear target when changing scope (different entity types)
      updateEditData({ product_scope: scope, target_id: undefined });
      // Open target picker if scope requires a target
      setShowTargetPicker(true);
    }
  };

  // Handle stacking mode change
  const handleStackingModeChange = (mode: 'exclusive' | 'non_stackable' | 'stackable') => {
    switch (mode) {
      case 'exclusive':
        updateEditData({ is_exclusive: true, is_stackable: false });
        break;
      case 'stackable':
        updateEditData({ is_exclusive: false, is_stackable: true });
        break;
      case 'non_stackable':
        updateEditData({ is_exclusive: false, is_stackable: false });
        break;
    }
  };

  const ZoneIcon = getZoneIcon(currentZoneScope);
  const targetName = getTargetName(currentProductScope, currentTarget);

  return (
    <div className="flex-1 bg-white overflow-y-auto">
      <div className="max-w-2xl mx-auto p-6 space-y-6">
        {/* Header */}
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            {isEditing ? (
              <input
                type="text"
                value={editData.display_name ?? rule.display_name}
                onChange={e => updateEditData({ display_name: e.target.value })}
                className="text-2xl font-bold text-gray-900 border-b-2 border-teal-500 bg-transparent outline-none pb-1 w-full"
              />
            ) : (
              <h2 className="text-2xl font-bold text-gray-900">{rule.display_name}</h2>
            )}
            <p className="text-sm text-gray-400 mt-1">{rule.name}</p>
          </div>

          <div className="flex items-center gap-2 ml-4">
            {isEditing ? (
              <>
                <button
                  onClick={handleCancelEdit}
                  className="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
                >
                  <X size={20} />
                </button>
                <button
                  onClick={handleSave}
                  disabled={saving}
                  className="px-4 py-2 bg-teal-600 text-white rounded-lg hover:bg-teal-700 transition-colors flex items-center gap-2 disabled:opacity-50"
                >
                  <Save size={16} />
                  {t('common.action.save')}
                </button>
              </>
            ) : (
              <>
                <button
                  onClick={handleStartEdit}
                  className="p-2 text-gray-500 hover:text-teal-600 hover:bg-teal-50 rounded-lg transition-colors"
                >
                  <Pencil size={20} />
                </button>
                <button
                  onClick={() => onDeleteRule(rule)}
                  className="p-2 text-gray-500 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                >
                  <Trash2 size={20} />
                </button>
              </>
            )}
          </div>
        </div>

        {/* Rule Info Card */}
        <div className="bg-gray-50 rounded-xl p-4 space-y-4">
          {/* Type and Adjustment */}
          <div className="grid grid-cols-2 gap-4">
            {/* Rule Type */}
            <div>
              <label className="text-xs text-gray-500 mb-1 block">
                {t('settings.price_rule.column.type')}
              </label>
              {isEditing ? (
                <div className="flex gap-2">
                  <button
                    onClick={() => handleRuleTypeChange('DISCOUNT')}
                    className={`flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      currentRuleType === 'DISCOUNT'
                        ? 'bg-amber-100 text-amber-700 ring-2 ring-amber-400'
                        : 'bg-white text-gray-600 hover:bg-gray-100'
                    }`}
                  >
                    {t('settings.price_rule.type.discount')}
                  </button>
                  <button
                    onClick={() => handleRuleTypeChange('SURCHARGE')}
                    className={`flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      currentRuleType === 'SURCHARGE'
                        ? 'bg-purple-100 text-purple-700 ring-2 ring-purple-400'
                        : 'bg-white text-gray-600 hover:bg-gray-100'
                    }`}
                  >
                    {t('settings.price_rule.type.surcharge')}
                  </button>
                </div>
              ) : (
                <span
                  className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium ${
                    isDiscount
                      ? 'bg-amber-100 text-amber-700'
                      : 'bg-purple-100 text-purple-700'
                  }`}
                >
                  <Percent size={14} />
                  {isDiscount
                    ? t('settings.price_rule.type.discount')
                    : t('settings.price_rule.type.surcharge')}
                </span>
              )}
            </div>

            {/* Adjustment Value */}
            <div>
              <label className="text-xs text-gray-500 mb-1 block">
                {t('settings.price_rule.column.value')}
              </label>
              {isEditing ? (
                <div className="space-y-2">
                  {/* Adjustment type toggle */}
                  <div className="flex gap-2">
                    <button
                      onClick={() => handleAdjustmentTypeChange('PERCENTAGE')}
                      className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                        currentAdjustmentType === 'PERCENTAGE'
                          ? 'bg-teal-500 text-white'
                          : 'bg-white text-gray-600 hover:bg-gray-100'
                      }`}
                    >
                      %
                    </button>
                    <button
                      onClick={() => handleAdjustmentTypeChange('FIXED_AMOUNT')}
                      className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                        currentAdjustmentType === 'FIXED_AMOUNT'
                          ? 'bg-teal-500 text-white'
                          : 'bg-white text-gray-600 hover:bg-gray-100'
                      }`}
                    >
                      €
                    </button>
                  </div>
                  {/* Value input */}
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      value={currentAdjustmentValue}
                      onChange={e =>
                        updateEditData({ adjustment_value: parseFloat(e.target.value) || 0 })
                      }
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm"
                      step={currentAdjustmentType === 'PERCENTAGE' ? '1' : '0.01'}
                      min={0}
                    />
                    <span className="text-gray-500 shrink-0">
                      {currentAdjustmentType === 'PERCENTAGE' ? '%' : '€'}
                    </span>
                  </div>
                </div>
              ) : (
                <span
                  className={`text-lg font-bold ${
                    isDiscount ? 'text-amber-600' : 'text-purple-600'
                  }`}
                >
                  {formatAdjustment()}
                </span>
              )}
            </div>
          </div>

          {/* Scope */}
          <div>
            <label className="text-xs text-gray-500 mb-2 block">
              {t('settings.price_rule.scope_section')}
            </label>
            <div className="bg-white rounded-lg p-3 space-y-2">
              {/* Product Scope */}
              {isEditing ? (
                <div className="space-y-2">
                  <div className="text-xs text-gray-500 mb-1">
                    {t('settings.price_rule.product_scope_label')}
                  </div>
                  <div className="grid grid-cols-4 gap-2">
                    {(['GLOBAL', 'CATEGORY', 'TAG', 'PRODUCT'] as ProductScope[]).map(scope => {
                      const Icon = PRODUCT_SCOPE_ICONS[scope];
                      const isSelected = currentProductScope === scope;
                      return (
                        <button
                          key={scope}
                          onClick={() => handleProductScopeChange(scope)}
                          className={`flex flex-col items-center gap-1 p-3 rounded-xl text-xs transition-colors ${
                            isSelected
                              ? 'bg-teal-50 text-teal-700 ring-2 ring-teal-400'
                              : 'bg-gray-50 text-gray-600 hover:bg-gray-100'
                          }`}
                        >
                          <Icon size={18} />
                          <span>{t(`settings.price_rule.scope.${scope.toLowerCase()}`)}</span>
                        </button>
                      );
                    })}
                  </div>
                  {/* Target selection button */}
                  {currentProductScope !== 'GLOBAL' && (
                    <button
                      onClick={() => setShowTargetPicker(true)}
                      className="w-full flex items-center justify-between p-3 bg-gray-50 rounded-xl hover:bg-gray-100 transition-colors"
                    >
                      <div className="flex items-center gap-2 text-sm">
                        <ProductScopeIcon size={16} className="text-gray-400" />
                        <span className="text-gray-600">
                          {targetName || t('settings.price_rule.edit.select_target')}
                        </span>
                      </div>
                      <ChevronRight size={16} className="text-gray-400" />
                    </button>
                  )}
                </div>
              ) : (
                <div className="flex items-center gap-2 text-sm">
                  <ProductScopeIcon size={16} className="text-gray-400" />
                  <span className="text-gray-600">
                    {t('settings.price_rule.product_scope_label')}:
                  </span>
                  <span className="font-medium text-gray-900">
                    {t(`settings.price_rule.scope.${currentProductScope.toLowerCase()}`)}
                    {targetName && ` - ${targetName}`}
                  </span>
                </div>
              )}

              {/* Zone Scope */}
              {isEditing ? (
                <button
                  onClick={() => setShowZonePicker(true)}
                  className="w-full flex items-center justify-between p-3 bg-gray-50 rounded-xl hover:bg-gray-100 transition-colors"
                >
                  <div className="flex items-center gap-2 text-sm">
                    <ZoneIcon size={16} className="text-gray-400" />
                    <span className="text-gray-600">
                      {t('settings.price_rule.zone_scope_label')}:
                    </span>
                    <span className="font-medium text-gray-900">
                      {getZoneName(currentZoneScope)}
                    </span>
                  </div>
                  <ChevronRight size={16} className="text-gray-400" />
                </button>
              ) : (
                <div className="flex items-center gap-2 text-sm">
                  <ZoneIcon size={16} className="text-gray-400" />
                  <span className="text-gray-600">
                    {t('settings.price_rule.zone_scope_label')}:
                  </span>
                  <span className="font-medium text-gray-900">{getZoneName(currentZoneScope)}</span>
                </div>
              )}
            </div>
          </div>

          {/* Behavior */}
          <div>
            <label className="text-xs text-gray-500 mb-2 block">
              {t('settings.price_rule.behavior_section')}
            </label>
            <div className="bg-white rounded-lg p-3 space-y-3">
              {/* Stacking Mode */}
              {isEditing ? (
                <div className="space-y-2">
                  <div className="text-xs text-gray-500">
                    {t('settings.price_rule.stacking_label')}
                  </div>
                  <div className="grid grid-cols-3 gap-2">
                    {(['stackable', 'non_stackable', 'exclusive'] as const).map(mode => {
                      const isSelected = getStackingMode() === mode;
                      return (
                        <button
                          key={mode}
                          onClick={() => handleStackingModeChange(mode)}
                          className={`px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
                            isSelected
                              ? mode === 'exclusive'
                                ? 'bg-red-100 text-red-700 ring-2 ring-red-400'
                                : mode === 'stackable'
                                  ? 'bg-green-100 text-green-700 ring-2 ring-green-400'
                                  : 'bg-gray-200 text-gray-700 ring-2 ring-gray-400'
                              : 'bg-gray-50 text-gray-600 hover:bg-gray-100'
                          }`}
                        >
                          {t(`settings.price_rule.stacking.${mode}`)}
                        </button>
                      );
                    })}
                  </div>
                </div>
              ) : (
                <div className="flex items-center gap-2 text-sm">
                  <span className="text-gray-600">{t('settings.price_rule.stacking_label')}:</span>
                  <span
                    className={`font-medium ${
                      currentIsExclusive
                        ? 'text-red-600'
                        : currentIsStackable
                          ? 'text-green-600'
                          : 'text-gray-600'
                    }`}
                  >
                    {getStackingLabel()}
                  </span>
                </div>
              )}

              {/* Status */}
              <div className="flex items-center gap-2 text-sm">
                <span className="text-gray-600">{t('settings.price_rule.status_label')}:</span>
                {isEditing ? (
                  <button
                    onClick={() => updateEditData({ is_active: !currentIsActive })}
                    className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                      currentIsActive
                        ? 'bg-green-100 text-green-700'
                        : 'bg-gray-100 text-gray-500'
                    }`}
                  >
                    {currentIsActive
                      ? t('common.status.enabled')
                      : t('common.status.disabled')}
                  </button>
                ) : (
                  <span
                    className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                      currentIsActive
                        ? 'bg-green-100 text-green-700'
                        : 'bg-gray-100 text-gray-500'
                    }`}
                  >
                    {currentIsActive ? t('common.status.enabled') : t('common.status.disabled')}
                  </span>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Time Visualization or Editor */}
        {isEditing ? (
          <button
            onClick={() => setShowTimeEditor(true)}
            className="w-full bg-gray-50 rounded-xl p-4 hover:bg-gray-100 transition-colors text-left"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Calendar size={16} className="text-gray-500" />
                <span className="text-sm font-medium text-gray-700">
                  {t('settings.price_rule.time_viz.title')}
                </span>
              </div>
              <ChevronRight size={16} className="text-gray-400" />
            </div>
            <div className="mt-2 text-sm text-gray-500">{formatTimeSummary()}</div>
          </button>
        ) : (
          <TimeVisualization rule={rule} />
        )}
      </div>

      {/* Pickers */}
      <ZonePicker
        isOpen={showZonePicker}
        selectedZone={currentZoneScope}
        onSelect={zone => updateEditData({ zone_scope: zone })}
        onClose={() => setShowZonePicker(false)}
      />

      <TargetPicker
        isOpen={showTargetPicker}
        productScope={currentProductScope}
        selectedTarget={currentTarget ?? null}
        onSelect={target_id => updateEditData({ target_id })}
        onClose={() => setShowTargetPicker(false)}
      />

      <TimeConditionEditor
        isOpen={showTimeEditor}
        value={{
          active_days: currentActiveDays,
          active_start_time: currentActiveStartTime,
          active_end_time: currentActiveEndTime,
          valid_from: currentValidFrom,
          valid_until: currentValidUntil,
        }}
        onChange={updateEditData}
        onClose={() => setShowTimeEditor(false)}
      />
    </div>
  );
};
