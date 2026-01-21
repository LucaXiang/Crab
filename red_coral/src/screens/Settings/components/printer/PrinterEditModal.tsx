import React, { useEffect, useState } from 'react';
import { Printer, Edit2, Plus, Save, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

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
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/20 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="bg-white rounded-2xl shadow-xl w-full max-w-md overflow-hidden animate-in zoom-in-95 duration-200 border border-gray-100">
        <div className="px-6 py-4 border-b border-gray-100 flex justify-between items-center bg-gray-50/50">
          <h3 className="font-bold text-gray-800 flex items-center gap-2">
            {initialData ? <Edit2 size={18} className="text-blue-500" /> : <Plus size={18} className="text-blue-500" />}
            {initialData ? (t('settings.printer.kitchenStation.editStation')) : (t('settings.printer.kitchenStation.addStation'))}
          </h3>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full text-gray-400 hover:text-gray-600 transition-colors">
            <X size={20} />
          </button>
        </div>

        <div className="p-6 space-y-4">
          <div>
            <label className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5 block">{t('common.label.name')}</label>
            <input
              className="w-full border border-gray-200 rounded-xl px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500 transition-all bg-gray-50 focus:bg-white"
              placeholder={t('common.hint.namePlaceholder')}
              value={formData.name}
              onChange={e => setFormData({ ...formData, name: e.target.value })}
              autoFocus
            />
          </div>

          <div>
            <label className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5 block">{t('settings.printer.form.targetPrinter')}</label>
            <div className="relative">
              <select
                className="w-full border border-gray-200 rounded-xl px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500 transition-all bg-gray-50 focus:bg-white appearance-none"
                value={formData.printerName}
                onChange={e => setFormData({ ...formData, printerName: e.target.value })}
              >
                <option value="">{t('settings.printer.form.selectSystemPrinter')}</option>
                {systemPrinters.map(p => (
                  <option key={p} value={p}>{p}</option>
                ))}
              </select>
              <Printer className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none" size={16} />
            </div>
          </div>

          <div>
            <label className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5 block">{t('common.label.description')}</label>
            <input
              className="w-full border border-gray-200 rounded-xl px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500 transition-all bg-gray-50 focus:bg-white"
              placeholder={t('common.hint.descriptionPlaceholder')}
              value={formData.description}
              onChange={e => setFormData({ ...formData, description: e.target.value })}
            />
          </div>
        </div>

        <div className="px-6 py-4 bg-gray-50 border-t border-gray-100 flex justify-end gap-3">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-600 hover:bg-gray-200 rounded-xl transition-colors"
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
            className="px-4 py-2 text-sm font-bold bg-blue-600 text-white hover:bg-blue-700 rounded-xl shadow-lg shadow-blue-200 transition-all flex items-center gap-2 disabled:opacity-50 disabled:shadow-none"
          >
            <Save size={16} />
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
