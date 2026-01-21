import React from 'react';
import { X } from 'lucide-react';
import { useI18n } from '../../../hooks/useI18n';
import { useAttributeActions } from '@/core/stores/resources';
import type { Attribute } from '@/core/domain/types/api';
import { FormField, inputClass } from './FormField';
import { useFormInitialization } from '../../../hooks/useFormInitialization';
import { useFormSubmit } from '../../../hooks/useFormSubmit';
import { SelectField } from '@/presentation/components/form/FormField/SelectField';

// Form state uses camelCase internally, converted to snake_case on submit
interface AttributeFormData {
  name: string;
  receiptName: string;
  isMultiSelect: boolean;
  displayOrder: number;
  isActive: boolean;
  showOnReceipt: boolean;
  showOnKitchenPrint: boolean;
  kitchenPrintName: string;
  scope: 'global' | 'inherited';
}

// Map Attribute (snake_case) to form data (camelCase)
const mapToFormData = (attr: Attribute | null): AttributeFormData => {
  if (!attr) {
    return {
      name: '',
      receiptName: '',
      isMultiSelect: false,
      displayOrder: 0,
      isActive: true,
      showOnReceipt: false,
      showOnKitchenPrint: false,
      kitchenPrintName: '',
      scope: 'inherited',
    };
  }
  return {
    name: attr.name,
    receiptName: attr.receipt_name || '',
    isMultiSelect: attr.is_multi_select,
    displayOrder: attr.display_order,
    isActive: attr.is_active,
    showOnReceipt: attr.show_on_receipt,
    showOnKitchenPrint: attr.show_on_kitchen_print,
    kitchenPrintName: attr.kitchen_print_name || '',
    scope: attr.scope,
  };
};

interface AttributeFormProps {
  isOpen: boolean;
  onClose: () => void;
  editingAttribute: Attribute | null;
}

export const AttributeForm: React.FC<AttributeFormProps> = React.memo(({
  isOpen,
  onClose,
  editingAttribute,
}) => {
  const { t } = useI18n();
  const { createAttribute, updateAttribute } = useAttributeActions();

  // Use form initialization hook with mapped data
  const [formData, setFormData] = useFormInitialization<AttributeFormData>(
    editingAttribute ? mapToFormData(editingAttribute) : null,
    mapToFormData(null),
    [isOpen]
  );

  // Use form submit hook
  const { handleSubmit } = useFormSubmit(
    editingAttribute,
    formData,
    {
      validationRules: (data) => {
        if (!data.name.trim()) {
          return t('settings.attribute.form.nameRequired');
        }
        return null;
      },
      onCreate: async (data) => {
        await createAttribute({
          name: data.name.trim(),
          scope: data.scope,
          display_order: data.displayOrder,
          show_on_receipt: data.showOnReceipt,
          receipt_name: data.receiptName?.trim() || undefined,
          show_on_kitchen_print: data.showOnKitchenPrint,
          kitchen_print_name: data.kitchenPrintName?.trim() || undefined,
        });
      },
      onUpdate: async (data) => {
        await updateAttribute({
          id: String(editingAttribute!.id),
          name: data.name.trim(),
          scope: data.scope,
          display_order: data.displayOrder,
          is_active: data.isActive,
          show_on_receipt: data.showOnReceipt,
          receipt_name: data.receiptName?.trim() || undefined,
          show_on_kitchen_print: data.showOnKitchenPrint,
          kitchen_print_name: data.kitchenPrintName?.trim() || undefined,
        });
      },
      onSuccess: onClose,
    }
  );

  const handleFieldChange = (field: string, value: any) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  if (!isOpen) return null;

  const SCOPE_OPTIONS = [
    { value: 'inherited', label: t('settings.attribute.scope.inherited') },
    { value: 'global', label: t('settings.attribute.scope.global') },
  ];

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
              {editingAttribute
                ? (t('settings.attribute.editAttribute'))
                : (t('settings.attribute.addAttribute'))
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
            <FormField label={t('settings.attribute.form.name')} required>
              <input
                value={formData.name}
                onChange={(e) => handleFieldChange('name', e.target.value)}
                placeholder={t('settings.attribute.form.namePlaceholder')}
                className={inputClass}
                autoFocus
              />
            </FormField>

            <SelectField
              label={t('settings.attribute.form.scope')}
              value={formData.scope}
              onChange={(value) => handleFieldChange('scope', value)}
              options={SCOPE_OPTIONS}
            />

            <div className="flex items-start space-x-3 py-2">
                <div className="flex items-center h-5">
                    <input
                        type="checkbox"
                        id="isMultiSelect"
                        checked={formData.isMultiSelect}
                        onChange={(e) => handleFieldChange('isMultiSelect', e.target.checked)}
                        className="w-4 h-4 text-teal-600 rounded border-gray-300 focus:ring-teal-500"
                    />
                </div>
                <label htmlFor="isMultiSelect" className="text-gray-700 cursor-pointer select-none">
                    <span className="font-medium block">{t('settings.attribute.form.isMultiSelect')}</span>
                    <span className="text-sm text-gray-500 block">
                        {t('settings.attribute.form.isMultiSelectDesc')}
                    </span>
                </label>
            </div>

            <FormField label={t('settings.attribute.form.receiptName')}>
              <input
                value={formData.receiptName}
                onChange={(e) => handleFieldChange('receiptName', e.target.value)}
                placeholder={t('settings.attribute.form.receiptNamePlaceholder')}
                className={inputClass}
              />
            </FormField>

            <FormField label={t('settings.attribute.form.sort')}>
              <input
                type="number"
                value={formData.displayOrder}
                onChange={(e) => handleFieldChange('displayOrder', parseInt(e.target.value) || 0)}
                placeholder={t('settings.form.placeholder.sortOrder')}
                className={inputClass}
              />
            </FormField>

            <FormField label={t('settings.attribute.form.showOnKitchenPrint')}>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={formData.showOnKitchenPrint}
                  onChange={(e) => handleFieldChange('showOnKitchenPrint', e.target.checked)}
                  className="w-4 h-4 text-teal-600 bg-gray-100 border-gray-300 rounded focus:ring-teal-500"
                />
                <span className="text-sm text-gray-700">
                  {t('settings.attribute.form.showOnKitchenPrintHint')}
                </span>
              </label>
            </FormField>

            {formData.showOnKitchenPrint && (
              <FormField label={t('settings.attribute.form.kitchenPrintName')}>
                <input
                  value={formData.kitchenPrintName}
                  onChange={(e) => handleFieldChange('kitchenPrintName', e.target.value)}
                  placeholder={t('settings.attribute.form.kitchenPrintNamePlaceholder')}
                  className={inputClass}
                />
              </FormField>
            )}

            <FormField label={t('settings.attribute.form.showOnReceipt')}>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={formData.showOnReceipt}
                  onChange={(e) => handleFieldChange('showOnReceipt', e.target.checked)}
                  className="w-4 h-4 text-teal-600 bg-gray-100 border-gray-300 rounded focus:ring-teal-500"
                />
                <span className="text-sm text-gray-700">
                  {t('settings.attribute.form.showOnReceiptHint')}
                </span>
              </label>
            </FormField>

            {editingAttribute && (
              <FormField label={t('settings.attribute.form.status')}>
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
            {editingAttribute
              ? (t('common.action.save'))
              : (t('common.action.create'))
            }
          </button>
        </div>
      </div>
    </div>
  );
});
