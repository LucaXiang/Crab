import React from 'react';
import { AlertCircle, Settings } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface PrinterSelectProps {
  label: string;
  icon: React.ElementType;
  value: string | null;
  onChange: (val: string | null) => void;
  printers: string[];
  loading: boolean;
  description?: string;
  badge?: React.ReactNode;
}

export const PrinterSelect: React.FC<PrinterSelectProps> = ({
  label,
  icon: Icon,
  value,
  onChange,
  printers,
  loading,
  description,
  badge
}) => {
  const { t } = useI18n();
  const isSelectedAvailable = value ? printers.includes(value) : false;

  return (
    <div className="group bg-white rounded-xl border border-gray-200 p-4 hover:border-blue-300 transition-all duration-300 shadow-sm hover:shadow-md">
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-3">
          <div className="p-2.5 bg-blue-50 text-blue-600 rounded-lg group-hover:scale-110 transition-transform duration-300">
            <Icon size={20} />
          </div>
          <div>
            <div className="font-bold text-gray-800 flex items-center gap-2">
              {label}
              {badge}
            </div>
            {description && (
              <p className="text-xs text-gray-500 mt-0.5">{description}</p>
            )}
          </div>
        </div>
      </div>

      <div className="relative">
        {loading ? (
          <div className="w-full border border-gray-100 rounded-xl p-2.5 bg-gray-50 text-gray-400 text-sm flex items-center gap-2">
             <div className="w-4 h-4 border-2 border-gray-200 border-t-blue-500 rounded-full animate-spin" />
             {t('settings.printer.message.loading_printers')}
          </div>
        ) : printers.length === 0 ? (
          <div className="w-full border border-amber-200 rounded-xl p-2.5 bg-amber-50 text-amber-600 text-sm flex items-center gap-2">
            <AlertCircle size={16} /> {t('settings.printer.message.no_printers')}
          </div>
        ) : (
          <>
            <select
              value={value || ''}
              onChange={(e) => onChange(e.target.value || null)}
              className="w-full border border-gray-200 rounded-xl p-2.5 pl-3 pr-10 bg-gray-50 text-sm font-medium text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500 transition-all cursor-pointer hover:bg-white appearance-none"
            >
              <option value="">{t('settings.printer.form.select_printer_placeholder')}</option>
              {printers.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
            <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-gray-400">
              <Settings size={14} />
            </div>
          </>
        )}
      </div>

      {!isSelectedAvailable && value && !loading && (
        <div className="mt-2 text-xs text-red-600 flex items-center gap-1.5 bg-red-50 p-2 rounded-lg border border-red-100 animate-pulse">
          <AlertCircle size={14} />
          {t('settings.printer.message.printer_unavailable')}
        </div>
      )}
    </div>
  );
};
