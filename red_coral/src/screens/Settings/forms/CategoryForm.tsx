import React, { useState } from 'react';
import { FormField, inputClass } from './FormField';
import { Layers, Filter } from 'lucide-react';
import { attributeHelpers, useTags } from '@/core/stores/resources';
import { AttributeSelectionModal } from './AttributeSelectionModal';
import { SelectField } from '@/presentation/components/form/FormField/SelectField';
import { KitchenPrinterSelector } from '@/presentation/components/form/FormField/KitchenPrinterSelector';
import { AttributeDisplayTag } from '@/presentation/components/form/FormField/AttributeDisplayTag';

interface CategoryFormProps {
  formData: {
    name: string;
    print_destinations?: number[];
    is_label_print_enabled?: boolean;
    selectedAttributeIds?: string[];
    attributeDefaultOptions?: Record<string, string | string[]>;
    is_virtual?: boolean;
    tag_ids?: string[];
    match_mode?: 'any' | 'all';
  };
  onFieldChange: (field: string, value: any) => void;
  t: (key: string) => string;
}

export const CategoryForm: React.FC<CategoryFormProps> = ({ formData, onFieldChange, t }) => {
  const [showAttributeModal, setShowAttributeModal] = useState(false);

  const selectedAttributeIds = formData.selectedAttributeIds || [];
  const tags = useTags();

  // Use stable helper directly (same reference every render)
  const getAttributeById = attributeHelpers.getAttributeById;

  // Virtual category state
  const isVirtual = formData.is_virtual ?? false;
  const selectedTagIds = formData.tag_ids || [];
  const matchMode = formData.match_mode || 'any';

  // Kitchen print state (derived from print_destinations array)
  const isKitchenPrintEnabled = (formData.print_destinations?.length ?? 0) > 0;
  const kitchenPrinterId = formData.print_destinations?.[0];

  // Toggle tag selection
  const handleTagToggle = (tagId: string) => {
    const newTagIds = selectedTagIds.includes(tagId)
      ? selectedTagIds.filter((id) => id !== tagId)
      : [...selectedTagIds, tagId];
    onFieldChange('tag_ids', newTagIds);
  };

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

      {/* Virtual Category Settings */}
      <section className="bg-white rounded-xl border border-gray-100 p-4 space-y-4 shadow-sm">
        <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900 pb-2 border-b border-gray-100">
          <Filter size={16} className="text-teal-500" />
          {t('settings.category.form.virtualSettings')}
        </h3>

        <div className="space-y-4">
          <SelectField
            label={t('settings.category.form.isVirtual')}
            value={isVirtual ? 'true' : 'false'}
            onChange={(value) => onFieldChange('is_virtual', value === 'true')}
            options={[
              { value: 'false', label: t('settings.category.form.regularCategory') },
              { value: 'true', label: t('settings.category.form.virtualCategory') }
            ]}
          />

          {isVirtual && (
            <div className="pl-4 border-l-2 border-teal-100 space-y-4">
              <SelectField
                label={t('settings.category.form.matchMode')}
                value={matchMode}
                onChange={(value) => onFieldChange('match_mode', value)}
                options={[
                  { value: 'any', label: t('settings.category.form.matchAny') },
                  { value: 'all', label: t('settings.category.form.matchAll') }
                ]}
              />

              <FormField label={t('settings.category.form.filterTags')}>
                <div className="min-h-[60px]">
                  {tags.length > 0 ? (
                    <div className="flex flex-wrap gap-2">
                      {tags.map((tag) => {
                        const isSelected = selectedTagIds.includes(tag.id);
                        return (
                          <button
                            key={tag.id}
                            type="button"
                            onClick={() => handleTagToggle(tag.id)}
                            className={`px-3 py-1.5 rounded-full text-sm font-medium transition-all ${
                              isSelected
                                ? 'bg-teal-500 text-white shadow-sm'
                                : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                            }`}
                            style={isSelected ? { backgroundColor: tag.color || undefined } : undefined}
                          >
                            {tag.name}
                          </button>
                        );
                      })}
                    </div>
                  ) : (
                    <div className="flex flex-col items-center justify-center py-4 text-gray-400 bg-gray-50 rounded-lg border border-dashed border-gray-200">
                      <p className="text-sm">{t('settings.category.form.noTagsAvailable')}</p>
                    </div>
                  )}
                </div>
              </FormField>

              {selectedTagIds.length === 0 && isVirtual && (
                <p className="text-xs text-amber-600">
                  {t('settings.category.form.selectTagsHint')}
                </p>
              )}
            </div>
          )}
        </div>
      </section>

      {/* Print Settings Section */}
      <section className="bg-white rounded-xl border border-gray-100 p-4 space-y-4 shadow-sm">
        <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900 pb-2 border-b border-gray-100">
          <Layers size={16} className="text-teal-500" />
          {t('settings.product.print.settings')}
        </h3>

        <div className="space-y-4">
          <SelectField
            label={t('settings.product.print.isKitchenPrintEnabled')}
            value={isKitchenPrintEnabled ? 'true' : 'false'}
            onChange={(value) => {
              if (value === 'true') {
                // Enable: set print_destinations with current printer or empty (will select later)
                onFieldChange('print_destinations', kitchenPrinterId ? [kitchenPrinterId] : []);
              } else {
                // Disable: clear print_destinations
                onFieldChange('print_destinations', []);
              }
            }}
            options={[
              { value: 'true', label: t('common.status.enabled') },
              { value: 'false', label: t('common.status.disabled') }
            ]}
          />

          {isKitchenPrintEnabled && (
            <div className="pl-4 border-l-2 border-teal-100">
              <KitchenPrinterSelector
                value={kitchenPrinterId}
                onChange={(value) => onFieldChange('print_destinations', value ? [value] : [])}
                t={t}
              />
            </div>
          )}

          <div className="border-t border-gray-50 my-2"></div>

          <SelectField
            label={t('settings.product.print.isLabelPrintEnabled')}
            value={formData.is_label_print_enabled ? 'true' : 'false'}
            onChange={(value) => onFieldChange('is_label_print_enabled', value === 'true')}
            options={[
              { value: 'true', label: t('common.status.enabled') },
              { value: 'false', label: t('common.status.disabled') }
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
