import React from 'react';
import { X } from 'lucide-react';
import { useI18n } from '../../../hooks/useI18n';
import { useOptionActions, attributeHelpers } from '@/core/stores/resources';
import type { AttributeOption } from '@/core/domain/types/api';
import { FormField, inputClass } from './FormField';
import { useFormInitialization } from '../../../hooks/useFormInitialization';
import { usePriceInput } from '../../../hooks/usePriceInput';
import { useFormSubmit } from '../../../hooks/useFormSubmit';

// Extended option type with index for UI (matches store type)
interface AttributeOptionWithIndex extends AttributeOption {
  index: number;
  attributeId: string;
}

// Form state uses camelCase internally
interface OptionFormData {
  name: string;
  receiptName: string;
  valueCode: string;
  priceModifier: number;
  isDefault: boolean;
  displayOrder: number;
  isActive: boolean;
}

// Map AttributeOption (snake_case) to form data (camelCase)
const mapToFormData = (opt: AttributeOptionWithIndex | null): OptionFormData => {
  if (!opt) {
    return {
      name: '',
      receiptName: '',
      valueCode: '',
      priceModifier: 0,
      isDefault: false,
      displayOrder: 0,
      isActive: true,
    };
  }
  return {
    name: opt.name,
    receiptName: opt.receipt_name || '',
    valueCode: opt.value_code || '',
    priceModifier: opt.price_modifier,
    isDefault: opt.is_default,
    displayOrder: opt.display_order,
    isActive: opt.is_active,
  };
};

interface OptionFormProps {
  isOpen: boolean;
  onClose: () => void;
  attributeId: string;
  editingOption: AttributeOptionWithIndex | null;
}

