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
} from 'lucide-react';
import type { PriceRule, PriceRuleUpdate } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';
import { useZoneStore } from '@/features/zone/store';
import { useCategoryStore } from '@/features/category/store';
import { useTagStore } from '@/features/tag/store';
import { useProductStore } from '@/core/stores/resources';
import { TimeVisualization } from './TimeVisualization';
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

  const isDiscount = rule.rule_type === 'DISCOUNT';
  const ProductScopeIcon = PRODUCT_SCOPE_ICONS[rule.product_scope] || Globe;

  // Get zone display name
  const getZoneName = (zoneScope: string): string => {
    if (zoneScope === 'zone:all') return t('settings.price_rule.zone.all');
    if (zoneScope === 'zone:retail') return t('settings.price_rule.zone.retail');
    const zoneId = zoneScope.replace('zone:', '');
    const zone = zones.find(z => z.id === zoneId || z.id === `zone:${zoneId}`);
    return zone?.name || zoneId;
  };

  // Get zone icon
  const getZoneIcon = (zoneScope: string): React.ElementType => {
    if (zoneScope === 'zone:all') return Globe;
    if (zoneScope === 'zone:retail') return ShoppingCart;
    return Armchair;
  };

  // Get target display name
  const getTargetName = (): string | null => {
    if (!rule.target) return null;

    switch (rule.product_scope) {
      case 'CATEGORY': {
        const cat = categories.find(c => c.id === rule.target);
        return cat?.name || rule.target;
      }
      case 'TAG': {
        const tag = tags.find(t => t.id === rule.target);
        return tag?.name || rule.target;
      }
      case 'PRODUCT': {
        const product = products.find(p => p.id === rule.target);
        return product?.name || rule.target;
      }
      default:
        return null;
    }
  };

  // Format adjustment
  const formatAdjustment = (): string => {
    const sign = isDiscount ? '-' : '+';
    if (rule.adjustment_type === 'PERCENTAGE') {
      return `${sign}${rule.adjustment_value}%`;
    }
    return `${sign}€${rule.adjustment_value.toFixed(2)}`;
  };

  // Get stacking mode
  const getStackingLabel = (): string => {
    if (rule.is_exclusive) return t('settings.price_rule.stacking.exclusive');
    if (rule.is_stackable) return t('settings.price_rule.stacking.stackable');
    return t('settings.price_rule.stacking.non_stackable');
  };

  // Start editing
  const handleStartEdit = () => {
    setEditData({
      display_name: rule.display_name,
      is_active: rule.is_active,
      adjustment_value: rule.adjustment_value,
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

  const ZoneIcon = getZoneIcon(rule.zone_scope);
  const targetName = getTargetName();

  return (
    <div className="flex-1 bg-white overflow-y-auto">
      <div className="max-w-2xl mx-auto p-6 space-y-6">
        {/* Header */}
        <div className="flex items-start justify-between">
          <div>
            {isEditing ? (
              <input
                type="text"
                value={editData.display_name ?? rule.display_name}
                onChange={e => setEditData({ ...editData, display_name: e.target.value })}
                className="text-2xl font-bold text-gray-900 border-b-2 border-blue-500 bg-transparent outline-none pb-1 w-full"
              />
            ) : (
              <h2 className="text-2xl font-bold text-gray-900">{rule.display_name}</h2>
            )}
            <p className="text-sm text-gray-400 mt-1">{rule.name}</p>
          </div>

          <div className="flex items-center gap-2">
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
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors flex items-center gap-2 disabled:opacity-50"
                >
                  <Save size={16} />
                  {t('common.action.save')}
                </button>
              </>
            ) : (
              <>
                <button
                  onClick={handleStartEdit}
                  className="p-2 text-gray-500 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
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
            <div>
              <label className="text-xs text-gray-500 mb-1 block">
                {t('settings.price_rule.column.type')}
              </label>
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
            </div>
            <div>
              <label className="text-xs text-gray-500 mb-1 block">
                {t('settings.price_rule.column.value')}
              </label>
              {isEditing ? (
                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    value={editData.adjustment_value ?? rule.adjustment_value}
                    onChange={e =>
                      setEditData({ ...editData, adjustment_value: parseFloat(e.target.value) || 0 })
                    }
                    className="w-24 px-3 py-1.5 border border-gray-300 rounded-lg text-sm"
                    step={rule.adjustment_type === 'PERCENTAGE' ? '1' : '0.01'}
                  />
                  <span className="text-gray-500">
                    {rule.adjustment_type === 'PERCENTAGE' ? '%' : '€'}
                  </span>
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
              <div className="flex items-center gap-2 text-sm">
                <ProductScopeIcon size={16} className="text-gray-400" />
                <span className="text-gray-600">
                  {t('settings.price_rule.product_scope_label')}:
                </span>
                <span className="font-medium text-gray-900">
                  {t(`settings.price_rule.scope.${rule.product_scope.toLowerCase()}`)}
                  {targetName && ` - ${targetName}`}
                </span>
              </div>
              <div className="flex items-center gap-2 text-sm">
                <ZoneIcon size={16} className="text-gray-400" />
                <span className="text-gray-600">
                  {t('settings.price_rule.zone_scope_label')}:
                </span>
                <span className="font-medium text-gray-900">{getZoneName(rule.zone_scope)}</span>
              </div>
            </div>
          </div>

          {/* Behavior */}
          <div>
            <label className="text-xs text-gray-500 mb-2 block">
              {t('settings.price_rule.behavior_section')}
            </label>
            <div className="flex items-center gap-4 text-sm">
              <div className="flex items-center gap-2">
                <span className="text-gray-600">{t('settings.price_rule.stacking_label')}:</span>
                <span
                  className={`font-medium ${
                    rule.is_exclusive
                      ? 'text-red-600'
                      : rule.is_stackable
                        ? 'text-green-600'
                        : 'text-gray-600'
                  }`}
                >
                  {getStackingLabel()}
                </span>
              </div>
              <span className="text-gray-300">|</span>
              <div className="flex items-center gap-2">
                <span className="text-gray-600">{t('settings.price_rule.status_label')}:</span>
                {isEditing ? (
                  <button
                    onClick={() => setEditData({ ...editData, is_active: !editData.is_active })}
                    className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                      editData.is_active ?? rule.is_active
                        ? 'bg-green-100 text-green-700'
                        : 'bg-gray-100 text-gray-500'
                    }`}
                  >
                    {(editData.is_active ?? rule.is_active)
                      ? t('common.status.enabled')
                      : t('common.status.disabled')}
                  </button>
                ) : (
                  <span
                    className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                      rule.is_active
                        ? 'bg-green-100 text-green-700'
                        : 'bg-gray-100 text-gray-500'
                    }`}
                  >
                    {rule.is_active ? t('common.status.enabled') : t('common.status.disabled')}
                  </span>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Time Visualization */}
        <TimeVisualization rule={rule} />
      </div>
    </div>
  );
};
