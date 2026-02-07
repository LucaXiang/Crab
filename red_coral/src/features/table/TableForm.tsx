import React from 'react';
import { SquareMenu, Settings } from 'lucide-react';
import { FormField, FormSection, inputClass, SelectField } from '@/shared/components/FormField';
import { Zone } from '@/core/domain/types';

interface TableFormData {
  name: string;
  capacity: number;
  zone_id?: number;
  is_active?: boolean;
}

interface TableFormProps {
  formData: TableFormData;
  zones: Zone[];
  onFieldChange: <K extends keyof TableFormData>(field: K, value: TableFormData[K]) => void;
  t: (key: string) => string;
}

export const TableForm: React.FC<TableFormProps> = ({ formData, zones, onFieldChange, t }) => {
  return (
    <div className="space-y-4">
      <FormSection title={t('settings.attribute.section.basic')} icon={SquareMenu}>
        <FormField label={t('settings.table.form.name')} required>
          <input
            value={formData.name}
            onChange={(e) => onFieldChange('name', e.target.value)}
            placeholder={t('settings.table.form.name_placeholder')}
            className={inputClass}
          />
        </FormField>

        <div className="grid grid-cols-2 gap-4">
          <FormField label={t('settings.table.form.capacity')} required>
            <input
              type="number"
              min={1}
              value={formData.capacity}
              onChange={(e) => onFieldChange('capacity', parseInt(e.target.value || '0') || 0)}
              placeholder={t('settings.form.placeholder.capacity')}
              className={inputClass}
            />
          </FormField>

          <SelectField
            label={t('table.zones')}
            value={formData.zone_id}
            onChange={(value) => onFieldChange('zone_id', value as number)}
            options={zones.map((z) => ({ value: z.id, label: z.name }))}
            required
          />
        </div>
      </FormSection>

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
    </div>
  );
};
