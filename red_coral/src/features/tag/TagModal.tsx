import React, { useState } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useSettingsModal,
  useSettingsFormMeta,
  useSettingsStore,
} from '@/core/stores/settings/useSettingsStore';
import { toast } from '@/presentation/components/Toast';
import { getErrorMessage } from '@/utils/error';
import { createTag, updateTag, deleteTag } from './mutations';
import { TagForm } from './TagForm';
import { DeleteConfirmation } from '@/shared/components/DeleteConfirmation';

export const TagModal: React.FC = React.memo(() => {
  const { t } = useI18n();
  const { modal, closeModal } = useSettingsModal();
  const { formData, setFormField, isFormDirty, formErrors } = useSettingsFormMeta();

  const refreshData = useSettingsStore((s) => s.refreshData);

  const [unsavedDialogOpen, setUnsavedDialogOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // Only render for TAG entity
  if (!modal.open || modal.entity !== 'TAG') return null;

  const { action, data } = modal;

  const getTitle = () => {
    const titles: Record<string, string> = {
      CREATE: t('settings.tag.add_tag'),
      EDIT: t('settings.tag.edit_tag'),
      DELETE: t('settings.tag.delete_tag'),
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
    try {
      await deleteTag(String(data.id));
      refreshData();
      toast.success(t('settings.tag.tag_deleted'));
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
      const tagPayload = {
        name: formData.name.trim(),
        color: formData.color || '#3B82F6',
        display_order: formData.display_order ?? 0,
        is_active: formData.is_active ?? true,
      };

      if (action === 'CREATE') {
        await createTag({
          name: tagPayload.name,
          color: tagPayload.color,
          display_order: tagPayload.display_order,
        });
        refreshData();
        toast.success(t('settings.tag.message.created'));
      } else {
        await updateTag(String(data.id), {
          name: tagPayload.name,
          color: tagPayload.color,
          display_order: tagPayload.display_order,
          is_active: tagPayload.is_active,
        });
        refreshData();
        toast.success(t('settings.tag.message.updated'));
      }
      closeModal();
    } catch (e: unknown) {
      toast.error(getErrorMessage(e));
    } finally {
      setIsSaving(false);
    }
  };

  const accent = 'indigo';
  const hasError = Object.values(formErrors).some(Boolean);
  const isSaveDisabled = !isFormDirty || hasError || isSaving;
  const saveEnabledClass = `px-5 py-2.5 bg-${accent}-600 text-white rounded-xl text-sm font-semibold hover:bg-${accent}-700 transition-colors shadow-lg shadow-${accent}-600/20`;
  const saveDisabledClass = 'px-5 py-2.5 bg-gray-200 text-gray-400 rounded-xl text-sm font-semibold cursor-not-allowed';

  const renderFormContent = () => {
    if (action === 'DELETE') {
      return <DeleteConfirmation name={data?.name} entity="TAG" t={t} />;
    }

    return (
      <TagForm
        formData={{
          name: formData.name,
          color: formData.color || '#3B82F6',
          display_order: formData.display_order ?? 0,
          is_active: formData.is_active ?? true,
        }}
        onFieldChange={setFormField as (field: string, value: any) => void}
        t={t}
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
            <button
              onClick={handleDelete}
              className="px-5 py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
            >
              {t('common.action.delete')}
            </button>
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
