import React, { useState, useRef } from 'react';
import {
  ArrowLeft,
  Save,
  Layers,
  Type,
  Image as ImageIcon,
  Trash2,
  GripVertical,
  Settings,
  Minus,
  HelpCircle,
  Loader2,
} from 'lucide-react';
import type { LabelTemplate, LabelField } from '@/core/types/store';
import { LabelTemplateEditor } from './LabelTemplateEditor';
import { FieldPropertiesPanel } from './FieldPropertiesPanel';
import { FieldHelperDialog } from './FieldHelperDialog';
import { SUPPORTED_LABEL_FIELDS } from './constants';
import { useI18n } from '@/hooks/useI18n';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { uploadImage } from '@/infrastructure/api/store';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog/ConfirmDialog';
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
  useSortable,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

// ── SortableLayerItem ──

interface SortableLayerItemProps {
  field: LabelField;
  isSelected: boolean;
  onSelect: () => void;
  onDelete: () => void;
  renderIcon: (field: LabelField) => React.ReactNode;
  renderInfo: (field: LabelField) => React.ReactNode;
}

const SortableLayerItem = ({
  field,
  isSelected,
  onSelect,
  onDelete,
  renderIcon,
  renderInfo,
}: SortableLayerItemProps) => {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: field.field_id,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    zIndex: isDragging ? 100 : ('auto' as const),
    opacity: isDragging ? 0.5 : 1,
    position: 'relative' as const,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      onClick={onSelect}
      className={`flex items-center gap-2 p-2 rounded-lg cursor-pointer border transition-all group ${
        isSelected
          ? 'bg-blue-50 border-blue-200 shadow-sm'
          : 'bg-white border-transparent hover:bg-gray-50 hover:border-gray-200'
      }`}
    >
      <div {...attributes} {...listeners} className="cursor-grab active:cursor-grabbing touch-none outline-none">
        <GripVertical size={14} className="text-gray-300" />
      </div>
      <div className="p-1.5 rounded bg-white border border-gray-100 text-gray-500">{renderIcon(field)}</div>
      <div className="flex-1 min-w-0">
        <div className={`text-sm font-medium truncate ${isSelected ? 'text-blue-700' : 'text-gray-700'}`}>
          {field.name || 'Untitled'}
        </div>
        <div className="text-[0.625rem] text-gray-400 font-mono truncate">{renderInfo(field)}</div>
      </div>
      <button
        onClick={e => {
          e.stopPropagation();
          onDelete();
        }}
        className="p-1.5 text-gray-300 hover:text-red-500 hover:bg-red-50 rounded transition-colors opacity-0 group-hover:opacity-100"
      >
        <Trash2 size={14} />
      </button>
    </div>
  );
};

// ── LabelEditorScreen ──

interface LabelEditorScreenProps {
  template: LabelTemplate;
  onSave: (template: LabelTemplate) => void;
  onClose: () => void;
}

