import React, { useEffect, useState } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useSettingsModal,
  useSettingsFormMeta,
  useSettingsStore,
} from '@/core/stores/settings/useSettingsStore';
import { toast } from '@/presentation/components/Toast';
import { getErrorMessage } from '@/utils/error';
import { CategoryForm } from './CategoryForm';
import { createCategory, updateCategory, deleteCategory, loadCategoryAttributes } from './mutations';
import { DeleteConfirmation } from '@/shared/components/DeleteConfirmation';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission } from '@/core/domain/types';

/**
 * CategoryModal - Modal for creating, editing, and deleting categories
 *
 * This component is specifically for category CRUD operations.
 * It uses the shared settings modal state from useSettingsModal.
 */
export const CategoryModal: React.FC = React.memo(() => {
  const { t } = useI18n();
  const { modal, closeModal } = useSettingsModal();
  const { formData, setFormField, setAsyncFormData, isFormDirty, formErrors } = useSettingsFormMeta();

  const refreshData = useSettingsStore((s) => s.refreshData);

  const [unsavedDialogOpen, setUnsavedDialogOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // Only render if modal is for CATEGORY entity
  const isCategory = modal.open && modal.entity === 'CATEGORY';

  // Load category attributes when editing
  // NOTE: modal.open is needed as dependency to reload data when reopening the same category
  useEffect(() => {
    const categoryId = modal.data?.id;
    if (isCategory && modal.action === 'EDIT' && categoryId) {
      const loadAttributes = async () => {
        try {
          const { attributeIds, defaultOptions } = await loadCategoryAttributes(String(categoryId));
          setAsyncFormData({
            selected_attribute_ids: attributeIds,
            attribute_default_options: defaultOptions
          });
        } catch {
          // Silently fail - attributes will be empty
        }
      };
      loadAttributes();
    }
  }, [isCategory, modal.open, modal.action, modal.data?.id, setAsyncFormData]);

  if (!isCategory) return null;

  const { action, data } = modal;

  const getTitle = () => {
    const titles: Record<string, string> = {
      CREATE: t('settings.category.add_category'),
      EDIT: t('settings.category.edit_category'),
      DELETE: t('settings.category.delete_category'),
    };
    return titles[action] || '';
  };

  const handleClose = () => {
    if (isFormDirty) {
      setUnsavedDialogOpen(true);
      return;
    }
    closeModal();
  };

  const handleConfirmDiscard = () => {
    setUnsavedDialogOpen(false);
    closeModal();
  };

  const handleCancelDiscard = () => {
    setUnsavedDialogOpen(false);
  };

  const handleDelete = async () => {
    if (!data?.id) return;
    try {
      await deleteCategory(String(data.id));
      toast.success(t('settings.category.category_deleted'));
      closeModal();
    } catch (e: unknown) {
      toast.error(getErrorMessage(e));
    }
  };

  const handleSave = async () => {
    if (isSaving) return;

    const hasError = Object.values(formErrors).some(Boolean);
    if (hasError) {
      toast.error(t('common.message.invalid_form'));
      return;
    }

    setIsSaving(true);
    try {
      if (action === 'CREATE') {
        await createCategory({
          name: formData.name,
          print_destinations: formData.print_destinations,
          is_kitchen_print_enabled: formData.is_kitchen_print_enabled,
          is_label_print_enabled: formData.is_label_print_enabled,
          is_active: formData.is_active,
          selected_attribute_ids: formData.selected_attribute_ids,
          attribute_default_options: formData.attribute_default_options,
          is_virtual: formData.is_virtual,
          tag_ids: formData.tag_ids,
          match_mode: formData.match_mode,
          sort_order: formData.sort_order,
        });
        refreshData();
        toast.success(t('settings.category.create_category_success'));
      } else if (action === 'EDIT' && data?.id) {
        await updateCategory(String(data.id), {
          name: formData.name,
          print_destinations: formData.print_destinations,
          is_kitchen_print_enabled: formData.is_kitchen_print_enabled,
          is_label_print_enabled: formData.is_label_print_enabled,
          is_active: formData.is_active,
          selected_attribute_ids: formData.selected_attribute_ids,
          attribute_default_options: formData.attribute_default_options,
          is_virtual: formData.is_virtual,
          tag_ids: formData.tag_ids,
          match_mode: formData.match_mode,
          sort_order: formData.sort_order,
        });
        refreshData();
        toast.success(t('settings.category.update_category_success'));
      }
      closeModal();
    } catch (e: unknown) {
      toast.error(getErrorMessage(e));
    } finally {
      setIsSaving(false);
    }
  };

  const accent = 'teal';
  const hasError = Object.values(formErrors).some(Boolean);
  const isSaveDisabled = !isFormDirty || hasError || isSaving;
  const saveEnabledClass = `px-5 py-2.5 bg-${accent}-600 text-white rounded-xl text-sm font-semibold hover:bg-${accent}-700 transition-colors shadow-lg shadow-${accent}-600/20`;
  const saveDisabledClass = 'px-5 py-2.5 bg-gray-200 text-gray-400 rounded-xl text-sm font-semibold cursor-not-allowed';

  const renderFormContent = () => {
    if (action === 'DELETE') {
      return <DeleteConfirmation name={data?.name} entity="CATEGORY" t={t} />;
    }

    return (
      <CategoryForm
        formData={{
          name: formData.name,
          print_destinations: formData.print_destinations,
          is_kitchen_print_enabled: formData.is_kitchen_print_enabled,
          is_label_print_enabled: formData.is_label_print_enabled,
          is_active: formData.is_active ?? true,
          selected_attribute_ids: formData.selected_attribute_ids,
          attribute_default_options: formData.attribute_default_options,
          is_virtual: formData.is_virtual,
          tag_ids: formData.tag_ids,
          match_mode: formData.match_mode,
        }}
        onFieldChange={setFormField}
        t={t}
        isEditMode={action === 'EDIT'}
      />
    );
  };

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div
        className="bg-white rounded-2xl shadow-2xl w-full max-w-lg flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className={`shrink-0 px-6 py-4 border-b border-gray-100 bg-linear-to-r from-${accent}-50 to-white`}>
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-gray-900">{getTitle()}</h2>
            <button
              onClick={handleClose}
              className="p-2 hover:bg-gray-100 rounded-xl transition-colors"
            >
              <X size={18} className="text-gray-500" />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto min-h-0 flex-1">
          <div className="space-y-4">
            {renderFormContent()}
          </div>
        </div>

        {/* Footer */}
        <div className="shrink-0 px-6 py-4 border-t border-gray-100 bg-gray-50/50 flex justify-end gap-3">
          <button
            onClick={handleClose}
            className="px-5 py-2.5 bg-white border border-gray-200 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-50 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          {action === 'DELETE' ? (
            <ProtectedGate permission={Permission.CATEGORIES_MANAGE}>
              <button
                onClick={handleDelete}
                className="px-5 py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
              >
                {t('common.action.delete')}
              </button>
            </ProtectedGate>
          ) : (
            <button
              onClick={handleSave}
              disabled={isSaveDisabled}
              className={isSaveDisabled ? saveDisabledClass : saveEnabledClass}
            >
              {action === 'CREATE' ? t('common.action.create') : t('common.action.save')}
            </button>
          )}
        </div>
      </div>

      {/* Unsaved Changes Dialog */}
      {unsavedDialogOpen && (
        <div className="fixed inset-0 z-90 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-sm w-full overflow-hidden animate-in zoom-in-95">
            <div className="p-6">
              <h3 className="text-lg font-bold text-gray-900 mb-2">{t('settings.unsaved_confirm')}</h3>
              <p className="text-sm text-gray-600 mb-6">{t('settings.unsaved_confirm_hint')}</p>
              <div className="grid grid-cols-2 gap-3">
                <button
                  onClick={handleCancelDiscard}
                  className="w-full py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-200 transition-colors"
                >
                  {t('common.action.cancel')}
                </button>
                <button
                  onClick={handleConfirmDiscard}
                  className="w-full py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
                >
                  {t('common.action.confirm')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
});
