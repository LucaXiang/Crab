import React from 'react';
import { Tag, Palette, Settings } from 'lucide-react';
import { FormField, FormSection, inputClass } from './FormField';
import { SelectField } from '@/presentation/components/form/FormField/SelectField';

interface TagFormProps {
  formData: {
    name: string;
    color: string;
    display_order?: number;
    is_active?: boolean;
  };
  onFieldChange: (field: string, value: any) => void;
  t: (key: string) => string;
}

// Predefined color palette for quick selection
const colorPalette = [
  '#3B82F6', // blue
  '#10B981', // emerald
  '#F59E0B', // amber
  '#EF4444', // red
  '#8B5CF6', // violet
  '#EC4899', // pink
  '#06B6D4', // cyan
  '#84CC16', // lime
  '#F97316', // orange
  '#6366F1', // indigo
];

export const TagForm: React.FC<TagFormProps> = ({ formData, onFieldChange, t }) => {
  const selectedColor = formData.color || '#3B82F6';

  return (
    <div className="space-y-4">
      {/* Basic Info */}
      <FormSection title={t('settings.attribute.section.basic')} icon={Tag}>
        <FormField label={t('settings.tag.name')} required>
          <input
            value={formData.name}
            onChange={(e) => onFieldChange('name', e.target.value)}
            placeholder={t('settings.tag.name_placeholder')}
            className={inputClass}
          />
        </FormField>
      </FormSection>

      {/* Color Settings */}
      <FormSection title={t('settings.tag.color')} icon={Palette}>
        {/* Color palette */}
        <div className="flex flex-wrap gap-2">
          {colorPalette.map((color) => (
            <button
              key={color}
              type="button"
              onClick={() => onFieldChange('color', color)}
              className={`w-8 h-8 rounded-lg transition-all ${
                selectedColor === color
                  ? 'ring-2 ring-offset-2 ring-gray-400 scale-110'
                  : 'hover:scale-105'
              }`}
              style={{ backgroundColor: color }}
              title={color}
            />
          ))}
        </div>

        {/* Custom color input */}
        <div className="flex items-center gap-3">
          <input
            type="color"
            value={selectedColor}
            onChange={(e) => onFieldChange('color', e.target.value)}
            className="w-10 h-10 rounded-lg cursor-pointer border border-gray-200"
          />
          <input
            type="text"
            value={selectedColor}
            onChange={(e) => {
              const value = e.target.value;
              if (/^#[0-9A-Fa-f]{0,6}$/.test(value)) {
                onFieldChange('color', value);
              }
            }}
            placeholder="#3B82F6"
            className={`${inputClass} flex-1 font-mono`}
          />
        </div>

        {/* Preview */}
        <div className="flex items-center gap-2 pt-2 border-t border-gray-100">
          <span className="text-sm text-gray-500">{t('settings.tag.preview')}:</span>
          <span
            className="px-3 py-1 rounded-full text-sm font-medium text-white"
            style={{ backgroundColor: selectedColor }}
          >
            {formData.name || t('settings.tag.sample_tag')}
          </span>
        </div>
      </FormSection>

      {/* Status Settings */}
      <FormSection title={t('common.label.status')} icon={Settings}>
        <SelectField
          label={t('common.label.is_active')}
          value={formData.is_active !== false ? 'true' : 'false'}
          onChange={(value) => onFieldChange('is_active', value === 'true')}
          options={[
            { value: 'true', label: t('common.status.enabled') },
            { value: 'false', label: t('common.status.disabled') },
          ]}
        />
      </FormSection>
    </div>
  );
};
