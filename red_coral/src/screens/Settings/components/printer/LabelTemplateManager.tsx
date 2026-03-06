import React, { useEffect, useState } from 'react';
import { Plus, Copy, Trash2, Edit2, Loader2, CircleDot, Circle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useActiveLabelTemplateId, usePrinterActions } from '@/core/stores/ui';
import {
  useLabelTemplateStore,
  useLabelTemplates,
  useLabelTemplatesLoading,
} from '@/core/stores/printer';
import type { LabelTemplate } from '@/core/domain/types/print';
import { LabelEditorScreen } from '../LabelEditorScreen';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { toast } from '@/presentation/components/Toast';
import { MAX_NAME_LEN } from '@/shared/constants/validation';

export const LabelTemplateManager: React.FC = () => {
  const { t } = useI18n();
  const templates = useLabelTemplates();
  const isLoading = useLabelTemplatesLoading();
  const { createTemplate, updateTemplate, deleteTemplate, duplicateTemplate, ensureDefaultTemplate } =
    useLabelTemplateStore();
  const isLoaded = useLabelTemplateStore((state) => state.isLoaded);

  const [showNewTemplateDialog, setShowNewTemplateDialog] = useState(false);
  const [templateName, setTemplateName] = useState('');
  const [templateWidth, setTemplateWidth] = useState(40);
  const [templateHeight, setTemplateHeight] = useState(30);

  const activeTemplateId = useActiveLabelTemplateId();
  const { setActiveLabelTemplateId } = usePrinterActions();

  const [confirmDialog, setConfirmDialog] = useState({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => {},
  });

  // Editor state
  const [isEditing, setIsEditing] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<LabelTemplate | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  // Load templates on mount
  useEffect(() => {
    ensureDefaultTemplate().then(() => {
      // Set active template if not set
      const state = useLabelTemplateStore.getState();
      if (!activeTemplateId && state.templates.length > 0) {
        setActiveLabelTemplateId(state.templates[0].id);
      }
    });
  }, []);

  const handleCreateTemplate = async () => {
    if (!templateName.trim()) return;

    setIsSaving(true);
    try {
      const newTemplate = await createTemplate({
        name: templateName,
        width: templateWidth,
        height: templateHeight,
        width_mm: templateWidth,
        height_mm: templateHeight,
        padding: 2,
        is_default: false,
        is_active: true,
        fields: [],
      });

      setShowNewTemplateDialog(false);
      setTemplateName('');
      handleEditTemplate(newTemplate);
    } catch {
      toast.error(t('common.message.error'));
    } finally {
      setIsSaving(false);
    }
  };

  const handleDuplicateTemplate = async (template: LabelTemplate) => {
    try {
      await duplicateTemplate(template);
      toast.success(t('common.message.success'));
    } catch {
      toast.error(t('common.message.error'));
    }
  };

  const handleDeleteTemplate = (template_id: number) => {
    if (templates.length === 1) {
      toast.error(t('settings.printer.alert.delete_last_template'));
      return;
    }

    setConfirmDialog({
      isOpen: true,
      title: t('settings.printer.alert.confirm_delete'),
      description: t('settings.printer.alert.confirm_delete_desc'),
      onConfirm: async () => {
        try {
          await deleteTemplate(template_id);
          setConfirmDialog((prev) => ({ ...prev, isOpen: false }));

          // If deleted template was active, select another one
          if (activeTemplateId === template_id) {
            const remaining = templates.filter((t) => t.id !== template_id);
            if (remaining.length > 0) {
              setActiveLabelTemplateId(remaining[0].id);
            }
          }
        } catch {
          toast.error(t('common.message.error'));
        }
      },
    });
  };

  const handleEditTemplate = (template: LabelTemplate) => {
    setEditingTemplate(template);
    setIsEditing(true);
  };

  const handleSaveEditor = async (updatedTemplate: LabelTemplate) => {
    try {
      const saved = await updateTemplate(updatedTemplate.id, updatedTemplate);
      setEditingTemplate(saved);
    } catch {
      toast.error(t('common.message.error'));
    }
  };

  const handleCloseEditor = () => {
    setIsEditing(false);
    setEditingTemplate(null);
  };

  if (isEditing && editingTemplate) {
    return (
      <LabelEditorScreen
        template={editingTemplate}
        onSave={handleSaveEditor}
        onClose={handleCloseEditor}
      />
    );
  }

  // Show loading state
  if (isLoading && !isLoaded) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
      </div>
    );
  }

  return (
    <div className="animate-in fade-in duration-300 space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <p className="text-sm text-gray-500">{t('settings.printer.template.active_hint')}</p>
        <button
          onClick={() => setShowNewTemplateDialog(true)}
          className="flex items-center gap-2 px-4 py-2 bg-gray-900 text-white rounded-xl hover:bg-black transition-colors shadow-lg shadow-gray-200 text-sm font-medium"
        >
          <Plus size={16} />
          {t('settings.printer.template.new')}
        </button>
      </div>

      {/* Template list */}
      <div className="space-y-3">
        {templates.map((template) => {
          const isActive = template.id === activeTemplateId;
          const fieldCount = template.fields?.length ?? 0;
          return (
            <div
              key={template.id}
              className={`bg-white rounded-xl border transition-all ${
                isActive
                  ? 'border-blue-300 ring-1 ring-blue-100'
                  : 'border-gray-200 hover:border-gray-300'
              }`}
            >
              <div className="flex items-center gap-4 px-4 py-3">
                {/* Radio-style selection */}
                <button
                  onClick={() => setActiveLabelTemplateId(template.id)}
                  className="shrink-0"
                  title={t('settings.printer.template.set_active')}
                >
                  {isActive ? (
                    <CircleDot size={22} className="text-blue-500" />
                  ) : (
                    <Circle size={22} className="text-gray-300 hover:text-gray-400 transition-colors" />
                  )}
                </button>

                {/* Template info */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className={`font-semibold text-sm ${isActive ? 'text-blue-900' : 'text-gray-800'}`}>
                      {template.name}
                    </span>
                    {isActive && (
                      <span className="text-xs px-1.5 py-0.5 rounded bg-blue-100 text-blue-700 font-medium">
                        {t('settings.printer.template.in_use')}
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-3 mt-0.5">
                    <span className="text-xs text-gray-400">
                      {template.width}mm × {template.height}mm
                    </span>
                    <span className="text-xs text-gray-400">
                      {fieldCount} {t('settings.printer.template.fields')}
                    </span>
                  </div>
                </div>

                {/* Actions */}
                <div className="flex items-center gap-1 shrink-0">
                  <button
                    onClick={() => handleEditTemplate(template)}
                    className="p-2 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
                    title={t('settings.printer.template.edit_design')}
                  >
                    <Edit2 size={16} />
                  </button>
                  <button
                    onClick={() => handleDuplicateTemplate(template)}
                    className="p-2 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
                    title={t('settings.printer.template.duplicate_template')}
                  >
                    <Copy size={16} />
                  </button>
                  <button
                    onClick={() => handleDeleteTemplate(template.id)}
                    className="p-2 text-gray-400 hover:text-primary-600 hover:bg-primary-50 rounded-lg transition-colors"
                    title={t('common.action.delete')}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            </div>
          );
        })}

        {/* New template row */}
        <button
          onClick={() => setShowNewTemplateDialog(true)}
          className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-gray-50 rounded-xl border-2 border-dashed border-gray-200 hover:border-gray-300 hover:bg-gray-100 transition-all text-gray-400 hover:text-gray-500"
        >
          <Plus size={18} />
          <span className="text-sm font-medium">{t('settings.printer.template.create_template')}</span>
        </button>
      </div>

      {/* Create Modal */}
      {showNewTemplateDialog && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 animate-in fade-in">
          <div className="bg-white rounded-2xl p-6 w-full max-w-sm shadow-2xl animate-in zoom-in-95">
            <h3 className="text-lg font-bold text-gray-800 mb-4">
              {t('settings.printer.template.new')}
            </h3>
            <div className="space-y-4">
              <div>
                <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5">
                  {t('settings.printer.template.form.name')}
                </label>
                <input
                  value={templateName}
                  onChange={(e) => setTemplateName(e.target.value)}
                  placeholder={t('settings.printer.template.form.name_placeholder')}
                  maxLength={MAX_NAME_LEN}
                  className="w-full border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500"
                  autoFocus
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5">
                    {t('settings.printer.template.form.width_mm')}
                  </label>
                  <input
                    type="number"
                    value={templateWidth}
                    onChange={(e) => setTemplateWidth(parseFloat(e.target.value) || 40)}
                    className="w-full border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500"
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5">
                    {t('settings.printer.template.form.height_mm')}
                  </label>
                  <input
                    type="number"
                    value={templateHeight}
                    onChange={(e) => setTemplateHeight(parseFloat(e.target.value) || 30)}
                    className="w-full border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500"
                  />
                </div>
              </div>
            </div>
            <div className="flex gap-3 mt-6 pt-4 border-t border-gray-100">
              <button
                onClick={() => setShowNewTemplateDialog(false)}
                className="flex-1 px-4 py-2 text-sm font-medium text-gray-600 hover:bg-gray-100 rounded-xl transition-colors"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleCreateTemplate}
                disabled={!templateName.trim() || isSaving}
                className="flex-1 px-4 py-2 text-sm font-bold bg-blue-600 text-white rounded-xl hover:bg-blue-700 transition-colors shadow-lg shadow-blue-200 disabled:opacity-50 disabled:shadow-none flex items-center justify-center gap-2"
              >
                {isSaving && <Loader2 size={14} className="animate-spin" />}
                {t('common.action.create')}
              </button>
            </div>
          </div>
        </div>
      )}

      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={() => setConfirmDialog((prev) => ({ ...prev, isOpen: false }))}
        confirmText={t('common.action.confirm')}
        cancelText={t('common.action.cancel')}
        variant="danger"
      />
    </div>
  );
};
