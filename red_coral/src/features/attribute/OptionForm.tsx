import React, { useMemo } from 'react';
import { X, Type, Printer, Settings2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useOptionActions } from './store';
import type { AttributeOption } from '@/core/domain/types/api';
import { FormField, FormSection, CheckboxField, inputClass } from '@/shared/components/FormField';
import { useFormInitialization } from '@/hooks/useFormInitialization';
import { usePriceInput } from '@/hooks/usePriceInput';
import { useFormSubmit } from '@/shared/hooks/useFormSubmit';

// Extended option type with index for UI (matches store type)
interface AttributeOptionWithIndex extends AttributeOption {
  index: number;
  attributeId: number;
}

// Form state uses camelCase internally
interface OptionFormData {
  name: string;
  receiptName: string;
  kitchenPrintName: string;
  priceModifier: number;
  displayOrder: number;
  enableQuantity: boolean;
  maxQuantity: number | null;
}

// Map AttributeOption (snake_case) to form data (camelCase)
const mapToFormData = (opt: AttributeOptionWithIndex | null): OptionFormData => {
  if (!opt) {
    return {
      name: '',
      receiptName: '',
      kitchenPrintName: '',
      priceModifier: 0,
      displayOrder: 0,
      enableQuantity: false,
      maxQuantity: null,
    };
  }
  return {
    name: opt.name,
    receiptName: opt.receipt_name || '',
    kitchenPrintName: opt.kitchen_print_name || '',
    priceModifier: opt.price_modifier,
    displayOrder: opt.display_order,
    enableQuantity: opt.enable_quantity ?? false,
    maxQuantity: opt.max_quantity ?? null,
  };
};

interface OptionFormProps {
  isOpen: boolean;
  onClose: () => void;
  attributeId: number;
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

  // Memoize the initial form data to prevent useEffect from re-running on every render
  // Include all editable fields in deps to ensure form updates when option data changes
  const initialFormData = useMemo(
    () => (editingOption ? mapToFormData(editingOption) : null),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [
      editingOption?.attributeId,
      editingOption?.index,
      editingOption?.name,
      editingOption?.price_modifier,
      editingOption?.enable_quantity,
      editingOption?.max_quantity,
    ]
  );

  // Use form initialization hook with mapped data
  const [formData, setFormData] = useFormInitialization<OptionFormData>(
    initialFormData,
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
          return t('settings.attribute.option.form.name_required');
        }
        return null;
      },
      onCreate: async (data) => {
        // Ensure price is committed before submit
        commitPrice();

        await createOption({
          attributeId,
          name: data.name.trim(),
          receipt_name: data.receiptName?.trim() || undefined,
          kitchen_print_name: data.kitchenPrintName?.trim() || undefined,
          price_modifier: data.priceModifier,
          display_order: data.displayOrder,
          enable_quantity: data.enableQuantity,
          max_quantity: data.enableQuantity ? data.maxQuantity : null,
        });
      },
      onUpdate: async (data) => {
        // Ensure price is committed before submit
        commitPrice();

        await updateOption({
          attributeId,
          index: editingOption!.index,
          name: data.name.trim(),
          receipt_name: data.receiptName?.trim() || undefined,
          kitchen_print_name: data.kitchenPrintName?.trim() || undefined,
          price_modifier: data.priceModifier,
          display_order: data.displayOrder,
          enable_quantity: data.enableQuantity,
          max_quantity: data.enableQuantity ? data.maxQuantity : null,
        });
      },
      onSuccess: onClose,
    }
  );

  const handleFieldChange = <K extends keyof OptionFormData>(field: K, value: OptionFormData[K]) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={onClose}
    >
      <div
        className="bg-gray-50 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-gray-200 bg-white">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-gray-900">
              {editingOption
                ? t('settings.attribute.option.edit_option')
                : t('settings.attribute.option.add_option')
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
        <div className="p-4 space-y-4 max-h-[70vh] overflow-y-auto">
          {/* 基本信息 */}
          <FormSection title={t('settings.attribute.section.basic')} icon={Type}>
            <FormField label={t('settings.attribute.option.form.name')} required>
              <input
                value={formData.name}
                onChange={(e) => handleFieldChange('name', e.target.value)}
                placeholder={t('settings.attribute.option.form.name_placeholder')}
                className={inputClass}
                autoFocus
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
                {t('settings.attribute.option.form.price_hint')}
              </p>
            </FormField>
          </FormSection>

          {/* 打印设置 */}
          <FormSection title={t('settings.attribute.section.print')} icon={Printer}>
            <FormField label={t('settings.attribute.option.form.receipt_name')}>
              <input
                value={formData.receiptName}
                onChange={(e) => handleFieldChange('receiptName', e.target.value)}
                placeholder={t('settings.attribute.option.form.receipt_name_placeholder')}
                className={inputClass}
              />
            </FormField>

            <FormField label={t('settings.attribute.option.form.kitchen_print_name')}>
              <input
                value={formData.kitchenPrintName}
                onChange={(e) => handleFieldChange('kitchenPrintName', e.target.value)}
                placeholder={t('settings.attribute.option.form.kitchen_print_name_placeholder')}
                className={inputClass}
              />
            </FormField>
          </FormSection>

          {/* 高级设置 */}
          <FormSection title={t('settings.attribute.section.advanced')} icon={Settings2} defaultCollapsed>
            <FormField label={t('settings.attribute.option.form.sort')}>
              <input
                type="number"
                value={formData.displayOrder}
                onChange={(e) => handleFieldChange('displayOrder', parseInt(e.target.value) || 0)}
                placeholder={t('settings.form.placeholder.sort_order')}
                className={inputClass}
              />
            </FormField>

            {/* 数量控制 */}
            <CheckboxField
              id="enableQuantity"
              label={t('settings.attribute.option.form.enable_quantity')}
              checked={formData.enableQuantity}
              onChange={(checked) => handleFieldChange('enableQuantity', checked)}
            />
            <p className="text-xs text-gray-500 -mt-2 mb-2">
              {t('settings.attribute.option.form.enable_quantity_hint')}
            </p>

            {formData.enableQuantity && (
              <FormField label={t('settings.attribute.option.form.max_quantity')}>
                <input
                  type="number"
                  min="1"
                  max="99"
                  value={formData.maxQuantity ?? ''}
                  onChange={(e) => {
                    const val = e.target.value ? parseInt(e.target.value, 10) : null;
                    handleFieldChange('maxQuantity', val);
                  }}
                  placeholder={t('settings.attribute.option.form.max_quantity_placeholder')}
                  className={inputClass}
                />
                <p className="mt-1 text-xs text-gray-500">
                  {t('settings.attribute.option.form.max_quantity_hint')}
                </p>
              </FormField>
            )}
          </FormSection>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-200 bg-white flex justify-end gap-3">
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
              ? t('common.action.save')
              : t('common.action.create')
            }
          </button>
        </div>
      </div>
    </div>
  );
});
