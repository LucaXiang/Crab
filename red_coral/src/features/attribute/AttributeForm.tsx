import React from 'react';
import { X, Type, Printer, Settings2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useAttributeActions } from './store';
import type { Attribute } from '@/core/domain/types/api';
import { FormField, FormSection, CheckboxField, SubField, inputClass } from '@/shared/components/FormField';
import { useFormInitialization } from '@/hooks/useFormInitialization';
import { useFormSubmit } from '@/shared/hooks/useFormSubmit';

// Form state uses camelCase internally, converted to snake_case on submit
interface AttributeFormData {
  name: string;
  receiptName: string;
  isMultiSelect: boolean;
  maxSelections: number | null;
  displayOrder: number;
  isActive: boolean;
  showOnReceipt: boolean;
  showOnKitchenPrint: boolean;
  kitchenPrintName: string;
}

// Map Attribute (snake_case) to form data (camelCase)
const mapToFormData = (attr: Attribute | null): AttributeFormData => {
  if (!attr) {
    return {
      name: '',
      receiptName: '',
      isMultiSelect: false,
      maxSelections: null,
      displayOrder: 0,
      isActive: true,
      showOnReceipt: false,
      showOnKitchenPrint: false,
      kitchenPrintName: '',
    };
  }
  return {
    name: attr.name,
    receiptName: attr.receipt_name || '',
    isMultiSelect: attr.is_multi_select,
    maxSelections: attr.max_selections ?? null,
    displayOrder: attr.display_order ?? 0,
    isActive: attr.is_active ?? true,
    showOnReceipt: attr.show_on_receipt ?? false,
    showOnKitchenPrint: attr.show_on_kitchen_print ?? false,
    kitchenPrintName: attr.kitchen_print_name || '',
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
          return t('settings.attribute.form.name_required');
        }
        return null;
      },
      onCreate: async (data) => {
        await createAttribute({
          name: data.name.trim(),
          is_multi_select: data.isMultiSelect,
          max_selections: data.isMultiSelect ? data.maxSelections : null,
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
          is_multi_select: data.isMultiSelect,
          max_selections: data.isMultiSelect ? data.maxSelections : null,
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

  const handleFieldChange = <K extends keyof AttributeFormData>(field: K, value: AttributeFormData[K]) => {
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
              {editingAttribute
                ? t('settings.attribute.edit_attribute')
                : t('settings.attribute.add_attribute')
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
            <FormField label={t('settings.attribute.form.name')} required>
              <input
                value={formData.name}
                onChange={(e) => handleFieldChange('name', e.target.value)}
                placeholder={t('settings.attribute.form.name_placeholder')}
                className={inputClass}
                autoFocus
              />
            </FormField>

            <CheckboxField
              id="isMultiSelect"
              label={t('settings.attribute.form.is_multi_select')}
              description={t('settings.attribute.form.is_multi_select_desc')}
              checked={formData.isMultiSelect}
              onChange={(checked) => handleFieldChange('isMultiSelect', checked)}
            />

            <SubField show={formData.isMultiSelect}>
              <FormField label={t('settings.attribute.form.max_selections')}>
                <input
                  type="number"
                  min={0}
                  value={formData.maxSelections ?? ''}
                  onChange={(e) => {
                    const val = e.target.value;
                    handleFieldChange('maxSelections', val === '' ? null : parseInt(val) || null);
                  }}
                  placeholder={t('settings.attribute.form.max_selections_placeholder')}
                  className={inputClass}
                />
                <p className="mt-1 text-xs text-gray-500">{t('settings.attribute.form.max_selections_hint')}</p>
              </FormField>
            </SubField>
          </FormSection>

          {/* 打印设置 */}
          <FormSection title={t('settings.attribute.section.print')} icon={Printer}>
            <CheckboxField
              id="showOnKitchenPrint"
              label={t('settings.attribute.form.show_on_kitchen_print')}
              description={t('settings.attribute.form.show_on_kitchen_print_hint')}
              checked={formData.showOnKitchenPrint}
              onChange={(checked) => handleFieldChange('showOnKitchenPrint', checked)}
            />
            <SubField show={formData.showOnKitchenPrint}>
              <FormField label={t('settings.attribute.form.kitchen_print_name')}>
                <input
                  value={formData.kitchenPrintName}
                  onChange={(e) => handleFieldChange('kitchenPrintName', e.target.value)}
                  placeholder={t('settings.attribute.form.kitchen_print_name_placeholder')}
                  className={inputClass}
                />
              </FormField>
            </SubField>

            <CheckboxField
              id="showOnReceipt"
              label={t('settings.attribute.form.show_on_receipt')}
              description={t('settings.attribute.form.show_on_receipt_hint')}
              checked={formData.showOnReceipt}
              onChange={(checked) => handleFieldChange('showOnReceipt', checked)}
            />
            <SubField show={formData.showOnReceipt}>
              <FormField label={t('settings.attribute.form.receipt_name')}>
                <input
                  value={formData.receiptName}
                  onChange={(e) => handleFieldChange('receiptName', e.target.value)}
                  placeholder={t('settings.attribute.form.receipt_name_placeholder')}
                  className={inputClass}
                />
              </FormField>
            </SubField>
          </FormSection>

          {/* 高级设置 */}
          <FormSection title={t('settings.attribute.section.advanced')} icon={Settings2} defaultCollapsed>
            <FormField label={t('settings.attribute.form.sort')}>
              <input
                type="number"
                value={formData.displayOrder}
                onChange={(e) => handleFieldChange('displayOrder', parseInt(e.target.value) || 0)}
                placeholder={t('settings.form.placeholder.sort_order')}
                className={inputClass}
              />
            </FormField>

            {editingAttribute && (
              <CheckboxField
                id="isActive"
                label={t('common.status.active')}
                description={t('settings.attribute.form.status_hint')}
                checked={formData.isActive}
                onChange={(checked) => handleFieldChange('isActive', checked)}
              />
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
            {editingAttribute
              ? t('common.action.save')
              : t('common.action.create')
            }
          </button>
        </div>
      </div>
    </div>
  );
});
