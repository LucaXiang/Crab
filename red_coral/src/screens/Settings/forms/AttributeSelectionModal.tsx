import React from 'react';
import { X } from 'lucide-react';
import { ProductAttributesSection } from './ProductAttributesSection';

interface AttributeSelectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  selectedAttributeIds: string[];
  attributeDefaultOptions?: Record<string, string | string[]>;
  onChange: (attributeIds: string[]) => void;
  onDefaultOptionChange?: (attributeId: string, optionIds: string[]) => void;
  t: (key: string) => string;
  inheritedAttributeIds?: string[];
}

export const AttributeSelectionModal: React.FC<AttributeSelectionModalProps> = ({
  isOpen,
  onClose,
  selectedAttributeIds,
  attributeDefaultOptions,
  onChange,
  onDefaultOptionChange,
  t,
  inheritedAttributeIds,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-100 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div 
        className="bg-white rounded-2xl shadow-2xl w-full max-w-md flex flex-col max-h-[85vh] overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100 bg-gray-50/50 shrink-0">
          <h3 className="text-lg font-bold text-gray-900">
            {t('settings.product.attribute.manage')}
          </h3>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-200 rounded-full transition-colors"
          >
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        <div className="p-6 overflow-y-auto min-h-0 flex-1">
           <ProductAttributesSection
             selectedAttributeIds={selectedAttributeIds}
             attributeDefaultOptions={attributeDefaultOptions}
             onChange={onChange}
             onDefaultOptionChange={onDefaultOptionChange}
             t={t}
             hideHeader={true}
             inheritedAttributeIds={inheritedAttributeIds}
           />
        </div>

        <div className="px-6 py-4 border-t border-gray-100 bg-gray-50/50 flex justify-end shrink-0">
          <button
            onClick={onClose}
            className="px-6 py-2 bg-teal-600 text-white rounded-xl text-sm font-bold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20"
          >
            {t('common.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};
