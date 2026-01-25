import React, { useState } from 'react';
import { FormField, FormSection, SubField, inputClass, SelectField, KitchenPrinterSelector, AttributeDisplayTag } from '@/shared/components/FormField';
import { Printer, Filter, Tags, Settings } from 'lucide-react';
import { attributeHelpers, useTags } from '@/core/stores/resources';
import { AttributeSelectionModal } from '@/features/attribute';

import type { PrintState } from '@/core/domain/types';

interface CategoryFormProps {
  formData: {
    name: string;
    print_destinations?: string[];
    is_kitchen_print_enabled?: PrintState;  // 0=disabled, 1=enabled
    is_label_print_enabled?: PrintState;    // 0=disabled, 1=enabled
    is_active?: boolean;
    selected_attribute_ids?: string[];
    attribute_default_options?: Record<string, string | string[]>;
    is_virtual?: boolean;
    tag_ids?: string[];
    match_mode?: 'any' | 'all';
    is_display?: boolean;
  };
  onFieldChange: (field: string, value: any) => void;
  t: (key: string) => string;
  /** Edit mode disables changing category type */
  isEditMode?: boolean;
}

export const CategoryForm: React.FC<CategoryFormProps> = ({ formData, onFieldChange, t, isEditMode = false }) => {
  const [showAttributeModal, setShowAttributeModal] = useState(false);

  const selectedAttributeIds = formData.selected_attribute_ids || [];
  const tags = useTags();

  // Use stable helper directly (same reference every render)
  const getAttributeById = attributeHelpers.getAttributeById;

  // Virtual category state
  const isVirtual = formData.is_virtual ?? false;
  const selectedTagIds = formData.tag_ids || [];
  const matchMode = formData.match_mode || 'any';

  // Kitchen print state - PrintState: 0=disabled, 1=enabled
  const printDestinations = formData.print_destinations ?? [];
  const kitchenPrinterId = printDestinations[0];
  // Default enabled(1), 0 is disabled so can't use ?? since 0 is falsy
  const isKitchenPrintEnabled: PrintState = formData.is_kitchen_print_enabled === 0 ? 0 : (formData.is_kitchen_print_enabled ?? 1);
  const isLabelPrintEnabled: PrintState = formData.is_label_print_enabled === 0 ? 0 : (formData.is_label_print_enabled ?? 1);

  // Toggle tag selection
  const handleTagToggle = (tagId: string) => {
    const newTagIds = selectedTagIds.includes(tagId)
      ? selectedTagIds.filter((id) => id !== tagId)
      : [...selectedTagIds, tagId];
    onFieldChange('tag_ids', newTagIds);
  };

  return (
    <div className="space-y-4">
      {/* Basic info - display directly, no section */}
      <FormField label={t('settings.category.form.name')} required>
        <input
          value={formData.name}
          onChange={(e) => onFieldChange('name', e.target.value)}
          placeholder={t('settings.category.form.name_placeholder')}
          className={inputClass}
        />
      </FormField>

      {/* Virtual category settings - show type selector when creating, only show for virtual when editing */}
      {(!isEditMode || isVirtual) && (
      <FormSection title={t('settings.category.form.virtual_settings')} icon={Filter}>
        {/* Show type selector when creating */}
        {!isEditMode && (
          <SelectField
            label={t('settings.category.form.is_virtual')}
            value={isVirtual ? 'true' : 'false'}
            onChange={(value) => onFieldChange('is_virtual', String(value) === 'true')}
            options={[
              { value: 'false', label: t('settings.category.form.regular_category') },
              { value: 'true', label: t('settings.category.form.virtual_category') }
            ]}
          />
        )}

        <SubField show={isVirtual}>
          <SelectField
            label={t('settings.category.form.match_mode')}
            value={matchMode}
            onChange={(value) => onFieldChange('match_mode', value)}
            options={[
              { value: 'any', label: t('settings.category.form.match_any') },
              { value: 'all', label: t('settings.category.form.match_all') }
            ]}
          />

          <FormField label={t('settings.category.form.filter_tags')}>
            <div className="min-h-[3.75rem]">
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
                  <p className="text-sm">{t('settings.category.form.no_tags_available')}</p>
                </div>
              )}
            </div>
          </FormField>

          {selectedTagIds.length === 0 && (
            <p className="text-xs text-amber-600">
              {t('settings.category.form.select_tags_hint')}
            </p>
          )}

          <SelectField
            label={t('settings.category.form.is_display')}
            value={formData.is_display !== false ? 'true' : 'false'}
            onChange={(value) => onFieldChange('is_display', String(value) === 'true')}
            options={[
              { value: 'true', label: t('common.status.show') },
              { value: 'false', label: t('common.status.hide') }
            ]}
          />
        </SubField>
      </FormSection>
      )}

      {/* Print settings - not shown for virtual categories */}
      {!isVirtual && (
      <FormSection title={t('settings.product.print.settings')} icon={Printer}>
        <SelectField
          label={t('settings.product.print.is_kitchen_print_enabled')}
          value={String(isKitchenPrintEnabled)}
          onChange={(value) => onFieldChange('is_kitchen_print_enabled', Number(value) as PrintState)}
          options={[
            { value: '1', label: t('common.status.enabled') },
            { value: '0', label: t('common.status.disabled') }
          ]}
        />

        <SubField show={isKitchenPrintEnabled === 1}>
          <KitchenPrinterSelector
            value={kitchenPrinterId}
            onChange={(value) => onFieldChange('print_destinations', value ? [value] : [])}
            t={t}
          />
        </SubField>

        <div className="border-t border-gray-100 pt-3">
          <SelectField
            label={t('settings.product.print.is_label_print_enabled')}
            value={String(isLabelPrintEnabled)}
            onChange={(value) => onFieldChange('is_label_print_enabled', Number(value) as PrintState)}
            options={[
              { value: '1', label: t('common.status.enabled') },
              { value: '0', label: t('common.status.disabled') }
            ]}
          />
        </div>
      </FormSection>
      )}

      {/* Category attributes - not shown for virtual categories */}
      {!isVirtual && (
      <FormSection title={t('settings.category.form.attributes')} icon={Tags}>
        <div className="flex items-center justify-between mb-3">
          <p className="text-xs text-gray-500">{t('settings.category.form.attributes_hint')}</p>
          <button
            type="button"
            onClick={() => setShowAttributeModal(true)}
            className="text-xs font-bold text-teal-600 hover:text-teal-700 hover:underline"
          >
            {t('settings.product.attribute.manage')}
          </button>
        </div>

        <div className="min-h-[3.75rem]">
          {selectedAttributeIds.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {selectedAttributeIds.map((id) => {
                const attr = getAttributeById(id);
                if (!attr) return null;

                const rawDefaults = formData.attribute_default_options?.[id];
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
              <p className="text-sm">{t('settings.product.attribute.no_selected')}</p>
            </div>
          )}
        </div>
      </FormSection>
      )}

      {/* Status Settings */}
      <FormSection title={t('common.label.status')} icon={Settings}>
        <SelectField
          label={t('common.label.is_active')}
          value={formData.is_active !== false ? 'true' : 'false'}
          onChange={(value) => onFieldChange('is_active', String(value) === 'true')}
          options={[
            { value: 'true', label: t('common.status.enabled') },
            { value: 'false', label: t('common.status.disabled') },
          ]}
        />
      </FormSection>

      <AttributeSelectionModal
        isOpen={showAttributeModal}
        onClose={() => setShowAttributeModal(false)}
        selectedAttributeIds={formData.selected_attribute_ids || []}
        attributeDefaultOptions={formData.attribute_default_options || {}}
        onChange={(ids) => onFieldChange('selected_attribute_ids', ids)}
        onDefaultOptionChange={(attrId, optionIds) => {
          const newDefaults = { ...formData.attribute_default_options, [attrId]: optionIds };
          if (!optionIds || optionIds.length === 0) delete newDefaults[attrId];
          onFieldChange('attribute_default_options', newDefaults);
        }}
        t={t}
      />
    </div>
  );
};
