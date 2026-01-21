import React from 'react';
import { FormField, inputClass } from './FormField';
import { SelectField } from '@/presentation/components/form/FormField/SelectField';

interface ZoneFormProps {
  formData: {
    name: string;
    surchargeType: 'none' | 'fixed' | 'percentage';
    surchargeAmount: number;
  };
  onFieldChange: (field: string, value: any) => void;
  t: (key: string) => string;
}

export const ZoneForm: React.FC<ZoneFormProps> = ({ formData, onFieldChange, t }) => {
  return (
    <>
      <FormField label={t('settings.table.zone.form.name')} required>
        <input
          value={formData.name}
          onChange={(e) => onFieldChange('name', e.target.value)}
          placeholder={t('settings.table.zone.form.namePlaceholder')}
          className={inputClass}
        />
      </FormField>
      <div className="grid grid-cols-2 gap-4">
        <SelectField
          label={t('settings.table.zone.form.surchargeType')}
          value={formData.surchargeType}
          onChange={(value) => onFieldChange('surchargeType', value as string)}
          options={[
            { value: 'none', label: t('common.dialog.none') },
            { value: 'fixed', label: t("settings.table.zone.form.surchargeFixed") },
            { value: 'percentage', label: t("settings.table.zone.form.surchargePercentage") },
          ]}
        />

        {formData.surchargeType !== 'none' && (
          <FormField label={t('settings.table.zone.form.surchargeAmount')}>
            <div className="relative">
              <input
                type="number"
                value={formData.surchargeAmount}
                onChange={(e) => onFieldChange('surchargeAmount', parseFloat(e.target.value))}
                className={`${inputClass} pl-8`}
                step={formData.surchargeType === 'percentage' ? '1' : '0.01'}
              />
              <span className="absolute left-3 top-2.5 text-gray-500">
                {formData.surchargeType === 'percentage' ? '%' : '€'}
              </span>
            </div>
          </FormField>
        )}
      </div>
    </>
  );
};
