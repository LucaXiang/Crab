import React from 'react';
import { Target, LayoutGrid, Tag, Package, MapPin } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { WizardState } from './index';
import { FormSection, FormField, selectClass } from '../../forms/FormField';
import { useCategoryStore, useTagStore, useProductStore, useZones } from '@/core/stores/resources';

interface Step3ScopeProps {
  state: WizardState;
  updateState: (updates: Partial<WizardState>) => void;
}

export const Step3Scope: React.FC<Step3ScopeProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  const categories = useCategoryStore((s) => s.items);
  const tags = useTagStore((s) => s.items);
  const products = useProductStore((s) => s.items);
  const zones = useZones();

  const scopeOptions = [
    { value: 'GLOBAL', label: t('settings.price_rule.scope.global'), icon: LayoutGrid, desc: t('settings.price_rule.wizard.scope_global_desc') },
    { value: 'CATEGORY', label: t('settings.price_rule.scope.category'), icon: LayoutGrid, desc: t('settings.price_rule.wizard.scope_category_desc') },
    { value: 'TAG', label: t('settings.price_rule.scope.tag'), icon: Tag, desc: t('settings.price_rule.wizard.scope_tag_desc') },
    { value: 'PRODUCT', label: t('settings.price_rule.scope.product'), icon: Package, desc: t('settings.price_rule.wizard.scope_product_desc') },
  ];

  const handleScopeChange = (scope: WizardState['product_scope']) => {
    updateState({ product_scope: scope, target: null });
  };

  const renderTargetSelector = () => {
    switch (state.product_scope) {
      case 'CATEGORY':
        return (
          <FormField label={t('settings.price_rule.wizard.select_category')} required>
            <select
              value={state.target || ''}
              onChange={(e) => updateState({ target: e.target.value || null })}
              className={selectClass}
            >
              <option value="">{t('common.hint.please_select')}</option>
              {categories.map((cat) => (
                <option key={cat.id} value={cat.id}>
                  {cat.name}
                </option>
              ))}
            </select>
          </FormField>
        );
      case 'TAG':
        return (
          <FormField label={t('settings.price_rule.wizard.select_tag')} required>
            <select
              value={state.target || ''}
              onChange={(e) => updateState({ target: e.target.value || null })}
              className={selectClass}
            >
              <option value="">{t('common.hint.please_select')}</option>
              {tags.map((tag) => (
                <option key={tag.id} value={tag.id}>
                  {tag.name}
                </option>
              ))}
            </select>
          </FormField>
        );
      case 'PRODUCT':
        return (
          <FormField label={t('settings.price_rule.wizard.select_product')} required>
            <select
              value={state.target || ''}
              onChange={(e) => updateState({ target: e.target.value || null })}
              className={selectClass}
            >
              <option value="">{t('common.hint.please_select')}</option>
              {products.map((prod) => (
                <option key={prod.id} value={prod.id}>
                  {prod.name}
                </option>
              ))}
            </select>
          </FormField>
        );
      default:
        return null;
    }
  };

  return (
    <FormSection title={t('settings.price_rule.wizard.step3_section')} icon={Target}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.price_rule.wizard.step3_desc')}
      </p>

      {/* Product Scope */}
      <div className="mb-6">
        <label className="block text-sm font-medium text-gray-700 mb-3">
          {t('settings.price_rule.wizard.product_scope')}
        </label>
        <div className="grid grid-cols-2 gap-3">
          {scopeOptions.map((option) => {
            const Icon = option.icon;
            const isSelected = state.product_scope === option.value;
            return (
              <button
                key={option.value}
                type="button"
                onClick={() => handleScopeChange(option.value as WizardState['product_scope'])}
                className={`flex items-start gap-3 p-4 rounded-xl border-2 text-left transition-all ${
                  isSelected
                    ? 'border-teal-500 bg-teal-50'
                    : 'border-gray-200 bg-white hover:border-gray-300'
                }`}
              >
                <div
                  className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0 ${
                    isSelected ? 'bg-teal-100 text-teal-600' : 'bg-gray-100 text-gray-400'
                  }`}
                >
                  <Icon size={20} />
                </div>
                <div>
                  <span className={`font-medium block ${isSelected ? 'text-teal-700' : 'text-gray-700'}`}>
                    {option.label}
                  </span>
                  <span className="text-xs text-gray-500">{option.desc}</span>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Target Selector */}
      {state.product_scope !== 'GLOBAL' && (
        <div className="mb-6">{renderTargetSelector()}</div>
      )}

      {/* Zone Scope */}
      <FormField label={t('settings.price_rule.wizard.zone_scope')} required>
        <div className="flex items-center gap-2 mb-2">
          <MapPin size={16} className="text-gray-400" />
          <span className="text-sm text-gray-600">{t('settings.price_rule.wizard.zone_scope_desc')}</span>
        </div>
        <select
          value={state.zone_scope}
          onChange={(e) => updateState({ zone_scope: e.target.value })}
          className={selectClass}
        >
          <option value="zone:all">{t('settings.price_rule.zone.all')}</option>
          <option value="zone:retail">{t('settings.price_rule.zone.retail')}</option>
          {zones.map((zone) => (
            <option key={zone.id} value={zone.id ?? ''}>
              {zone.name}
            </option>
          ))}
        </select>
      </FormField>
    </FormSection>
  );
};
