import React from 'react';
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
  Settings,
  ChevronRight,
  Calendar,
} from 'lucide-react';
import type { PriceRule, ProductScope } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';
import { TimeVisualization } from './TimeVisualization';
import { ZonePicker } from './ZonePicker';
import { TargetPicker } from './TargetPicker';
import { TimeConditionEditor } from './TimeConditionEditor';
import { useRuleEditor } from './useRuleEditor';

interface RuleDetailPanelProps {
  rule: PriceRule | null;
  onRuleUpdated: () => void;
  onDeleteRule: (rule: PriceRule) => void;
}

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

  return <RuleDetailContent rule={rule} onRuleUpdated={onRuleUpdated} onDeleteRule={onDeleteRule} />;
};

// Separate component so the hook is only called when rule is non-null
const RuleDetailContent: React.FC<{
  rule: PriceRule;
  onRuleUpdated: () => void;
  onDeleteRule: (rule: PriceRule) => void;
}> = ({ rule, onRuleUpdated, onDeleteRule }) => {
  const { t } = useI18n();
  const editor = useRuleEditor(rule, onRuleUpdated);

  const ProductScopeIcon = PRODUCT_SCOPE_ICONS[editor.current.productScope] || Globe;
  const ZoneIcon = editor.getZoneIcon(editor.current.zoneScope);
  const targetName = editor.getTargetName(editor.current.productScope, editor.current.targetId);

  return (
    <div className="flex-1 bg-white overflow-y-auto">
      <div className="max-w-2xl mx-auto p-6 space-y-6">
        {/* Header */}
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            {editor.isEditing ? (
              <input
                type="text"
                value={editor.current.name}
                onChange={e => editor.updateEditData({ name: e.target.value })}
                className="text-2xl font-bold text-gray-900 border-b-2 border-teal-500 bg-transparent outline-none pb-1 w-full"
              />
            ) : (
              <h2 className="text-2xl font-bold text-gray-900">{rule.name}</h2>
            )}
          </div>

          <div className="flex items-center gap-2 ml-4">
            {editor.isEditing ? (
              <>
                <button
                  onClick={editor.handleCancelEdit}
                  className="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
                >
                  <X size={20} />
                </button>
                <button
                  onClick={editor.handleSave}
                  disabled={editor.saving}
                  className="px-4 py-2 bg-teal-600 text-white rounded-lg hover:bg-teal-700 transition-colors flex items-center gap-2 disabled:opacity-50"
                >
                  <Save size={16} />
                  {t('common.action.save')}
                </button>
              </>
            ) : (
              <>
                <button
                  onClick={editor.handleStartEdit}
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
              {editor.isEditing ? (
                <div className="flex gap-2">
                  <button
                    onClick={() => editor.handleRuleTypeChange('DISCOUNT')}
                    className={`flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      editor.current.ruleType === 'DISCOUNT'
                        ? 'bg-amber-100 text-amber-700 ring-2 ring-amber-400'
                        : 'bg-white text-gray-600 hover:bg-gray-100'
                    }`}
                  >
                    {t('settings.price_rule.type.discount')}
                  </button>
                  <button
                    onClick={() => editor.handleRuleTypeChange('SURCHARGE')}
                    className={`flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      editor.current.ruleType === 'SURCHARGE'
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
                    editor.isDiscount
                      ? 'bg-amber-100 text-amber-700'
                      : 'bg-purple-100 text-purple-700'
                  }`}
                >
                  <Percent size={14} />
                  {editor.isDiscount
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
              {editor.isEditing ? (
                <div className="space-y-2">
                  <div className="flex gap-2">
                    <button
                      onClick={() => editor.handleAdjustmentTypeChange('PERCENTAGE')}
                      className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                        editor.current.adjustmentType === 'PERCENTAGE'
                          ? 'bg-teal-500 text-white'
                          : 'bg-white text-gray-600 hover:bg-gray-100'
                      }`}
                    >
                      %
                    </button>
                    <button
                      onClick={() => editor.handleAdjustmentTypeChange('FIXED_AMOUNT')}
                      className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                        editor.current.adjustmentType === 'FIXED_AMOUNT'
                          ? 'bg-teal-500 text-white'
                          : 'bg-white text-gray-600 hover:bg-gray-100'
                      }`}
                    >
                      €
                    </button>
                  </div>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      value={editor.current.adjustmentValue}
                      onChange={e =>
                        editor.updateEditData({ adjustment_value: parseFloat(e.target.value) || 0 })
                      }
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm"
                      step={editor.current.adjustmentType === 'PERCENTAGE' ? '1' : '0.01'}
                      min={0}
                    />
                    <span className="text-gray-500 shrink-0">
                      {editor.current.adjustmentType === 'PERCENTAGE' ? '%' : '€'}
                    </span>
                  </div>
                </div>
              ) : (
                <span
                  className={`text-lg font-bold ${
                    editor.isDiscount ? 'text-amber-600' : 'text-purple-600'
                  }`}
                >
                  {editor.formatAdjustment()}
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
              {editor.isEditing ? (
                <div className="space-y-2">
                  <div className="text-xs text-gray-500 mb-1">
                    {t('settings.price_rule.product_scope_label')}
                  </div>
                  <div className="grid grid-cols-4 gap-2">
                    {(['GLOBAL', 'CATEGORY', 'TAG', 'PRODUCT'] as ProductScope[]).map(scope => {
                      const Icon = PRODUCT_SCOPE_ICONS[scope];
                      const isSelected = editor.current.productScope === scope;
                      return (
                        <button
                          key={scope}
                          onClick={() => editor.handleProductScopeChange(scope)}
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
                  {editor.current.productScope !== 'GLOBAL' && (
                    <button
                      onClick={() => editor.setShowTargetPicker(true)}
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
                    {t(`settings.price_rule.scope.${editor.current.productScope.toLowerCase()}`)}
                    {targetName && ` - ${targetName}`}
                  </span>
                </div>
              )}

              {/* Zone Scope */}
              {editor.isEditing ? (
                <button
                  onClick={() => editor.setShowZonePicker(true)}
                  className="w-full flex items-center justify-between p-3 bg-gray-50 rounded-xl hover:bg-gray-100 transition-colors"
                >
                  <div className="flex items-center gap-2 text-sm">
                    <ZoneIcon size={16} className="text-gray-400" />
                    <span className="text-gray-600">
                      {t('settings.price_rule.zone_scope_label')}:
                    </span>
                    <span className="font-medium text-gray-900">
                      {editor.getZoneName(editor.current.zoneScope)}
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
                  <span className="font-medium text-gray-900">{editor.getZoneName(editor.current.zoneScope)}</span>
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
              {editor.isEditing ? (
                <div className="space-y-2">
                  <div className="text-xs text-gray-500">
                    {t('settings.price_rule.stacking_label')}
                  </div>
                  <div className="grid grid-cols-3 gap-2">
                    {(['stackable', 'non_stackable', 'exclusive'] as const).map(mode => {
                      const isSelected = editor.getStackingMode() === mode;
                      return (
                        <button
                          key={mode}
                          onClick={() => editor.handleStackingModeChange(mode)}
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
                      editor.current.isExclusive
                        ? 'text-red-600'
                        : editor.current.isStackable
                          ? 'text-green-600'
                          : 'text-gray-600'
                    }`}
                  >
                    {editor.getStackingLabel()}
                  </span>
                </div>
              )}

              {/* Status */}
              <div className="flex items-center gap-2 text-sm">
                <span className="text-gray-600">{t('settings.price_rule.status_label')}:</span>
                {editor.isEditing ? (
                  <button
                    onClick={() => editor.updateEditData({ is_active: !editor.current.isActive })}
                    className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                      editor.current.isActive
                        ? 'bg-green-100 text-green-700'
                        : 'bg-gray-100 text-gray-500'
                    }`}
                  >
                    {editor.current.isActive
                      ? t('common.status.enabled')
                      : t('common.status.disabled')}
                  </button>
                ) : (
                  <span
                    className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                      editor.current.isActive
                        ? 'bg-green-100 text-green-700'
                        : 'bg-gray-100 text-gray-500'
                    }`}
                  >
                    {editor.current.isActive ? t('common.status.enabled') : t('common.status.disabled')}
                  </span>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Time Visualization or Editor */}
        {editor.isEditing ? (
          <button
            onClick={() => editor.setShowTimeEditor(true)}
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
            <div className="mt-2 text-sm text-gray-500">{editor.formatTimeSummary()}</div>
          </button>
        ) : (
          <TimeVisualization rule={rule} />
        )}
      </div>

      {/* Pickers */}
      <ZonePicker
        isOpen={editor.showZonePicker}
        selectedZone={editor.current.zoneScope}
        onSelect={zone => editor.updateEditData({ zone_scope: zone })}
        onClose={() => editor.setShowZonePicker(false)}
      />

      <TargetPicker
        isOpen={editor.showTargetPicker}
        productScope={editor.current.productScope}
        selectedTarget={editor.current.targetId ?? null}
        onSelect={target_id => editor.updateEditData({ target_id })}
        onClose={() => editor.setShowTargetPicker(false)}
      />

      <TimeConditionEditor
        isOpen={editor.showTimeEditor}
        value={{
          active_days: editor.current.activeDays ?? undefined,
          active_start_time: editor.current.activeStartTime ?? undefined,
          active_end_time: editor.current.activeEndTime ?? undefined,
          valid_from: editor.current.validFrom ?? undefined,
          valid_until: editor.current.validUntil ?? undefined,
        }}
        onChange={editor.updateEditData}
        onClose={() => editor.setShowTimeEditor(false)}
      />
    </div>
  );
};
