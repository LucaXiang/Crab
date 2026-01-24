import React, { useEffect, useState } from 'react';
import { Printer, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { FormField, FormSection, inputClass, selectClass } from '@/shared/components/FormField';

interface PrinterFormData {
  name: string;
  printerName: string;
  description: string;
}

interface PrinterEditModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialData?: { name?: string; printerName?: string; description?: string } | null;
  onSave: (data: PrinterFormData) => Promise<void>;
  systemPrinters: string[];
}

export const PrinterEditModal: React.FC<PrinterEditModalProps> = ({
  isOpen,
  onClose,
  initialData,
  onSave,
  systemPrinters
}) => {
  const { t } = useI18n();
  const [formData, setFormData] = useState<PrinterFormData>({ name: '', printerName: '', description: '' });

  useEffect(() => {
    if (isOpen) {
      setFormData({
        name: initialData?.name || '',
        printerName: initialData?.printerName || '',
        description: initialData?.description || ''
      });
    }
  }, [isOpen, initialData]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={onClose}
    >
      <div
        className="bg-gray-50 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-gray-200 bg-white">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-gray-900">
              {initialData
                ? t('settings.printer.kitchen_station.edit_station')
                : t('settings.printer.kitchen_station.add_station')}
            </h2>
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
          <FormSection title={t('settings.attribute.section.basic')} icon={Printer}>
            <FormField label={t('common.label.name')} required>
              <input
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder={t('common.hint.name_placeholder')}
                className={inputClass}
                autoFocus
              />
            </FormField>

            <FormField label={t('settings.printer.form.target_printer')} required>
              <div className="relative">
                <select
                  value={formData.printerName}
                  onChange={(e) => setFormData({ ...formData, printerName: e.target.value })}
                  className={selectClass}
                >
                  <option value="">{t('settings.printer.form.select_system_printer')}</option>
                  {systemPrinters.map((p) => (
                    <option key={p} value={p}>{p}</option>
                  ))}
                </select>
              </div>
            </FormField>

            <FormField label={t('common.label.description')}>
              <input
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                placeholder={t('common.hint.description_placeholder')}
                className={inputClass}
              />
            </FormField>
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
            onClick={() => {
              if (formData.name) {
                onSave(formData);
              }
            }}
            disabled={!formData.name}
            className="px-5 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-semibold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20 disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none"
          >
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
