import React, { useEffect, useState } from 'react';
import { X, Plus, Edit, Trash2, Star, List } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { useConfirmDialog } from '@/shared/hooks/useConfirmDialog';
import { formatCurrency } from '@/utils/currency';
import { getErrorMessage } from '@/utils/error';
import { logger } from '@/utils/logger';
import { SpecificationFormModal } from './SpecificationFormModal';
import { canDeleteSpec, setDefaultSpec } from './spec-utils';
import { useProductStore } from './store';
import type { ProductSpec, ProductSpecInput } from '@/core/domain/types';

const getApi = () => createTauriClient();

interface SpecificationManagementModalProps {
  isOpen: boolean;
  onClose: () => void;
  productId: number;
  productName: string;
}

export const SpecificationManagementModal: React.FC<SpecificationManagementModalProps> = React.memo(({
  isOpen,
  onClose,
  productId,
  productName,
}) => {
  const { t } = useI18n();

  // Get specs directly from store (no API call needed)
  const product = useProductStore((state) => state.items.find((p) => p.id === productId));
  const [specs, setSpecs] = useState<ProductSpec[]>([]);
  const [isSaving, setIsSaving] = useState(false);

  // Form modal state
  const [formOpen, setFormOpen] = useState(false);
  const [editingSpec, setEditingSpec] = useState<ProductSpec | null>(null);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);

  // Confirm dialog state
  const confirmDialog = useConfirmDialog();

  // Sync specs from store when modal opens or product changes
  useEffect(() => {
    if (isOpen && product) {
      setSpecs(product.specs || []);
    }
  }, [isOpen, product]);

  const saveSpecs = async (newSpecs: ProductSpec[]) => {
    setIsSaving(true);
    try {
      const specInputs: ProductSpecInput[] = newSpecs.map(({ name, price, display_order, is_default, is_active, is_root, receipt_name }) => ({
        name, price, display_order, is_default, is_active, is_root, receipt_name,
      }));
      const updated = await getApi().updateProduct(productId, { specs: specInputs });
      setSpecs(newSpecs);

      // Optimistic update - sync mechanism will also update via broadcast
      // This ensures immediate UI update without waiting for sync
      if (updated) {
        useProductStore.getState().optimisticUpdate(productId, (prev) => ({
          ...prev,
          specs: updated.specs,
        }));
      }

      return true;
    } catch (error) {
      logger.error('Failed to save specs', error);
      toast.error(getErrorMessage(error));
      return false;
    } finally {
      setIsSaving(false);
    }
  };

  const handleAddSpec = () => {
    setEditingSpec(null);
    setEditingIndex(null);
    setFormOpen(true);
  };

  const handleEditSpec = (spec: ProductSpec, index: number) => {
    setEditingSpec(spec);
    setEditingIndex(index);
    setFormOpen(true);
  };

  const handleDeleteSpec = (spec: ProductSpec, index: number) => {
    if (!canDeleteSpec(spec)) {
      toast.error(t('settings.specification.message.root_cannot_delete'));
      return;
    }

    confirmDialog.show(
      t('settings.specification.action.delete'),
      t('settings.specification.confirm.delete'),
      async () => {
        confirmDialog.close();
        const newSpecs = specs.filter((_, i) => i !== index);
        const success = await saveSpecs(newSpecs);
        if (success) {
          toast.success(t('settings.specification.message.deleted'));
        }
      },
    );
  };

  const handleToggleDefault = async (index: number) => {
    const spec = specs[index];
    const isCurrentlyDefault = spec.is_default;

    // If currently default, clear it; otherwise set as default
    const newSpecs = setDefaultSpec(specs, isCurrentlyDefault ? null : index);
    const success = await saveSpecs(newSpecs);
    if (success) {
      toast.success(t('settings.specification.message.updated'));
    }
  };

  const handleSaveSpec = async (spec: ProductSpec, index: number | null) => {
    let newSpecs: ProductSpec[];

    if (index === null) {
      // Create new spec
      newSpecs = [...specs, { ...spec, display_order: specs.length }];
    } else {
      // Update existing spec
      newSpecs = specs.map((s, i) => (i === index ? spec : s));
    }

    // If new spec is default, clear other defaults
    if (spec.is_default) {
      const targetIndex = index === null ? newSpecs.length - 1 : index;
      newSpecs = setDefaultSpec(newSpecs, targetIndex);
    }

    const success = await saveSpecs(newSpecs);
    if (success) {
      toast.success(index === null ? t('settings.specification.message.created') : t('settings.specification.message.updated'));
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={onClose}
    >
      <div
        className="bg-gray-50 rounded-2xl shadow-2xl w-full max-w-2xl overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-gray-200 bg-white">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-bold text-gray-900">
                {t('settings.specification.manage')}
              </h2>
              <p className="text-sm text-gray-500 mt-0.5">{productName}</p>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={handleAddSpec}
                disabled={isSaving}
                className="inline-flex items-center gap-1.5 px-3 py-2 bg-orange-500 text-white rounded-xl text-sm font-semibold hover:bg-orange-600 transition-colors shadow-lg shadow-orange-500/20 disabled:opacity-50"
              >
                <Plus size={16} />
                <span>{t('settings.specification.add_new')}</span>
              </button>
              <button
                onClick={onClose}
                className="p-2 hover:bg-gray-100 rounded-xl transition-colors"
              >
                <X size={18} className="text-gray-500" />
              </button>
            </div>
          </div>
        </div>

        {/* Content */}
        <div className="p-4 max-h-[60vh] overflow-y-auto">
          {specs.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mb-4">
                <List className="text-gray-300" size={32} />
              </div>
              <p className="text-gray-500 font-medium">{t('settings.specification.no_specs')}</p>
              <button
                onClick={handleAddSpec}
                className="mt-3 text-orange-600 hover:text-orange-700 font-medium text-sm hover:underline"
              >
                {t('settings.specification.create_first')}
              </button>
            </div>
          ) : (
            <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
              {/* Table Header */}
              <div className="grid grid-cols-10 gap-2 px-4 py-3 bg-gray-50 border-b border-gray-200 text-xs font-semibold text-gray-500 uppercase tracking-wider">
                <div className="col-span-3">{t('settings.specification.form.name')}</div>
                <div className="col-span-2">{t('settings.specification.form.receipt_name')}</div>
                <div className="col-span-2 text-right">{t('settings.specification.form.price')}</div>
                <div className="col-span-1 text-center">{t('settings.specification.label.default')}</div>
                <div className="col-span-2 text-right">{t('common.action.actions')}</div>
              </div>

              {/* Table Body */}
              <div className="divide-y divide-gray-100">
                {specs.map((spec, index) => (
                  <div
                    key={index}
                    className={`grid grid-cols-10 gap-2 px-4 py-3 items-center hover:bg-gray-50 transition-colors group ${
                      spec.is_root ? 'bg-amber-50/50' : ''
                    }`}
                  >
                    {/* Name */}
                    <div className="col-span-3 flex items-center gap-2">
                      <span className="font-medium text-gray-900 truncate">{spec.name}</span>
                      {spec.is_root && (
                        <span className="px-1.5 py-0.5 text-[0.625rem] font-medium bg-amber-100 text-amber-700 rounded">
                          {t('settings.specification.label.root')}
                        </span>
                      )}
                    </div>

                    {/* Receipt Name */}
                    <div className="col-span-2 text-sm text-gray-500 truncate">
                      {spec.receipt_name || '-'}
                    </div>

                    {/* Price */}
                    <div className="col-span-2 text-right font-mono text-sm text-gray-700">
                      {formatCurrency(spec.price)}
                    </div>

                    {/* Default Toggle */}
                    <div className="col-span-1 flex justify-center">
                      <button
                        onClick={() => handleToggleDefault(index)}
                        disabled={isSaving}
                        className={`p-1.5 rounded-lg transition-colors ${
                          spec.is_default
                            ? 'bg-orange-100 text-orange-600'
                            : 'text-gray-300 hover:text-orange-500 hover:bg-orange-50'
                        }`}
                        title={spec.is_default ? t('settings.specification.label.cancel_default') : t('settings.specification.label.set_default')}
                      >
                        <Star size={16} className={spec.is_default ? 'fill-current' : ''} />
                      </button>
                    </div>

                    {/* Actions */}
                    <div className="col-span-2 flex items-center justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
                        onClick={() => handleEditSpec(spec, index)}
                        disabled={isSaving}
                        className="p-1.5 text-gray-400 hover:text-orange-600 hover:bg-orange-50 rounded-lg transition-colors"
                        title={t('common.action.edit')}
                      >
                        <Edit size={14} />
                      </button>
                      {canDeleteSpec(spec) && (
                        <button
                          onClick={() => handleDeleteSpec(spec, index)}
                          disabled={isSaving}
                          className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                          title={t('common.action.delete')}
                        >
                          <Trash2 size={14} />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-200 bg-white flex justify-end">
          <button
            onClick={onClose}
            className="px-5 py-2.5 bg-white border border-gray-200 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-50 transition-colors"
          >
            {t('common.action.close')}
          </button>
        </div>
      </div>

      {/* Spec Form Modal */}
      {formOpen && (
        <SpecificationFormModal
          isOpen={formOpen}
          onClose={() => {
            setFormOpen(false);
            setEditingSpec(null);
            setEditingIndex(null);
          }}
          spec={editingSpec}
          specIndex={editingIndex}
          isRootSpec={editingSpec?.is_root ?? false}
          onSave={handleSaveSpec}
        />
      )}

      {/* Confirm Dialog */}
      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={confirmDialog.close}
      />
    </div>
  );
});
