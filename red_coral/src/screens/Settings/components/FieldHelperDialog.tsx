import React from 'react';
import { X, Copy } from 'lucide-react';
import { SUPPORTED_LABEL_FIELDS } from '../../../types/labelTemplate';
import { useI18n } from '../../../hooks/useI18n';

interface FieldHelperDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectField?: (fieldKey: string) => void;
}

export const FieldHelperDialog: React.FC<FieldHelperDialogProps> = ({
  isOpen,
  onClose,
  onSelectField
}) => {
  const { t } = useI18n();

  if (!isOpen) return null;

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    // Could add toast here
  };

  return (
    <div
      className="fixed inset-0 z-60 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm animate-fade-in"
      onClick={handleBackdropClick}
    >
      <div className="bg-white rounded-xl shadow-2xl w-full max-w-3xl overflow-hidden animate-scale-in border border-gray-100 flex flex-col max-h-[85vh]">
        <div className="p-4 border-b border-gray-100 flex items-center justify-between bg-gray-50/50">
          <h3 className="font-bold text-gray-800 text-lg">
            {t('settings.supportedFields')}
          </h3>
          <button
            onClick={onClose}
            className="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-0">
          <table className="w-full text-left text-sm">
            <thead className="bg-gray-50 text-gray-600 font-semibold sticky top-0 z-10 shadow-sm">
              <tr>
                <th className="p-4 pb-3 w-32">{t("settings.label.fieldKey")}</th>
                <th className="p-4 pb-3">{t("settings.label.description")}</th>
                <th className="p-4 pb-3">{t("settings.label.example")}</th>
                <th className="p-4 pb-3 w-16"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {SUPPORTED_LABEL_FIELDS.map((field) => (
                <tr 
                  key={field.key} 
                  className="hover:bg-blue-50/50 transition-colors group"
                >
                  <td className="p-4 py-3 font-mono text-blue-600 font-medium">
                    {`{${field.key}}`}
                  </td>
                  <td className="p-4 py-3 text-gray-700">
                    <span className="inline-block px-2 py-0.5 rounded text-[10px] bg-gray-100 text-gray-500 font-medium mr-2 uppercase tracking-wide">
                      {field.category}
                    </span>
                    {field.description}
                  </td>
                  <td className="p-4 py-3 text-gray-500 italic">
                    {field.example}
                  </td>
                  <td className="p-4 py-3 text-right">
                    <button
                      onClick={() => handleCopy(`{${field.key}}`)}
                      className="p-1.5 text-gray-300 hover:text-blue-500 hover:bg-blue-50 rounded transition-colors opacity-0 group-hover:opacity-100"
                      title="Copy to clipboard"
                    >
                      <Copy size={16} />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="p-4 border-t border-gray-100 bg-gray-50 text-xs text-gray-500 text-center">
           {t("settings.label.fieldHelperHint")}
        </div>
      </div>
    </div>
  );
};
