import React, { useEffect, useState } from 'react';
import { Server, X, Monitor, Wifi } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { FormField, FormSection, inputClass, selectClass } from '@/shared/components/FormField';

type PrinterType = 'driver' | 'network';

interface PrinterFormData {
  name: string;
  description: string;
  printerType: PrinterType;
  // Driver printer fields
  driverName: string;
  // Network printer fields
  ip: string;
  port: number;
}

interface PrinterEditModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialData?: {
    id?: string;
    name?: string;
    description?: string;
    printerType?: PrinterType;
    driverName?: string;
    ip?: string;
    port?: number;
    // Legacy field for backward compatibility
    printerName?: string;
  } | null;
  onSave: (data: PrinterFormData) => Promise<void>;
  systemPrinters: string[];
}

const DEFAULT_PORT = 9100;

export const PrinterEditModal: React.FC<PrinterEditModalProps> = ({
  isOpen,
  onClose,
  initialData,
  onSave,
  systemPrinters,
}) => {
  const { t } = useI18n();
  const [formData, setFormData] = useState<PrinterFormData>({
    name: '',
    description: '',
    printerType: 'driver',
    driverName: '',
    ip: '',
    port: DEFAULT_PORT,
  });
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (isOpen) {
      // Determine printer type from initial data
      let printerType: PrinterType = 'driver';
      if (initialData?.printerType) {
        printerType = initialData.printerType;
      } else if (initialData?.ip) {
        printerType = 'network';
      }

      setFormData({
        name: initialData?.name || '',
        description: initialData?.description || '',
        printerType,
        driverName: initialData?.driverName || initialData?.printerName || '',
        ip: initialData?.ip || '',
        port: initialData?.port || DEFAULT_PORT,
      });
    }
  }, [isOpen, initialData]);

  const handleSave = async () => {
    if (!formData.name) return;

    // Validate based on printer type
    if (formData.printerType === 'driver' && !formData.driverName) return;
    if (formData.printerType === 'network' && !formData.ip) return;

    setSaving(true);
    try {
      await onSave(formData);
      onClose();
    } finally {
      setSaving(false);
    }
  };

  const isValid = formData.name && (
    (formData.printerType === 'driver' && formData.driverName) ||
    (formData.printerType === 'network' && formData.ip)
  );

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
              {initialData?.id
                ? t('settings.printer.print_stations.edit')
                : t('settings.printer.print_stations.add')}
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
          {/* Basic Info */}
          <FormSection title={t('settings.attribute.section.basic')} icon={Server}>
            <FormField label={t('common.label.name')} required>
              <input
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder={t('settings.printer.print_stations.name_placeholder')}
                className={inputClass}
                autoFocus
              />
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

          {/* Printer Type Selection */}
          <FormSection title={t('settings.printer.print_stations.printer_config')} icon={Monitor}>
            {/* Type Toggle */}
            <div className="flex bg-gray-100 p-1 rounded-xl mb-4">
              <button
                type="button"
                onClick={() => setFormData({ ...formData, printerType: 'driver' })}
                className={`flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium transition-all ${
                  formData.printerType === 'driver'
                    ? 'bg-white text-green-700 shadow-sm'
                    : 'text-gray-600 hover:text-gray-900'
                }`}
              >
                <Monitor size={16} />
                {t('settings.printer.print_stations.type_local')}
              </button>
              <button
                type="button"
                onClick={() => setFormData({ ...formData, printerType: 'network' })}
                className={`flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium transition-all ${
                  formData.printerType === 'network'
                    ? 'bg-white text-blue-700 shadow-sm'
                    : 'text-gray-600 hover:text-gray-900'
                }`}
              >
                <Wifi size={16} />
                {t('settings.printer.print_stations.type_network')}
              </button>
            </div>

            {/* Driver Printer Fields */}
            {formData.printerType === 'driver' && (
              <div className="animate-in fade-in slide-in-from-top-1 duration-200">
                <FormField label={t('settings.printer.form.target_printer')} required>
                  <div className="relative">
                    <select
                      value={formData.driverName}
                      onChange={(e) => setFormData({ ...formData, driverName: e.target.value })}
                      className={selectClass}
                    >
                      <option value="">{t('settings.printer.form.select_system_printer')}</option>
                      {systemPrinters.map((p) => (
                        <option key={p} value={p}>
                          {p}
                        </option>
                      ))}
                    </select>
                  </div>
                </FormField>
                {systemPrinters.length === 0 && (
                  <p className="text-xs text-amber-600 mt-2">
                    {t('settings.printer.message.no_printers')}
                  </p>
                )}
              </div>
            )}

            {/* Network Printer Fields */}
            {formData.printerType === 'network' && (
              <div className="animate-in fade-in slide-in-from-top-1 duration-200 space-y-3">
                <FormField label={t('settings.printer.print_stations.ip_address')} required>
                  <input
                    type="text"
                    value={formData.ip}
                    onChange={(e) => setFormData({ ...formData, ip: e.target.value })}
                    placeholder="192.168.1.100"
                    className={inputClass}
                    pattern="^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$"
                  />
                </FormField>

                <FormField label={t('settings.printer.print_stations.port')}>
                  <input
                    type="number"
                    value={formData.port}
                    onChange={(e) =>
                      setFormData({ ...formData, port: parseInt(e.target.value) || DEFAULT_PORT })
                    }
                    placeholder="9100"
                    min={1}
                    max={65535}
                    className={inputClass}
                  />
                  <p className="text-xs text-gray-500 mt-1">
                    {t('settings.printer.print_stations.port_hint')}
                  </p>
                </FormField>
              </div>
            )}
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
            onClick={handleSave}
            disabled={!isValid || saving}
            className="px-5 py-2.5 bg-indigo-600 text-white rounded-xl text-sm font-semibold hover:bg-indigo-700 transition-colors shadow-lg shadow-indigo-600/20 disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none flex items-center gap-2"
          >
            {saving && (
              <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            )}
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
