import React from 'react';
import { FormField, FormSection, inputClass } from './FormField';
import { Settings } from 'lucide-react';
import { SelectField } from '@/presentation/components/form/FormField/SelectField';

interface ZoneFormProps {
  formData: {
    name: string;
    is_active?: boolean;
  };
  onFieldChange: (field: string, value: any) => void;
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
          className={inputClass}
        />
      </FormField>

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
