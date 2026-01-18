import React from 'react';
import { PackagePlus, X } from 'lucide-react';
import { SpecificationManager } from './SpecificationManager';
import { ProductSpecification } from '@/types';

interface SpecificationManagementModalProps {
  isOpen: boolean;
  onClose: () => void;
  productId: string | null;
  basePrice?: number;
  baseExternalId?: number;
  initialSpecifications?: ProductSpecification[];
  onSpecificationsChange?: (specs?: ProductSpecification[]) => void;
  hasMultiSpec?: boolean;
  onEnableMultiSpec?: (enabled: boolean) => void;
  t: (key: string) => string;
}

export const SpecificationManagementModal: React.FC<SpecificationManagementModalProps> = ({
  isOpen,
  onClose,
  productId,
  basePrice,
  baseExternalId,
  initialSpecifications,
  onSpecificationsChange,
  hasMultiSpec,
  onEnableMultiSpec,
  t,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-100 bg-black/50 backdrop-blur-sm flex items-center justify-center p-0 md:p-4 animate-in fade-in duration-200">
      <div 
        className="bg-white md:rounded-2xl shadow-2xl w-full max-w-5xl flex flex-col h-full md:h-auto md:max-h-[85vh] overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100 bg-gray-50/50 shrink-0">
          <div className="flex items-center gap-4">
            <h3 className="text-lg font-bold text-gray-900">
              {t("settings.specification.manage")}
            </h3>
            {onEnableMultiSpec && (
              <div className="flex items-center gap-2 bg-white px-3 py-1.5 rounded-lg border border-gray-200 shadow-sm">
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={hasMultiSpec}
                    onChange={(e) => onEnableMultiSpec(e.target.checked)}
                    className="sr-only peer"
                  />
                  <div className="w-9 h-5 bg-gray-200 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-blue-600"></div>
                  <span className="ml-2 text-sm font-medium text-gray-700">
                    {hasMultiSpec ? (t('common.enabled')) : (t('common.disabled'))}
                  </span>
                </label>
              </div>
            )}
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-200 rounded-full transition-colors"
          >
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        <div className="flex-1 overflow-hidden relative bg-white flex flex-col">
          {hasMultiSpec ? (
             <SpecificationManager
                productId={productId}
                basePrice={basePrice}
                baseExternalId={baseExternalId}
                initialSpecifications={initialSpecifications}
                onSpecificationsChange={onSpecificationsChange}
                t={t}
             />
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center text-gray-500 p-8">
               <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mb-4">
                  <PackagePlus size={32} className="text-gray-400" />
                </div>
               <p className="text-lg font-medium text-gray-900 mb-2">
                 {t("settings.specification.multiDisabled")}
               </p>
               <p className="text-sm text-gray-500 max-w-md text-center mb-6">
                 {t("settings.specification.enableMultiHint")}
               </p>
               <button
                 onClick={() => onEnableMultiSpec?.(true)}
                 className="px-6 py-2.5 bg-blue-600 text-white font-bold rounded-xl hover:bg-blue-700 transition-colors shadow-lg shadow-blue-500/30"
               >
                 {t("settings.specification.enableMulti")}
               </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
