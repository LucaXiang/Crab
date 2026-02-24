import React, { useMemo } from 'react';
import { X, Type, FileText, Star } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { FormField, FormSection, inputClass } from '@/shared/components/FormField';
import { useFormInitialization } from '@/hooks/useFormInitialization';
import { usePriceInput } from '@/hooks/usePriceInput';
import type { ProductSpec } from '@/core/domain/types';
import { validateSpecData, createEmptySpec } from './spec-utils';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';

interface SpecFormData {
  name: string;
  receiptName: string;
  price: number;
  isDefault: boolean;
}

const mapToFormData = (spec: ProductSpec | null): SpecFormData => {
  if (!spec) {
    const empty = createEmptySpec();
    return {
      name: '',
      receiptName: '',
      price: empty.price ?? 0,
      isDefault: false,
    };
  }
  return {
    name: spec.name,
    receiptName: spec.receipt_name || '',
    price: spec.price,
    isDefault: spec.is_default,
  };
};

interface SpecificationFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  spec: ProductSpec | null;
  specIndex: number | null;
  isRootSpec: boolean;
  onSave: (spec: ProductSpec, index: number | null) => Promise<void>;
}

export const SpecificationFormModal: React.FC<SpecificationFormModalProps> = React.memo(({
  isOpen,
  onClose,
  spec,
  specIndex,
  isRootSpec,
  onSave,
}) => {
  const { t } = useI18n();
  const isEditing = spec !== null;

  // Memoize the initial form data to prevent useEffect from re-running on every render
  // Specs don't have id, use specIndex from parent as stable key
  const initialFormData = useMemo(
    () => (spec ? mapToFormData(spec) : null),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [specIndex]
  );

  const [formData, setFormData] = useFormInitialization<SpecFormData>(
    initialFormData,
    mapToFormData(null),
    [isOpen]
  );

  const [isSubmitting, setIsSubmitting] = React.useState(false);

  const { priceInput, handlePriceChange, commitPrice, handlePriceKeyDown } = usePriceInput(
    formData.price || 0,
    {
      minValue: 0,
      onCommit: (value) => setFormData((prev) => ({ ...prev, price: value })),
    }
  );

  const handleFieldChange = <K extends keyof SpecFormData>(field: K, value: SpecFormData[K]) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  const handleSubmit = async () => {
    commitPrice();

    const specData: Partial<ProductSpec> = {
      name: formData.name.trim(),
      price: formData.price,
    };

    const error = validateSpecData(specData, isRootSpec, t);
    if (error) {
      toast.error(error);
      return;
    }

    setIsSubmitting(true);
    try {
      const fullSpec: ProductSpec = {
        name: formData.name.trim(),
        receipt_name: formData.receiptName.trim() || null,
        price: (isRootSpec && isEditing && spec) ? spec.price : formData.price,
        display_order: spec?.display_order ?? 0,
        is_default: formData.isDefault,
        is_root: isRootSpec,
        is_active: true,
      };

      await onSave(fullSpec, specIndex);
      onClose();
    } catch (error) {
      logger.error('Failed to save spec', error);
      toast.error(t('common.message.error'));
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  const canSubmit =
    (isRootSpec || formData.name.trim()) &&
    !isSubmitting;

  return (
    <div
      className="fixed inset-0 z-90 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={onClose}
    >
      <div
        className="bg-gray-50 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-gray-200 bg-white">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <h2 className="text-lg font-bold text-gray-900">
                {isEditing ? t('settings.specification.edit') : t('settings.specification.add_new')}
              </h2>
              {isRootSpec && (
                <span className="px-2 py-0.5 text-xs font-medium bg-amber-100 text-amber-700 rounded">
                  {t('settings.specification.label.root')}
                </span>
              )}
            </div>
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
          <FormSection title={t('settings.attribute.section.basic')} icon={Type}>
            <FormField label={t('settings.specification.form.name')} required={!isRootSpec}>
              <input
                value={formData.name}
                onChange={(e) => handleFieldChange('name', e.target.value)}
                placeholder={t('settings.specification.form.name_placeholder')}
                className={inputClass}
                autoFocus
              />
            </FormField>

            <FormField label={t('settings.specification.form.receipt_name')}>
              <div className="relative">
                <div className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400">
                  <FileText size={14} />
                </div>
                <input
                  value={formData.receiptName}
                  onChange={(e) => handleFieldChange('receiptName', e.target.value)}
                  placeholder={t('settings.specification.form.receipt_name_placeholder')}
                  className={`${inputClass} pl-9`}
                />
              </div>
            </FormField>

            <FormField label={t('settings.specification.form.price')} required={!isRootSpec}>
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 text-sm">EUR</span>
                <input
                  type="text"
                  inputMode="decimal"
                  value={priceInput}
                  onChange={handlePriceChange}
                  onBlur={commitPrice}
                  onFocus={(e) => e.currentTarget.select()}
                  onKeyDown={handlePriceKeyDown}
                  placeholder={t('settings.form.placeholder.price')}
                  className={`${inputClass} pl-10 ${isRootSpec && isEditing ? 'bg-gray-100 text-gray-500 cursor-not-allowed' : ''}`}
                  disabled={isRootSpec && isEditing}
                />
              </div>
              {isRootSpec && isEditing && (
                <p className="mt-1 text-xs text-gray-500">
                  {t('settings.specification.form.base_spec_price_hint')}
                </p>
              )}
            </FormField>

          </FormSection>

          {/* Default Toggle */}
          <FormSection title={t('settings.specification.form.set_default')} icon={Star}>
            <div
              onClick={() => handleFieldChange('isDefault', !formData.isDefault)}
              className={`flex items-center gap-3 p-3 rounded-xl border cursor-pointer transition-colors ${
                formData.isDefault
                  ? 'bg-orange-50 border-orange-200'
                  : 'bg-white border-gray-200 hover:border-gray-300'
              }`}
            >
              <div
                className={`w-5 h-5 rounded-full border-2 flex items-center justify-center transition-colors ${
                  formData.isDefault
                    ? 'border-orange-500 bg-orange-500'
                    : 'border-gray-300'
                }`}
              >
                {formData.isDefault && (
                  <Star size={12} className="text-white fill-white" />
                )}
              </div>
              <div className="flex-1">
                <span className={`font-medium ${formData.isDefault ? 'text-orange-700' : 'text-gray-700'}`}>
                  {formData.isDefault ? t('settings.specification.label.default') : t('settings.specification.label.set_default')}
                </span>
                <p className="text-xs text-gray-500 mt-0.5">
                  {t('settings.specification.form.set_default_hint')}
                </p>
              </div>
            </div>
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
            disabled={!canSubmit}
            className="px-5 py-2.5 bg-orange-600 text-white rounded-xl text-sm font-semibold hover:bg-orange-700 transition-colors shadow-lg shadow-orange-600/20 disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none disabled:hover:bg-orange-600"
          >
            {isSubmitting ? t('common.message.loading') : (isEditing ? t('common.action.save') : t('common.action.create'))}
          </button>
        </div>
      </div>
    </div>
  );
});
