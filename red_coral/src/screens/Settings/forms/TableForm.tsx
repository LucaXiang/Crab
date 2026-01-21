import React from 'react';
import { FormField, inputClass } from './FormField';
import { SelectField } from '@/presentation/components/form/FormField/SelectField';
import { Zone } from '@/core/domain/types';

interface TableFormProps {
  formData: {
    name: string;
    capacity: number;
    zoneId: string;
  };
  zones: Zone[];
  onFieldChange: (field: string, value: any) => void;
  t: (key: string) => string;
}

export const TableForm: React.FC<TableFormProps> = ({ formData, zones, onFieldChange, t }) => {
  return (
    <>
      <FormField label={t('settings.table.form.name')} required>
        <input
          value={formData.name}
          onChange={(e) => onFieldChange('name', e.target.value)}
          placeholder={t('settings.table.form.namePlaceholder')}
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
          value={formData.zoneId}
          onChange={(value) => onFieldChange('zoneId', value as string)}
          options={zones.map((z) => ({ value: z.id, label: z.name }))}
          required
        />
      </div>
    </>
  );
};