export const LabelEditorScreen: React.FC<LabelEditorScreenProps> = ({
  template: initialTemplate,
  onSave,
  onClose,
}) => {
  const { t } = useI18n();
  const token = useAuthStore(s => s.token);
  const [template, setTemplate] = useState<LabelTemplate>(initialTemplate);
  const [selectedFieldId, setSelectedFieldId] = useState<string | null>(null);
  const [showLayers, setShowLayers] = useState(true);
  const [showProperties, setShowProperties] = useState(true);
  const [showHelper, setShowHelper] = useState(false);
  const [showOffsetBorder, setShowOffsetBorder] = useState(true);
  const [isDirty, setIsDirty] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const pendingFilesRef = useRef(new Map<string, File>());
  const [dialogConfig, setDialogConfig] = useState<{
    isOpen: boolean;
    title: string;
    description: string;
    variant: 'info' | 'warning' | 'danger';
  }>({ isOpen: false, title: '', description: '', variant: 'info' });

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (active.id !== over?.id) {
      const reversedFields = [...template.fields].reverse();
      const oldIndex = reversedFields.findIndex(f => f.field_id === active.id);
      const newIndex = reversedFields.findIndex(f => f.field_id === over?.id);
      const newReversedFields = arrayMove(reversedFields, oldIndex, newIndex);
      handleTemplateChange({ ...template, fields: newReversedFields.reverse() });
    }
  };

  const selectedField = template.fields.find(f => f.field_id === selectedFieldId) || null;

  const handleTemplateChange = (updatedTemplate: LabelTemplate) => {
    setTemplate(updatedTemplate);
    setIsDirty(true);
  };

  const handleFieldUpdate = (updatedField: LabelField) => {
    const updatedFields = template.fields.map(f => (f.field_id === updatedField.field_id ? updatedField : f));
    handleTemplateChange({ ...template, fields: updatedFields });
  };

  const handleAddField = (type: 'text' | 'image' | 'separator') => {
    const generateId = () => `field_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    let newField: LabelField;

    if (type === 'text') {
      newField = {
        field_id: generateId(),
        field_type: 'text',
        name: t('settings.label.field.new_text'),
        x: 10,
        y: 10,
        width: 100,
        height: 20,
        font_size: 12,
        font_weight: 'normal',
        alignment: 'left',
        template: t('settings.label.field.default_text'),
        data_source: '',
        visible: true,
      };
    } else if (type === 'image') {
      newField = {
        field_id: generateId(),
        field_type: 'image',
        name: t('settings.label.field.new_image'),
        x: 10,
        y: 10,
        width: 80,
        height: 80,
        font_size: 12,
        maintain_aspect_ratio: true,
        data_key: '',
        source_type: 'image',
        data_source: '',
        visible: true,
      };
    } else {
      newField = {
        field_id: generateId(),
        field_type: 'separator',
        name: t('settings.label.field.default_separator'),
        x: 8,
        y: 50,
        width: 100,
        height: 2,
        font_size: 12,
        data_source: '',
        visible: true,
      };
    }

    handleTemplateChange({ ...template, fields: [...template.fields, newField] });
    setSelectedFieldId(newField.field_id);
  };

  const handleDeleteField = (fieldId: string) => {
    handleTemplateChange({ ...template, fields: template.fields.filter(f => f.field_id !== fieldId) });
    if (selectedFieldId === fieldId) setSelectedFieldId(null);
  };

  const handleFileSelect = (fieldId: string, file: File) => {
    pendingFilesRef.current.set(fieldId, file);
  };

  const saveAndExit = async () => {
    if (!token) return;
    setIsSaving(true);
    try {
      // Upload any pending images first
      let updatedFields = [...template.fields];

      for (const field of updatedFields) {
        if (field.source_type === 'image' && field._pending_blob_url) {
          const file = pendingFilesRef.current.get(field.field_id);
          if (!file) continue;
          try {
            const hash = await uploadImage(token, file);
            updatedFields = updatedFields.map(f =>
              f.field_id === field.field_id ? { ...f, template: hash, _pending_blob_url: undefined } : f,
            );
          } catch (e) {
            console.error('Failed to upload image for field', field.field_id, e);
          }
        }
      }

      // Clean up _pending_blob_url from all fields before saving
      const cleanedFields = updatedFields.map(f => {
        const { _pending_blob_url, ...rest } = f;
        return rest as LabelField;
      });

      onSave({ ...template, fields: cleanedFields });
      setIsDirty(false);
    } finally {
      setIsSaving(false);
    }
  };

  const renderLayerIcon = (field: LabelField) => {
    switch (field.field_type) {
      case 'text':
        return <Type size={14} />;
      case 'image':
        return <ImageIcon size={14} />;
      case 'separator':
        return <Minus size={14} />;
      default:
        return null;
    }
  };

  const renderLayerInfo = (field: LabelField) => {
    switch (field.field_type) {
      case 'text':
        return field.template || field.name;
      case 'image':
      case 'barcode':
      case 'qrcode':
        return field.data_key || field.name;
      case 'separator':
        return t('settings.label.horizontal_line');
      default:
        return '';
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-gray-100 flex flex-col animate-in fade-in duration-200">
      {/* Header */}
      <div className="h-16 bg-white border-b border-gray-200 flex items-center justify-between px-4 shadow-sm z-10">
        <div className="flex items-center gap-4">
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full text-gray-500 transition-colors">
            <ArrowLeft size={20} />
          </button>

          <div className="h-6 w-px bg-gray-200 mx-2" />

          <button
            onClick={() => setShowLayers(!showLayers)}
            className={`p-2 rounded-lg transition-all flex items-center gap-2 ${
              showLayers ? 'bg-gray-100 text-gray-900' : 'text-gray-500 hover:bg-gray-50'
            }`}
          >
            <Layers size={20} />
            <span className="text-sm font-medium hidden md:inline">{t('settings.label.layers')}</span>
          </button>

          <div className="flex flex-col ml-2">
            <h1 className="text-lg font-bold text-gray-800">{template.name}</h1>
            <span className="text-xs text-gray-500 font-mono">
              {template.width_mm}mm x {template.height_mm}mm
            </span>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={() => setShowProperties(!showProperties)}
            className={`p-2 rounded-lg transition-all flex items-center gap-2 ${
              showProperties ? 'bg-gray-100 text-gray-900' : 'text-gray-500 hover:bg-gray-50'
            }`}
          >
            <Settings size={20} />
            <span className="text-sm font-medium hidden md:inline">{t('settings.common.properties')}</span>
          </button>

          <div className="h-6 w-px bg-gray-200 mx-2" />

          <button
            onClick={() => setShowHelper(true)}
            className="p-2 text-gray-500 hover:text-gray-900 hover:bg-gray-50 rounded-lg transition-colors"
            title={t('settings.supported_fields')}
          >
            <HelpCircle size={20} />
          </button>

          <button
            onClick={saveAndExit}
            disabled={!isDirty || isSaving}
            className={`flex items-center gap-2 px-4 py-2 bg-gray-900 text-white rounded-lg transition-colors font-medium shadow-lg shadow-gray-200 ${
              !isDirty || isSaving ? 'opacity-50 cursor-not-allowed' : 'hover:bg-black'
            }`}
          >
            {isSaving ? <Loader2 size={18} className="animate-spin" /> : <Save size={18} />}
            {isSaving ? t('common.status.saving') : t('common.action.save')}
          </button>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 relative overflow-hidden">
        {/* Canvas Layer */}
        <div className="absolute inset-0 bg-gray-100 overflow-hidden z-0">
          <LabelTemplateEditor
            template={template}
            onTemplateChange={handleTemplateChange}
            onFieldSelect={field => setSelectedFieldId(field?.field_id || null)}
            selectedFieldId={selectedFieldId}
            visibleAreaInsets={{
              left: showLayers ? 256 : 0,
              right: showProperties ? 320 : 0,
              top: 0,
              bottom: 0,
            }}
            showOffsetBorder={showOffsetBorder}
          />
        </div>

        {/* UI Layer */}
        <div className="absolute inset-0 z-10 pointer-events-none flex justify-between">
          {/* Left Sidebar: Layers */}
          <div
            className={`pointer-events-auto h-full bg-white border-r border-gray-200 flex flex-col transition-all duration-300 ${showLayers ? 'w-64' : 'w-0 overflow-hidden'}`}
          >
            <div className="p-4 border-b border-gray-100 flex items-center justify-between bg-gray-50/50">
              <h3 className="font-bold text-gray-700 flex items-center gap-2">
                <Layers size={16} />
                {t('settings.label.layers')}
              </h3>
            </div>

            <div className="p-3 grid grid-cols-3 gap-2 border-b border-gray-100">
              <button
                onClick={() => handleAddField('text')}
                className="flex flex-col items-center justify-center gap-1 p-2 bg-white border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 text-[0.625rem] font-medium transition-all"
              >
                <Type size={16} />
                {t('settings.label.field.text')}
              </button>
              <button
                onClick={() => handleAddField('image')}
                className="flex flex-col items-center justify-center gap-1 p-2 bg-white border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 text-[0.625rem] font-medium transition-all"
              >
                <ImageIcon size={16} />
                {t('settings.label.field.image')}
              </button>
              <button
                onClick={() => handleAddField('separator')}
                className="flex flex-col items-center justify-center gap-1 p-2 bg-white border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 text-[0.625rem] font-medium transition-all"
              >
                <Minus size={16} />
                {t('settings.label.field.line')}
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-2 space-y-1">
              <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                <SortableContext
                  items={template.fields
                    .slice()
                    .reverse()
                    .map(f => f.field_id)}
                  strategy={verticalListSortingStrategy}
                >
                  {template.fields
                    .slice()
                    .reverse()
                    .map(field => (
                      <SortableLayerItem
                        key={field.field_id}
                        field={field}
                        isSelected={selectedFieldId === field.field_id}
                        onSelect={() => setSelectedFieldId(field.field_id)}
                        onDelete={() => handleDeleteField(field.field_id)}
                        renderIcon={renderLayerIcon}
                        renderInfo={renderLayerInfo}
                      />
                    ))}
                </SortableContext>
              </DndContext>

              {template.fields.length === 0 && (
                <div className="text-center py-8 text-gray-400 text-xs">{t('settings.label.no_layers')}</div>
              )}
            </div>
          </div>

          {/* Right Sidebar: Properties */}
          <div
            className={`pointer-events-auto h-full bg-white border-l border-gray-200 transition-all duration-300 ${showProperties ? 'w-80' : 'w-0 overflow-hidden'}`}
          >
            {selectedField ? (
              <FieldPropertiesPanel
                field={selectedField}
                onFieldUpdate={handleFieldUpdate}
                onClose={() => setSelectedFieldId(null)}
                pendingFiles={pendingFilesRef.current}
                onFileSelect={handleFileSelect}
              />
            ) : (
              <div className="p-4">
                <h3 className="font-bold text-gray-800 mb-4 flex items-center gap-2">
                  <Settings size={18} />
                  {t('settings.printer.templates')}
                </h3>

                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      {t('settings.label.template_name')}
                    </label>
                    <input
                      type="text"
                      value={template.name}
                      onChange={e => handleTemplateChange({ ...template, name: e.target.value })}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    />
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        {t('settings.label.width_mm')}
                      </label>
                      <input
                        type="number"
                        value={template.width_mm ?? 0}
                        onChange={e => handleTemplateChange({ ...template, width_mm: Number(e.target.value) })}
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        {t('settings.label.height_mm')}
                      </label>
                      <input
                        type="number"
                        value={template.height_mm ?? 0}
                        onChange={e => handleTemplateChange({ ...template, height_mm: Number(e.target.value) })}
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                      />
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        {t('settings.label.padding_x')}
                      </label>
                      <input
                        type="number"
                        value={template.padding_mm_x || 0}
                        onChange={e => handleTemplateChange({ ...template, padding_mm_x: Number(e.target.value) })}
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        step={0.1}
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        {t('settings.label.padding_y')}
                      </label>
                      <input
                        type="number"
                        value={template.padding_mm_y || 0}
                        onChange={e => handleTemplateChange({ ...template, padding_mm_y: Number(e.target.value) })}
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        step={0.1}
                      />
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      id="showOffsetBorder"
                      checked={showOffsetBorder}
                      onChange={e => setShowOffsetBorder(e.target.checked)}
                      className="rounded border-gray-300 text-blue-600 focus:ring-blue-500 h-4 w-4"
                    />
                    <label htmlFor="showOffsetBorder" className="text-sm text-gray-600 select-none cursor-pointer">
                      {t('settings.label.show_offset_border')}
                    </label>
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      {t('settings.label.render_dpi')}
                    </label>
                    <input
                      type="number"
                      value={template.render_dpi || 203}
                      onChange={e => handleTemplateChange({ ...template, render_dpi: Number(e.target.value) })}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                      step={1}
                    />
                    <p className="text-xs text-gray-400 mt-1">{t('settings.label.render_dpi_hint')}</p>
                  </div>

                  <div className="pt-4 border-t border-gray-100">
                    <p className="text-xs text-gray-500 mb-4">{t('settings.label.select_element_hint')}</p>

                    <div className="flex items-center justify-between mb-2">
                      <label className="text-sm font-medium text-gray-700">
                        {t('settings.label.test_data_json')}
                      </label>
                      <button
                        onClick={() => {
                          const sampleData = SUPPORTED_LABEL_FIELDS.reduce(
                            (acc, field) => {
                              acc[field.key] = field.example;
                              return acc;
                            },
                            {} as Record<string, string>,
                          );
                          handleTemplateChange({ ...template, test_data: JSON.stringify(sampleData, null, 2) });
                        }}
                        className="text-xs text-blue-600 hover:text-blue-700 font-medium hover:underline"
                      >
                        {t('settings.label.fill_sample')}
                      </button>
                    </div>
                    <textarea
                      value={template.test_data || ''}
                      onChange={e => handleTemplateChange({ ...template, test_data: e.target.value })}
                      className="w-full h-40 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent font-mono text-sm resize-none"
                      placeholder='{"price": "10.00", "item_name": "Test Item"}'
                    />
                    <p className="text-xs text-gray-400 mt-2">{t('settings.label.test_data_hint')}</p>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      <ConfirmDialog
        isOpen={dialogConfig.isOpen}
        title={dialogConfig.title}
        description={dialogConfig.description}
        variant={dialogConfig.variant}
        showCancel={false}
        confirmText={t('common.dialog.ok')}
        onConfirm={() => setDialogConfig(prev => ({ ...prev, isOpen: false }))}
        onCancel={() => setDialogConfig(prev => ({ ...prev, isOpen: false }))}
      />

      <FieldHelperDialog isOpen={showHelper} onClose={() => setShowHelper(false)} />
    </div>
  );
};
