import React, { useState } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useSettingsModal,
  useSettingsFormMeta,
} from '@/core/stores/settings/useSettingsStore';
import { useZones } from '@/core/stores/resources';
import { toast } from '@/presentation/components/Toast';
import { getErrorMessage } from '@/utils/error';
import { TableForm } from './TableForm';
import { createTable, updateTable, deleteTable } from './mutations';
import { DeleteConfirmation } from '@/shared/components/DeleteConfirmation';

export const TableModal: React.FC = React.memo(() => {
  const { t } = useI18n();
  const { modal, closeModal } = useSettingsModal();
  const { formData, setFormField, isFormDirty, formErrors } = useSettingsFormMeta();

  const zones = useZones();
  const [unsavedDialogOpen, setUnsavedDialogOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // Only render if modal is open and entity is TABLE
  if (!modal.open || modal.entity !== 'TABLE') return null;

  const { action, data } = modal;

  const getTitle = () => {
    const titles: Record<string, string> = {
      CREATE: t('settings.table.add_table'),
      EDIT: t('settings.table.edit_table'),
      DELETE: t('settings.table.delete_table'),
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
      await deleteTable(String(data.id));
      toast.success(t('settings.table.table_deleted'));
      closeModal();
    } catch (e) {
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
      const tablePayload = {
        name: formData.name.trim(),
        zone: formData.zone,
        capacity: Math.max(1, formData.capacity ?? 1),
        is_active: formData.is_active ?? true,
      };

      if (action === 'CREATE') {
        await createTable({
          name: tablePayload.name,
          zone: String(tablePayload.zone),
          capacity: Number(tablePayload.capacity),
        });
        toast.success(t('settings.table.message.created'));
      } else if (data?.id) {
        await updateTable(String(data.id), {
          name: tablePayload.name,
          zone: String(tablePayload.zone),
          capacity: Number(tablePayload.capacity),
          is_active: tablePayload.is_active,
        });
        toast.success(t('settings.table.message.updated'));
      }
      closeModal();
    } catch (e) {
      toast.error(getErrorMessage(e));
    } finally {
      setIsSaving(false);
    }
  };

  const accent = 'blue';
  const hasError = Object.values(formErrors).some(Boolean);
  const isSaveDisabled = !isFormDirty || hasError || isSaving;
  const saveEnabledClass = `px-5 py-2.5 bg-${accent}-600 text-white rounded-xl text-sm font-semibold hover:bg-${accent}-700 transition-colors shadow-lg shadow-${accent}-600/20`;
  const saveDisabledClass = 'px-5 py-2.5 bg-gray-200 text-gray-400 rounded-xl text-sm font-semibold cursor-not-allowed';

  const renderFormContent = () => {
    if (action === 'DELETE') {
      return <DeleteConfirmation name={data?.name} entity="TABLE" t={t} />;
    }

    return (
      <TableForm
        formData={{
          name: formData.name,
          capacity: formData.capacity ?? 1,
          zone: formData.zone ?? '',
          is_active: formData.is_active ?? true,
        }}
        zones={zones}
        onFieldChange={setFormField}
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
