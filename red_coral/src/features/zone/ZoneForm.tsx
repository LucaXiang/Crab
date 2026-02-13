import React from 'react';
import { FormField, inputClass } from '@/shared/components/FormField';
import { MAX_NAME_LEN, MAX_NOTE_LEN } from '@/shared/constants/validation';

interface ZoneFormData {
  name: string;
  description?: string;
}

interface ZoneFormProps {
  formData: ZoneFormData;
  onFieldChange: <K extends keyof ZoneFormData>(field: K, value: ZoneFormData[K]) => void;
  t: (key: string) => string;
}

export const ZoneForm: React.FC<ZoneFormProps> = ({ formData, onFieldChange, t }) => {
  return (
    <div className="space-y-4">
      <FormField label={t('settings.table.zone.form.name')} required>
        <input
          value={formData.name}
          onChange={(e) => onFieldChange('name', e.target.value)}
          placeholder={t('settings.table.zone.form.name_placeholder')}
          maxLength={MAX_NAME_LEN}
          className={inputClass}
        />
      </FormField>

      <FormField label={t('settings.table.zone.form.description')}>
        <textarea
          value={formData.description || ''}
          onChange={(e) => onFieldChange('description', e.target.value)}
          placeholder={t('settings.table.zone.form.description_placeholder')}
          maxLength={MAX_NOTE_LEN}
          className={`${inputClass} resize-none`}
          rows={2}
        />
      </FormField>
    </div>
  );
};
