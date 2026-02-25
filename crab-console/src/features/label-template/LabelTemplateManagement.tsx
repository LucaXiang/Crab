import React, { useCallback, useEffect, useState } from 'react';
import { Plus, Tag, Pencil, Trash2, Copy, Loader2, Monitor } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import {
  listLabelTemplates,
  createLabelTemplate,
  updateLabelTemplate,
  deleteLabelTemplate,
} from '@/infrastructure/api/store';
import type { LabelTemplate, LabelTemplateCreate, LabelFieldInput } from '@/core/types/store';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog/ConfirmDialog';
import { LabelEditorScreen } from './LabelEditorScreen';
import { DEFAULT_LABEL_TEMPLATES } from './constants';

export const LabelTemplateManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);

  const [templates, setTemplates] = useState<LabelTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const mq = window.matchMedia('(max-width: 1023px)');
    setIsMobile(mq.matches);
    const handler = (e: MediaQueryListEvent) => setIsMobile(e.matches);
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  // Editor state
  const [editingTemplate, setEditingTemplate] = useState<LabelTemplate | null>(null);
  const [isCreating, setIsCreating] = useState(false);

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<LabelTemplate | null>(null);
  const [deleting, setDeleting] = useState(false);

  const fetchTemplates = useCallback(async () => {
    if (!token || !storeId) return;
    try {
      setLoading(true);
      const data = await listLabelTemplates(token, storeId);
      setTemplates(data);
      setError('');
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [token, storeId]);

  useEffect(() => {
    fetchTemplates();
  }, [fetchTemplates]);

  const handleCreateFromPreset = async (preset: (typeof DEFAULT_LABEL_TEMPLATES)[number]) => {
    if (!token || !storeId) return;
    try {
      const payload: LabelTemplateCreate = {
        name: preset.name,
        description: preset.description,
        width: preset.width,
        height: preset.height,
        padding: preset.padding,
        width_mm: preset.width_mm,
        height_mm: preset.height_mm,
        is_default: preset.is_default,
        is_active: preset.is_active,
        fields: preset.fields as LabelFieldInput[],
      };
      await createLabelTemplate(token, storeId, payload);
      await fetchTemplates();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleCreateBlank = () => {
    if (isMobile) return;
    setIsCreating(true);
    setEditingTemplate({
      id: 0, // Sentinel: new template
      name: t('settings.label.new_template'),
      width: 40,
      height: 30,
      width_mm: 40,
      height_mm: 30,
      padding: 2,
      fields: [],
      is_default: false,
      is_active: true,
      created_at: Date.now(),
      updated_at: Date.now(),
    });
  };

  const handleEdit = (tmpl: LabelTemplate) => {
    if (isMobile) return;
    setIsCreating(false);
    setEditingTemplate(tmpl);
  };

  const handleDuplicate = async (tmpl: LabelTemplate) => {
    if (!token || !storeId) return;
    try {
      const payload: LabelTemplateCreate = {
        name: `${tmpl.name} (Copy)`,
        description: tmpl.description,
        width: tmpl.width,
        height: tmpl.height,
        padding: tmpl.padding,
        width_mm: tmpl.width_mm,
        height_mm: tmpl.height_mm,
        padding_mm_x: tmpl.padding_mm_x,
        padding_mm_y: tmpl.padding_mm_y,
        render_dpi: tmpl.render_dpi,
        test_data: tmpl.test_data,
        fields: tmpl.fields as LabelFieldInput[],
        is_default: false,
        is_active: true,
      };
      await createLabelTemplate(token, storeId, payload);
      await fetchTemplates();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleSave = async (tmpl: LabelTemplate) => {
    if (!token || !storeId) return;
    try {
      if (isCreating || tmpl.id === 0) {
        const payload: LabelTemplateCreate = {
          name: tmpl.name,
          description: tmpl.description,
          width: tmpl.width,
          height: tmpl.height,
          padding: tmpl.padding,
          width_mm: tmpl.width_mm,
          height_mm: tmpl.height_mm,
          padding_mm_x: tmpl.padding_mm_x,
          padding_mm_y: tmpl.padding_mm_y,
          render_dpi: tmpl.render_dpi,
          test_data: tmpl.test_data,
          fields: tmpl.fields as LabelFieldInput[],
          is_default: tmpl.is_default,
          is_active: tmpl.is_active,
        };
        await createLabelTemplate(token, storeId, payload);
      } else {
        await updateLabelTemplate(token, storeId, tmpl.id, {
          name: tmpl.name,
          description: tmpl.description,
          width: tmpl.width,
          height: tmpl.height,
          padding: tmpl.padding,
          width_mm: tmpl.width_mm,
          height_mm: tmpl.height_mm,
          padding_mm_x: tmpl.padding_mm_x,
          padding_mm_y: tmpl.padding_mm_y,
          render_dpi: tmpl.render_dpi,
          test_data: tmpl.test_data,
          fields: tmpl.fields as LabelFieldInput[],
          is_default: tmpl.is_default,
          is_active: tmpl.is_active,
        });
      }
      setEditingTemplate(null);
      setIsCreating(false);
      await fetchTemplates();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDelete = async () => {
    if (!token || !storeId || !deleteTarget) return;
    setDeleting(true);
    try {
      await deleteLabelTemplate(token, storeId, deleteTarget.id);
      setDeleteTarget(null);
      await fetchTemplates();
    } catch (e) {
      setError(String(e));
    } finally {
      setDeleting(false);
    }
  };

  // If editor is open, show it full-screen
  if (editingTemplate) {
    return (
      <LabelEditorScreen
        template={editingTemplate}
        onSave={handleSave}
        onClose={() => {
          setEditingTemplate(null);
          setIsCreating(false);
        }}
      />
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900">{t('settings.label.templates')}</h2>
          <p className="text-sm text-gray-500 mt-1">{t('settings.label.templates_desc')}</p>
        </div>
        <button
          onClick={handleCreateBlank}
          disabled={isMobile}
          className="flex items-center gap-2 px-4 py-2 bg-gray-900 text-white rounded-lg hover:bg-black transition-colors font-medium disabled:opacity-40 disabled:cursor-not-allowed"
        >
          <Plus size={18} />
          {t('common.action.create')}
        </button>
      </div>

      {isMobile && (
        <div className="flex items-center gap-3 px-4 py-3 bg-amber-50 border border-amber-200 rounded-xl text-sm text-amber-700">
          <Monitor size={18} className="shrink-0" />
          {t('common.hint.desktop_only')}
        </div>
      )}

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm">{error}</div>
      )}

      {loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 size={24} className="animate-spin text-gray-400" />
        </div>
      ) : (
        <>
          {/* Template Grid */}
          {templates.length > 0 && (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {templates.map(tmpl => (
                <div
                  key={tmpl.id}
                  className="bg-white rounded-xl border border-gray-200 p-5 hover:shadow-md transition-shadow group"
                >
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h3 className="font-bold text-gray-800">{tmpl.name}</h3>
                      {tmpl.description && (
                        <p className="text-xs text-gray-500 mt-0.5">{tmpl.description}</p>
                      )}
                    </div>
                    <div className={`flex items-center gap-1 ${isMobile ? '' : 'opacity-0 group-hover:opacity-100'} transition-opacity`}>
                      <button
                        onClick={() => handleEdit(tmpl)}
                        disabled={isMobile}
                        className="p-1.5 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-colors disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:text-gray-400 disabled:hover:bg-transparent"
                        title={isMobile ? t('common.hint.desktop_only') : t('common.action.edit')}
                      >
                        <Pencil size={14} />
                      </button>
                      <button
                        onClick={() => handleDuplicate(tmpl)}
                        className="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
                        title={t('common.action.duplicate')}
                      >
                        <Copy size={14} />
                      </button>
                      <button
                        onClick={() => setDeleteTarget(tmpl)}
                        className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                        title={t('common.action.delete')}
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </div>

                  <div className="flex items-center gap-3 text-xs text-gray-400">
                    <span className="font-mono">
                      {tmpl.width_mm ?? tmpl.width}mm x {tmpl.height_mm ?? tmpl.height}mm
                    </span>
                    <span>{tmpl.fields.length} fields</span>
                    {tmpl.is_default && (
                      <span className="px-1.5 py-0.5 bg-blue-50 text-blue-600 rounded text-[0.625rem] font-medium">
                        Default
                      </span>
                    )}
                    {!tmpl.is_active && (
                      <span className="px-1.5 py-0.5 bg-gray-100 text-gray-500 rounded text-[0.625rem] font-medium">
                        Inactive
                      </span>
                    )}
                  </div>

                  {/* Mini preview: field type badges */}
                  <div className="flex flex-wrap gap-1 mt-3">
                    {tmpl.fields.slice(0, 6).map(f => (
                      <span
                        key={f.field_id}
                        className="px-1.5 py-0.5 bg-gray-50 text-gray-500 rounded text-[0.625rem] font-mono"
                      >
                        {f.field_type}
                      </span>
                    ))}
                    {tmpl.fields.length > 6 && (
                      <span className="px-1.5 py-0.5 text-gray-400 text-[0.625rem]">
                        +{tmpl.fields.length - 6}
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Presets section */}
          {templates.length === 0 && (
            <div className="text-center py-12 bg-white rounded-xl border border-gray-200">
              <Tag size={40} className="mx-auto text-gray-300 mb-4" />
              <h3 className="text-lg font-bold text-gray-700 mb-2">{t('settings.label.no_templates')}</h3>
              <p className="text-sm text-gray-500 mb-6">{t('settings.label.no_templates_desc')}</p>
              <div className="flex flex-wrap justify-center gap-3">
                {DEFAULT_LABEL_TEMPLATES.map((preset, i) => (
                  <button
                    key={i}
                    onClick={() => !isMobile && handleCreateFromPreset(preset)}
                    disabled={isMobile}
                    className="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm font-medium disabled:opacity-40 disabled:cursor-not-allowed"
                  >
                    {preset.name}
                  </button>
                ))}
                <button
                  onClick={handleCreateBlank}
                  disabled={isMobile}
                  className="px-4 py-2 bg-gray-900 text-white rounded-lg hover:bg-black transition-colors text-sm font-medium disabled:opacity-40 disabled:cursor-not-allowed"
                >
                  {t('settings.label.create_blank')}
                </button>
              </div>
            </div>
          )}
        </>
      )}

      <ConfirmDialog
        isOpen={!!deleteTarget}
        title={t('common.action.delete')}
        description={t('settings.label.delete_confirm', { name: deleteTarget?.name ?? '' })}
        variant="danger"
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};
