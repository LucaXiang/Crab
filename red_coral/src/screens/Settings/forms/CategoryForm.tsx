import React, { useState } from 'react';
import { FormField, inputClass } from './FormField';
import { Layers } from 'lucide-react';
import { attributeHelpers } from '@/core/stores/resources';
import { AttributeSelectionModal } from './AttributeSelectionModal';
import { SelectField } from '@/presentation/components/form/FormField/SelectField';
import { KitchenPrinterSelector } from '@/presentation/components/form/FormField/KitchenPrinterSelector';
import { AttributeDisplayTag } from '@/presentation/components/form/FormField/AttributeDisplayTag';

interface CategoryFormProps {
  formData: {
    name: string;
    kitchenPrinterId?: number;
    isKitchenPrintEnabled?: boolean;
    isLabelPrintEnabled?: boolean;
    selectedAttributeIds?: string[];
    attributeDefaultOptions?: Record<string, string | string[]>;
  };
  onFieldChange: (field: string, value: any) => void;
  t: (key: string) => string;
}

export const CategoryForm: React.FC<CategoryFormProps> = ({ formData, onFieldChange, t }) => {
  const [showAttributeModal, setShowAttributeModal] = useState(false);

  const selectedAttributeIds = formData.selectedAttributeIds || [];

  // Use stable helper directly (same reference every render)
  const getAttributeById = attributeHelpers.getAttributeById;

  return (
    <div className="space-y-4">
      <FormField label={t('settings.category.form.name')} required>
        <input
          value={formData.name}
          onChange={(e) => onFieldChange('name', e.target.value)}
          placeholder={t('settings.category.form.namePlaceholder')}
          className={inputClass}
        />
      </FormField>

      {/* Print Settings Section */}
      <section className="bg-white rounded-xl border border-gray-100 p-4 space-y-4 shadow-sm">
        <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900 pb-2 border-b border-gray-100">
          <Layers size={16} className="text-teal-500" />
          {t('settings.product.print.settings')}
        </h3>

        <div className="space-y-4">
          <SelectField
            label={t('settings.product.print.isKitchenPrintEnabled')}
            value={formData.isKitchenPrintEnabled ? 'true' : 'false'}
            onChange={(value) => onFieldChange('isKitchenPrintEnabled', value === 'true')}
            options={[
              { value: 'true', label: t('common.enabled') },
              { value: 'false', label: t('common.disabled') }
            ]}
          />

          {formData.isKitchenPrintEnabled && (
            <div className="pl-4 border-l-2 border-teal-100">
              <KitchenPrinterSelector
                value={formData.kitchenPrinterId}
                onChange={(value) => onFieldChange('kitchenPrinterId', value)}
                t={t}
              />
            </div>
          )}

          <div className="border-t border-gray-50 my-2"></div>

          <SelectField
            label={t('settings.product.print.isLabelPrintEnabled')}
            value={formData.isLabelPrintEnabled ? 'true' : 'false'}
            onChange={(value) => onFieldChange('isLabelPrintEnabled', value === 'true')}
            options={[
              { value: 'true', label: t('common.enabled') },
              { value: 'false', label: t('common.disabled') }
            ]}
          />
        </div>
      </section>

      {/* Category Attributes */}
      <section className="bg-white rounded-xl border border-gray-100 p-4 space-y-4 shadow-sm mt-4">
        <div className="flex items-center justify-between pb-2 border-b border-gray-100">
          <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900">
            <Layers size={16} className="text-teal-500" />
            {t('settings.category.form.attributes')}
          </h3>
          <button
            type="button"
            onClick={() => setShowAttributeModal(true)}
            className="text-xs font-bold text-teal-600 hover:text-teal-700 hover:underline"
          >
            {t('settings.product.attribute.manage')}
          </button>
        </div>

        <div className="min-h-[60px]">
          {selectedAttributeIds.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {selectedAttributeIds.map((id) => {
                const attr = getAttributeById(id);
                if (!attr) return null;

                const rawDefaults = formData.attributeDefaultOptions?.[id];
                const defaultOptionIds = Array.isArray(rawDefaults)
                    ? rawDefaults
                    : (rawDefaults ? [rawDefaults] : []);

                return (
                  <AttributeDisplayTag
                    key={id}
                    attribute={attr}
                    defaultOptionIds={defaultOptionIds}
                    t={t}
                  />
                );
              })}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-4 text-gray-400 bg-gray-50 rounded-lg border border-dashed border-gray-200">
              <p className="text-sm">{t('settings.product.attribute.noSelected')}</p>
            </div>
          )}
        </div>
      </section>

      <AttributeSelectionModal
        isOpen={showAttributeModal}
        onClose={() => setShowAttributeModal(false)}
        selectedAttributeIds={formData.selectedAttributeIds || []}
        attributeDefaultOptions={formData.attributeDefaultOptions || {}}
        onChange={(ids) => onFieldChange('selectedAttributeIds', ids)}
        onDefaultOptionChange={(attrId, optionIds) => {
          const newDefaults = { ...formData.attributeDefaultOptions, [attrId]: optionIds };
          if (!optionIds || optionIds.length === 0) delete newDefaults[attrId];
          onFieldChange('attributeDefaultOptions', newDefaults);
        }}
        t={t}
      />
    </div>
  );
};
