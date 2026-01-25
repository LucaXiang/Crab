import React, { useEffect, useState } from 'react';
import { LayoutTemplate, Plus, Copy, Trash2, Edit2, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useActiveLabelTemplateId, usePrinterActions } from '@/core/stores/ui';
import { LabelTemplate, DEFAULT_LABEL_TEMPLATES } from '@/core/domain/types/print';
import { LabelEditorScreen } from '../LabelEditorScreen';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { toast } from '@/presentation/components/Toast';

// TODO: label_templates 应该迁移到服务端存储，实现多设备共享模板
const STORAGE_KEY = 'label_templates';

export const LabelTemplateManager: React.FC = () => {
  const { t } = useI18n();
  const [templates, setTemplates] = useState<LabelTemplate[]>([]);
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

  // Load templates
  useEffect(() => {
    const storedTemplates = localStorage.getItem(STORAGE_KEY);
    if (storedTemplates) {
      try {
        setTemplates(JSON.parse(storedTemplates));
      } catch (e) {
        console.error('Failed to load templates:', e);
      }
    }

    if (!storedTemplates || JSON.parse(storedTemplates).length === 0) {
      const defaultTemplate: LabelTemplate = {
        ...DEFAULT_LABEL_TEMPLATES[0],
        id: `template_${Date.now()}`,
        isDefault: false,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      setTemplates([defaultTemplate]);
      saveTemplatesToStorage([defaultTemplate]);
      if (!activeTemplateId) {
        setActiveLabelTemplateId(defaultTemplate.id);
      }
    } else if (!activeTemplateId && templates.length > 0) {
       const parsed = JSON.parse(storedTemplates);
       if (parsed.length > 0) setActiveLabelTemplateId(parsed[0].id);
    }
  }, []);

  const saveTemplatesToStorage = (templatesToSave: LabelTemplate[]) => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(templatesToSave));
  };

  const generateId = () => `template_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`;

  const handleCreateTemplate = () => {
    if (!templateName.trim()) return;

    const newTemplate: LabelTemplate = {
      id: generateId(),
      name: templateName,
      width: templateWidth,
      height: templateHeight,
      widthMm: templateWidth,
      heightMm: templateHeight,
      padding: 2,
      isDefault: false,
      isActive: true,
      fields: [],
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    const updatedTemplates = [...templates, newTemplate];
    setTemplates(updatedTemplates);
    saveTemplatesToStorage(updatedTemplates);
    setShowNewTemplateDialog(false);
    setTemplateName('');

    handleEditTemplate(newTemplate);
  };

  const handleDuplicateTemplate = (template: LabelTemplate) => {
    const duplicated: LabelTemplate = {
      ...template,
      id: generateId(),
      name: `${template.name} (Copy)`,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
    const updatedTemplates = [...templates, duplicated];
    setTemplates(updatedTemplates);
    saveTemplatesToStorage(updatedTemplates);
  };

  const handleDeleteTemplate = (templateId: string) => {
    if (templates.length === 1) {
      toast.error(t('settings.printer.alert.delete_last_template'));
      return;
    }

    setConfirmDialog({
      isOpen: true,
      title: t('settings.printer.alert.confirm_delete'),
      description: t('settings.printer.alert.confirm_delete_desc'),
      onConfirm: () => {
        const updatedTemplates = templates.filter((tmpl) => tmpl.id !== templateId);
        setTemplates(updatedTemplates);
        saveTemplatesToStorage(updatedTemplates);
        setConfirmDialog(prev => ({ ...prev, isOpen: false }));
      }
    });
  };

  const handleEditTemplate = (template: LabelTemplate) => {
    setEditingTemplate(template);
    setIsEditing(true);
  };

  const handleSaveEditor = (updatedTemplate: LabelTemplate) => {
    const updatedTemplates = templates.map((tmpl) =>
      tmpl.id === updatedTemplate.id ? { ...updatedTemplate, updatedAt: new Date().toISOString() } : tmpl
    );
    setTemplates(updatedTemplates);
    saveTemplatesToStorage(updatedTemplates);
    setEditingTemplate(updatedTemplate);
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

  return (
    <div className="animate-in fade-in duration-300">
      <div className="flex justify-end items-center mb-6">
        <button
          onClick={() => setShowNewTemplateDialog(true)}
          className="flex items-center gap-2 px-4 py-2 bg-gray-900 text-white rounded-xl hover:bg-black transition-colors shadow-lg shadow-gray-200"
        >
          <Plus size={18} />
          {t('settings.printer.template.new')}
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
        {templates.map((template) => {
          const isActive = template.id === activeTemplateId;
          return (
          <div
            key={template.id}
            onClick={() => setActiveLabelTemplateId(template.id)}
            className={`group bg-white rounded-2xl border p-5 transition-all duration-300 flex flex-col cursor-pointer relative ${
              isActive
                ? 'border-blue-500 shadow-md ring-2 ring-blue-100'
                : 'border-gray-200 hover:shadow-lg hover:border-blue-200'
            }`}
          >
            {isActive && (
              <div className="absolute top-4 right-4 bg-blue-500 text-white p-1 rounded-full shadow-sm">
                <Check size={14} strokeWidth={3} />
              </div>
            )}

            <div className="flex justify-between items-start mb-4">
              <div className={`p-3 rounded-xl transition-colors ${
                isActive ? 'bg-blue-100 text-blue-700' : 'bg-blue-50 text-blue-600 group-hover:bg-blue-600 group-hover:text-white'
              }`}>
                <LayoutTemplate size={24} />
              </div>
              <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDuplicateTemplate(template);
                  }}
                  className="p-2 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
                  title={t('settings.printer.template.duplicate_template')}
                >
                  <Copy size={16} />
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDeleteTemplate(template.id);
                  }}
                  className="p-2 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                  title={t('common.action.delete')}
                >
                  <Trash2 size={16} />
                </button>
              </div>
            </div>

            <h4 className={`font-bold mb-1 ${isActive ? 'text-blue-900' : 'text-gray-800'}`}>{template.name}</h4>
            <p className="text-sm text-gray-500 mb-6 flex items-center gap-2">
              <span className={`w-1.5 h-1.5 rounded-full ${isActive ? 'bg-blue-400' : 'bg-gray-300'}`}></span>
              {template.width}mm × {template.height}mm
            </p>

            <button
              onClick={(e) => {
                e.stopPropagation();
                handleEditTemplate(template);
              }}
              className={`mt-auto w-full py-2.5 border font-medium rounded-xl transition-all flex items-center justify-center gap-2 ${
                isActive
                  ? 'bg-blue-50 border-blue-200 text-blue-700 hover:bg-blue-100'
                  : 'border-gray-200 text-gray-700 hover:bg-blue-600 hover:border-blue-600 hover:text-white'
              }`}
            >
              <Edit2 size={16} />
              {t('settings.printer.template.edit_design')}
            </button>
          </div>
        );
        })}

        {/* New Template Card */}
        <button
          onClick={() => setShowNewTemplateDialog(true)}
          className="bg-gray-50 rounded-2xl border-2 border-dashed border-gray-200 p-5 hover:bg-gray-100 hover:border-gray-300 transition-all flex flex-col items-center justify-center text-gray-400 gap-3 min-h-[12.5rem]"
        >
          <div className="w-12 h-12 rounded-full bg-white flex items-center justify-center shadow-sm">
            <Plus size={24} />
          </div>
          <span className="font-medium">{t('settings.printer.template.create_template')}</span>
        </button>
      </div>

      {/* Create Modal */}
      {showNewTemplateDialog && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 animate-in fade-in">
          <div className="bg-white rounded-2xl p-6 w-full max-w-sm shadow-2xl animate-in zoom-in-95">
            <h3 className="text-lg font-bold text-gray-800 mb-4">{t('settings.printer.template.new')}</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5">{t('settings.printer.template.form.name')}</label>
                <input
                  value={templateName}
                  onChange={(e) => setTemplateName(e.target.value)}
                  placeholder={t('settings.printer.template.form.name_placeholder')}
                  className="w-full border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500"
                  autoFocus
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5">{t('settings.printer.template.form.width_mm')}</label>
                  <input
                    type="number"
                    value={templateWidth}
                    onChange={(e) => setTemplateWidth(parseFloat(e.target.value) || 40)}
                    className="w-full border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500"
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5">{t('settings.printer.template.form.height_mm')}</label>
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
                disabled={!templateName.trim()}
                className="flex-1 px-4 py-2 text-sm font-bold bg-blue-600 text-white rounded-xl hover:bg-blue-700 transition-colors shadow-lg shadow-blue-200 disabled:opacity-50 disabled:shadow-none"
              >
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
        onCancel={() => setConfirmDialog(prev => ({ ...prev, isOpen: false }))}
        confirmText={t('common.action.confirm')}
        cancelText={t('common.action.cancel')}
        variant="danger"
      />
    </div>
  );
};
