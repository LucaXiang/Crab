import React from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { SUPPORTED_LABEL_FIELDS } from './constants';

interface FieldHelperDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const FieldHelperDialog: React.FC<FieldHelperDialogProps> = ({ isOpen, onClose }) => {
  const { t } = useI18n();

  if (!isOpen) return null;

  const grouped = SUPPORTED_LABEL_FIELDS.reduce<Record<string, typeof SUPPORTED_LABEL_FIELDS>>((acc, field) => {
    (acc[field.category] ||= []).push(field);
    return acc;
  }, {});

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 backdrop-blur-sm" onClick={onClose}>
      <div className="bg-white rounded-2xl shadow-xl max-w-lg w-full mx-4 max-h-[80vh] overflow-hidden" onClick={e => e.stopPropagation()}>
        <div className="p-4 border-b border-gray-100 flex items-center justify-between">
          <h3 className="text-lg font-bold text-gray-800">{t('settings.supported_fields')}</h3>
          <button onClick={onClose} className="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded transition-colors">
            <X size={18} />
          </button>
        </div>
        <div className="p-4 overflow-y-auto max-h-[60vh]">
          <p className="text-sm text-gray-500 mb-4">{t('settings.label.field_helper_hint')}</p>
          {Object.entries(grouped).map(([category, fields]) => (
            <div key={category} className="mb-4">
              <h4 className="text-xs font-bold text-gray-400 uppercase tracking-wider mb-2">{category}</h4>
              <div className="space-y-1">
                {fields.map(f => (
                  <div key={f.key} className="flex items-center gap-3 p-2 rounded-lg hover:bg-gray-50">
                    <code className="text-xs bg-blue-50 text-blue-700 px-2 py-0.5 rounded font-mono whitespace-nowrap">
                      {`{${f.key}}`}
                    </code>
                    <span className="text-sm text-gray-700 flex-1">{f.label}</span>
                    <span className="text-xs text-gray-400">{f.example}</span>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