export const OptionForm: React.FC<OptionFormProps> = React.memo(({
  isOpen,
  onClose,
  attributeId,
  editingOption,
}) => {
  const { t } = useI18n();
  const { createOption, updateOption } = useOptionActions();

  // Use stable helpers directly
  const getAttributeById = attributeHelpers.getAttributeById;
  const getOptionsByAttributeId = attributeHelpers.getOptionsByAttributeId;

  // Use form initialization hook with mapped data
  const [formData, setFormData] = useFormInitialization<OptionFormData>(
    editingOption ? mapToFormData(editingOption) : null,
    mapToFormData(null),
    [isOpen]
  );

  // Use price input hook (allows negative values for price modifiers)
  const { priceInput, handlePriceChange, commitPrice, handlePriceKeyDown } = usePriceInput(
    formData.priceModifier || 0,
    {
      allowNegative: true,
      onCommit: (value) => setFormData(prev => ({ ...prev, priceModifier: value }))
    }
  );

  // Use form submit hook
  const { handleSubmit } = useFormSubmit(
    editingOption,
    formData,
    {
      validationRules: (data) => {
        if (!data.name.trim()) {
          return t('settings.attribute.option.form.nameRequired');
        }
        return null;
      },
      onCreate: async (data) => {
        // Ensure price is committed before submit
        commitPrice();

        // Logic to ensure only one default option for Single Choice attributes
        if (data.isDefault) {
          const attribute = getAttributeById(attributeId);
          if (attribute && (attribute.attr_type === 'SINGLE_REQUIRED' || attribute.attr_type === 'SINGLE_OPTIONAL')) {
            const existingOptions = getOptionsByAttributeId(attributeId);
            const otherDefaults = existingOptions.filter(opt => opt.is_default);

            if (otherDefaults.length > 0) {
              await Promise.all(otherDefaults.map(other =>
                updateOption({ attributeId, index: other.index, is_default: false })
              ));
            }
          }
        }

        await createOption({
          attributeId,
          name: data.name.trim(),
          receipt_name: data.receiptName?.trim() || undefined,
          value_code: data.valueCode?.trim() || undefined,
          price_modifier: data.priceModifier,
          is_default: data.isDefault,
          display_order: data.displayOrder,
        });
      },
      onUpdate: async (data) => {
        // Ensure price is committed before submit
        commitPrice();

        // Logic to ensure only one default option for Single Choice attributes
        if (data.isDefault) {
          const attribute = getAttributeById(attributeId);
          if (attribute && (attribute.attr_type === 'SINGLE_REQUIRED' || attribute.attr_type === 'SINGLE_OPTIONAL')) {
            const existingOptions = getOptionsByAttributeId(attributeId);
            const otherDefaults = existingOptions.filter(opt =>
              opt.is_default && opt.index !== editingOption!.index
            );

            if (otherDefaults.length > 0) {
              await Promise.all(otherDefaults.map(other =>
                updateOption({ attributeId, index: other.index, is_default: false })
              ));
            }
          }
        }

        await updateOption({
          attributeId,
          index: editingOption!.index,
          name: data.name.trim(),
          receipt_name: data.receiptName?.trim() || undefined,
          value_code: data.valueCode?.trim() || undefined,
          price_modifier: data.priceModifier,
          is_default: data.isDefault,
          display_order: data.displayOrder,
          is_active: data.isActive,
        });
      },
      onSuccess: onClose,
    }
  );

  const handleFieldChange = (field: string, value: any) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={onClose}
    >
      <div
        className="bg-white rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-gray-100 bg-linear-to-r from-teal-50 to-white">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-gray-900">
              {editingOption
                ? (t('settings.attribute.option.editOption'))
                : (t('settings.attribute.option.addOption'))
              }
            </h2>
            <button
              onClick={onClose}
              className="p-2 hover:bg-gray-100 rounded-xl transition-colors"
            >
              <X size={18} className="text-gray-500" />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="p-6">
          <div className="space-y-4">
            <FormField label={t('settings.attribute.option.form.name')} required>
              <input
                value={formData.name}
                onChange={(e) => handleFieldChange('name', e.target.value)}
                placeholder={t('settings.attribute.option.form.namePlaceholder')}
                className={inputClass}
                autoFocus
              />
            </FormField>

            <FormField label={t('settings.attribute.option.form.receiptName')}>
              <input
                value={formData.receiptName}
                onChange={(e) => handleFieldChange('receiptName', e.target.value)}
                placeholder={t('settings.attribute.option.form.receiptNamePlaceholder')}
                className={inputClass}
              />
            </FormField>

            <FormField label={t('settings.attribute.option.form.valueCode')}>
              <input
                value={formData.valueCode}
                onChange={(e) => handleFieldChange('valueCode', e.target.value)}
                placeholder={t('settings.attribute.option.form.valueCodePlaceholder')}
                className={inputClass}
              />
            </FormField>

            <FormField label={t('settings.attribute.option.form.price')}>
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 text-sm">€</span>
                <input
                  type="text"
                  inputMode="decimal"
                  value={priceInput}
                  onChange={handlePriceChange}
                  onBlur={commitPrice}
                  onFocus={(e) => e.currentTarget.select()}
                  onKeyDown={handlePriceKeyDown}
                  placeholder={t('settings.form.placeholder.price')}
                  className={`${inputClass} pl-7`}
                />
              </div>
              <p className="mt-1 text-xs text-gray-500">
                {t('settings.attribute.option.form.priceHint')}
              </p>
            </FormField>

            <div className="grid grid-cols-2 gap-4">
              <FormField label={t('settings.attribute.option.form.sort')}>
                <input
                  type="number"
                  value={formData.displayOrder}
                  onChange={(e) => handleFieldChange('displayOrder', parseInt(e.target.value) || 0)}
                  placeholder={t('settings.form.placeholder.sortOrder')}
                  className={inputClass}
                />
              </FormField>

              <FormField label={t('settings.attribute.option.form.default')}>
                <label className="flex items-center gap-2 cursor-pointer h-full">
                  <input
                    type="checkbox"
                    checked={formData.isDefault}
                    onChange={(e) => handleFieldChange('isDefault', e.target.checked)}
                    className="w-4 h-4 text-teal-600 bg-gray-100 border-gray-300 rounded focus:ring-teal-500"
                  />
                  <span className="text-sm text-gray-700">
                    {t('settings.attribute.option.form.setAsDefault')}
                  </span>
                </label>
              </FormField>
            </div>

            {editingOption && (
              <FormField label={t('settings.attribute.option.form.status')}>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.isActive}
                    onChange={(e) => handleFieldChange('isActive', e.target.checked)}
                    className="w-4 h-4 text-teal-600 bg-gray-100 border-gray-300 rounded focus:ring-teal-500"
                  />
                  <span className="text-sm text-gray-700">
                    {t('common.status.active')}
                  </span>
                </label>
              </FormField>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-100 bg-gray-50/50 flex justify-end gap-3">
          <button
            onClick={onClose}
            className="px-5 py-2.5 bg-white border border-gray-200 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-50 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={handleSubmit}
            disabled={!formData.name.trim()}
            className="px-5 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-semibold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20 disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none disabled:hover:bg-teal-600"
          >
            {editingOption
              ? (t('common.action.save'))
              : (t('common.action.create'))
            }
          </button>
        </div>
      </div>
    </div>
  );
});
