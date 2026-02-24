import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { invokeApi } from '@/infrastructure/api';
import { logger } from '@/utils/logger';
import { ArrowLeft, Save, Layers, Type, Image as ImageIcon, Trash2, GripVertical, Settings, Minus, Printer, HelpCircle, Loader2 } from 'lucide-react';
import { LabelTemplate, LabelField, SUPPORTED_LABEL_FIELDS } from '@/core/domain/types/print';
import { LabelTemplateEditor } from './LabelTemplateEditor';
import { FieldPropertiesPanel } from './FieldPropertiesPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FieldHelperDialog } from './FieldHelperDialog';
import { useI18n } from '../../../hooks/useI18n';
import { useLabelPrinter } from '@/core/stores/ui';
import { NumberInput } from '@/presentation/components/ui/NumberInput';
import { JsonEditor } from '@/presentation/components/ui/JsonEditor';
import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, DragEndEvent } from '@dnd-kit/core';
import { arrayMove, SortableContext, sortableKeyboardCoordinates, verticalListSortingStrategy, useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

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
  renderInfo
}: SortableLayerItemProps) => {
  const { t } = useI18n();
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging
  } = useSortable({ id: field.field_id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    zIndex: isDragging ? 100 : 'auto',
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
      <div className="p-1.5 rounded bg-white border border-gray-100 text-gray-500">
        {renderIcon(field)}
      </div>
      <div className="flex-1 min-w-0">
        <div className={`text-sm font-medium truncate ${isSelected ? 'text-blue-700' : 'text-gray-700'}`}>
          {field.name || t('settings.common.untitled')}
        </div>
        <div className="text-[0.625rem] text-gray-400 font-mono truncate">
          {renderInfo(field)}
        </div>
      </div>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
        className="p-1.5 text-gray-300 hover:text-primary-500 hover:bg-primary-50 rounded transition-colors opacity-0 group-hover:opacity-100"
      >
        <Trash2 size={14} />
      </button>
    </div>
  );
};

interface LabelEditorScreenProps {
  template: LabelTemplate;
  onSave: (template: LabelTemplate) => void;
  onClose: () => void;
  systemFonts?: string[];
}

export const LabelEditorScreen: React.FC<LabelEditorScreenProps> = ({
  template: initialTemplate,
  onSave,
  onClose,
  systemFonts = ['Arial', 'Courier New', 'Times New Roman', 'Verdana']
}) => {
  const { t } = useI18n();
  const labelPrinter = useLabelPrinter();
  const [template, setTemplate] = useState<LabelTemplate>(initialTemplate);
  const [selectedFieldId, setSelectedFieldId] = useState<string | null>(null);
  const [showLayers, setShowLayers] = useState(true);
  const [showProperties, setShowProperties] = useState(true);
  const [showHelper, setShowHelper] = useState(false);
  const [showOffsetBorder, setShowOffsetBorder] = useState(true);
  const [isDirty, setIsDirty] = useState(false);
  const [dialogConfig, setDialogConfig] = useState<{
    isOpen: boolean;
    title: string;
    description: string;
    variant: 'info' | 'warning' | 'danger';
  }>({
    isOpen: false,
    title: '',
    description: '',
    variant: 'info'
  });

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (active.id !== over?.id) {
      // Create a reversed copy of fields (because the list is displayed in reverse)
      const reversedFields = [...template.fields].reverse();
      
      const oldIndex = reversedFields.findIndex((f) => f.field_id === active.id);
      const newIndex = reversedFields.findIndex((f) => f.field_id === over?.id);
      
      const newReversedFields = arrayMove(reversedFields, oldIndex, newIndex);
      
      // Update template with the re-reversed array
      handleTemplateChange({
        ...template,
        fields: newReversedFields.reverse()
      });
    }
  };

  const selectedField = template.fields.find(f => f.field_id === selectedFieldId) || null;

  const handleTemplateChange = (updatedTemplate: LabelTemplate) => {
    setTemplate(updatedTemplate);
    setIsDirty(true);
  };

  const handleFieldUpdate = (updatedField: LabelField) => {
    const updatedFields = template.fields.map(f => 
      f.field_id === updatedField.field_id ? updatedField : f
    );
    handleTemplateChange({ ...template, fields: updatedFields });
  };

  const handleAddField = (type: 'text' | 'image' | 'separator') => {
    const generateId = () => `field_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    let newField: LabelField;

    if (type === 'text') {
      newField = {
        field_id: generateId(),
        field_type: 'text',
        name: t("settings.label.field.new_text"),
        x: 10, y: 10, width: 100, height: 20,
        font_size: 12, font_weight: 'normal', alignment: 'left',
        template: t("settings.label.field.default_text"),
        data_source: '',
        visible: true,
      };
    } else if (type === 'image') {
      newField = {
        field_id: generateId(),
        field_type: 'image',
        name: t("settings.label.field.new_image"),
        x: 10, y: 10, width: 80, height: 80,
        font_size: 12,
        maintain_aspect_ratio: true, data_key: '',
        source_type: 'image',
        data_source: '',
        visible: true,
      };
    } else {
       newField = {
         field_id: generateId(),
         field_type: 'separator',
         name: t("settings.label.field.default_separator"),
         x: 8, y: 50, width: 100, height: 2,
         font_size: 12,
         data_source: '',
         visible: true,
       };
    }

    handleTemplateChange({ ...template, fields: [...template.fields, newField] });
    setSelectedFieldId(newField.field_id);
  };

  const handleDeleteField = (fieldId: string) => {
    handleTemplateChange({
      ...template,
      fields: template.fields.filter(f => f.field_id !== fieldId)
    });
    if (selectedFieldId === fieldId) {
      setSelectedFieldId(null);
    }
  };

  const [isSaving, setIsSaving] = useState(false);

  const saveAndExit = async () => {
    setIsSaving(true);
    try {
      // Upload any pending images first
      const fieldsWithPendingImages = template.fields.filter(
        f => f.source_type === 'image' && f._pending_image_path
      );

      let updatedFields = [...template.fields];

      for (const field of fieldsWithPendingImages) {
        try {
          const hash = await invoke<string>('save_image', { sourcePath: field._pending_image_path });
          // Update the field with the hash and clear pending path
          updatedFields = updatedFields.map(f =>
            f.field_id === field.field_id
              ? { ...f, template: hash, _pending_image_path: undefined }
              : f
          );
        } catch (e) {
          logger.error('Failed to upload image for field', e, { component: 'LabelEditor', fieldId: field.field_id });
          // Continue with other fields even if one fails
        }
      }

      // Clean up _pending_image_path from all fields before saving
      const cleanedFields = updatedFields.map(f => {
        const { _pending_image_path, ...rest } = f;
        return rest as LabelField;
      });

      const templateToSave = { ...template, fields: cleanedFields };
      onSave(templateToSave);
      setIsDirty(false);
    } finally {
      setIsSaving(false);
    }
  };

  const handlePrintTest = async () => {
      try {
        if (!labelPrinter) {
          alert(t("settings.label.select_printer_first"));
          return;
        }

        // Parse test data
        let test_data: Record<string, unknown> = {};
        try {
          if (template.test_data) {
            test_data = JSON.parse(template.test_data) as Record<string, unknown>;
          }
        } catch {
          alert(t("settings.label.invalid_json"));
          return;
        }

        // Generate/Load Base64 images for all Image fields
        const imageFields = template.fields.filter(f => f.field_type === 'image' || f.field_type === 'barcode' || f.field_type === 'qrcode');
        for (const field of imageFields) {
          const source_type = (field.source_type || 'image').toLowerCase();

          if (source_type === 'qrcode' || source_type === 'barcode') {
            // Normalize source_type for comparison
            const normalizedType = source_type as 'qrcode' | 'barcode';
            let content = field.template || field.data_key || '';
            content = content.replace(/\{(\w+)\}/g, (_, key) => {
              return test_data[key] !== undefined ? String(test_data[key]) : `{${key}}`;
            });

            if (!content) continue;

            try {
              if (normalizedType === 'qrcode') {
                const QRCode = (await import('qrcode')).default;
                const dataUri = await QRCode.toDataURL(content, {
                  margin: 1,
                  errorCorrectionLevel: 'M'
                });
                test_data[field.data_key || field.name] = dataUri;
              } else if (normalizedType === 'barcode') {
                const JsBarcode = (await import('jsbarcode')).default;
                // Generate barcode on Canvas (converts to PNG for backend compatibility)
                const canvas = document.createElement('canvas');
                JsBarcode(canvas, content, {
                  format: 'CODE128',
                  displayValue: false,
                  margin: 0,
                  width: 2,
                  height: 80
                });
                const dataUri = canvas.toDataURL('image/png');
                test_data[field.data_key || field.name] = dataUri;
              }
            } catch (genError) {
              logger.warn(`Failed to generate ${source_type} for field`, { component: 'LabelEditor', fieldId: field.field_id, detail: String(genError) });
            }
          } else if (source_type === 'image' || source_type === 'productimage') {
            // Load regular image and convert to Base64
            const imagePath = field.template || field.data_key || '';
            if (!imagePath) continue;

            // Apply variable injection to image path
            let resolvedPath = imagePath.replace(/\{(\w+)\}/g, (_, key) => {
              return test_data[key] !== undefined ? String(test_data[key]) : `{${key}}`;
            });

            try {
              // Check if it's already a data URI
              if (resolvedPath.startsWith('data:')) {
                test_data[field.data_key || field.name] = resolvedPath;
              } else if (resolvedPath.startsWith('http://') || resolvedPath.startsWith('https://')) {
                // URL: fetch and convert to Base64
                const response = await fetch(resolvedPath);
                const blob = await response.blob();
                const dataUri = await new Promise<string>((resolve) => {
                  const reader = new FileReader();
                  reader.onloadend = () => resolve(reader.result as string);
                  reader.readAsDataURL(blob);
                });
                test_data[field.data_key || field.name] = dataUri;
              } else {
                // Local file path: use Tauri's convertFileSrc and fetch
                const { convertFileSrc } = await import('@tauri-apps/api/core');
                const assetUrl = convertFileSrc(resolvedPath);
                const response = await fetch(assetUrl);
                const blob = await response.blob();
                const dataUri = await new Promise<string>((resolve) => {
                  const reader = new FileReader();
                  reader.onloadend = () => resolve(reader.result as string);
                  reader.readAsDataURL(blob);
                });
                test_data[field.data_key || field.name] = dataUri;
              }
            } catch (loadError) {
              logger.warn('Failed to load image for field', { component: 'LabelEditor', fieldId: field.field_id, path: resolvedPath, detail: String(loadError) });
              // Continue without this image
            }
          }
        }

        const ticketData = {
          printer_name: labelPrinter,
          data: test_data,
          template: template,
          label_width_mm: template.width_mm ?? template.width ?? 40,
          label_height_mm: template.height_mm ?? template.height ?? 30,
          override_dpi: template.render_dpi
        };

        await invokeApi('print_label', { request: ticketData });
        setDialogConfig({
          isOpen: true,
          title: t('common.message.success'),
          description: (t("settings.label.test_sent")).replace('{printer}', String(labelPrinter ?? '')),
          variant: 'info'
        });
      } catch (e) {
        logger.error('Failed to print test label', e, { component: 'LabelEditor' });
        const errorMsg = String(e);
        setDialogConfig({
          isOpen: true,
          title: t('common.message.error'),
          description: (t("settings.label.test_failed")).replace('{error}', errorMsg),
          variant: 'danger'
        });
      }
  };
  
  const renderLayerIcon = (field: LabelField) => {
      switch (field.field_type) {
          case 'text': return <Type size={14} />;
          case 'image': return <ImageIcon size={14} />;
          case 'separator': return <Minus size={14} />;
          default: return null;
      }
  };

  const renderLayerInfo = (field: LabelField) => {
      switch (field.field_type) {
          case 'text': return field.template || field.name;
          case 'image': return field.data_key || field.name;
          case 'barcode': return field.data_key || field.name;
          case 'qrcode': return field.data_key || field.name;
          case 'separator': return t("settings.label.horizontal_line");
          default: return '';
      }
  };

  return (
    <div className="fixed inset-0 z-50 bg-gray-100 flex flex-col animate-in fade-in duration-200">
      {/* Header */}
      <div className="h-16 bg-white border-b border-gray-200 flex items-center justify-between px-4 shadow-sm z-10">
        <div className="flex items-center gap-4">
          <button 
            onClick={onClose}
            className="p-2 hover:bg-gray-100 rounded-full text-gray-500 transition-colors"
          >
            <ArrowLeft size={20} />
          </button>
          
          <div className="h-6 w-px bg-gray-200 mx-2" />

          <button
             onClick={() => setShowLayers(!showLayers)}
             className={`p-2 rounded-lg transition-all flex items-center gap-2 ${
               showLayers ? 'bg-gray-100 text-gray-900' : 'text-gray-500 hover:bg-gray-50'
             }`}
             title={t('settings.label.toggle_layers')}
           >
             <Layers size={20} />
             <span className="text-sm font-medium hidden md:inline">{t('settings.label.layers')}</span>
           </button>

          <div className="flex flex-col ml-2">
            <h1 className="text-lg font-bold text-gray-800">{template.name}</h1>
            <span className="text-xs text-gray-500 font-mono">
              {template.width_mm}mm × {template.height_mm}mm
            </span>
          </div>
        </div>

        <div className="flex items-center gap-3">
           <button
             onClick={() => setShowProperties(!showProperties)}
             className={`p-2 rounded-lg transition-all flex items-center gap-2 ${
               showProperties ? 'bg-gray-100 text-gray-900' : 'text-gray-500 hover:bg-gray-50'
             }`}
             title={t('settings.label.toggle_properties')}
           >
             <Settings size={20} />
             <span className="text-sm font-medium hidden md:inline">{t('settings.common.properties')}</span>
           </button>

          <div className="h-6 w-px bg-gray-200 mx-2" />

          <button
            onClick={handlePrintTest}
            className="p-2 text-gray-500 hover:text-gray-900 hover:bg-gray-50 rounded-lg transition-colors"
            title={t('settings.label.print_test_label')}
          >
            <Printer size={20} />
          </button>

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
              (!isDirty || isSaving) ? 'opacity-50 cursor-not-allowed' : 'hover:bg-black'
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
              onFieldSelect={(field) => setSelectedFieldId(field?.field_id || null)}
              selectedFieldId={selectedFieldId}
              visibleAreaInsets={{
                left: showLayers ? 256 : 0,
                right: showProperties ? 320 : 0,
                top: 0,
                bottom: 0
              }}
              showOffsetBorder={showOffsetBorder}
            />
        </div>

        {/* UI Layer */}
        <div className="absolute inset-0 z-10 pointer-events-none flex justify-between">
          {/* Left Sidebar: Layers */}
          <div className={`pointer-events-auto h-full bg-white border-r border-gray-200 flex flex-col transition-all duration-300 ${showLayers ? 'w-64' : 'w-0 overflow-hidden'}`}>
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
              {t("settings.label.field.text")}
            </button>
            <button
              onClick={() => handleAddField('image')}
              className="flex flex-col items-center justify-center gap-1 p-2 bg-white border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 text-[0.625rem] font-medium transition-all"
            >
              <ImageIcon size={16} />
              {t("settings.label.field.image")}
            </button>
            <button
              onClick={() => handleAddField('separator')}
              className="flex flex-col items-center justify-center gap-1 p-2 bg-white border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 text-[0.625rem] font-medium transition-all"
            >
              <Minus size={16} />
              {t("settings.label.field.line")}
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={handleDragEnd}
            >
              <SortableContext
                items={template.fields.slice().reverse().map(f => f.field_id)}
                strategy={verticalListSortingStrategy}
              >
                {template.fields.slice().reverse().map((field) => (
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
              <div className="text-center py-8 text-gray-400 text-xs">
                {t("settings.label.no_layers")}
              </div>
            )}
          </div>
        </div>

        {/* Right Sidebar: Properties */}
        <div className={`pointer-events-auto h-full bg-white border-l border-gray-200 transition-all duration-300 ${showProperties ? 'w-80' : 'w-0 overflow-hidden'}`}>
           {selectedField ? (
             <FieldPropertiesPanel
               field={selectedField}
               onFieldUpdate={handleFieldUpdate}
               onClose={() => setSelectedFieldId(null)}
             />
           ) : (
             <div className="p-4">
               <h3 className="font-bold text-gray-800 mb-4 flex items-center gap-2">
                 <Settings size={18} />
                 {t('settings.printer.templates')}
               </h3>
               
               <div className="space-y-4">
                 <div>
                   <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.template_name")}</label>
                   <input
                     type="text"
                     value={template.name}
                     onChange={(e) => handleTemplateChange({ ...template, name: e.target.value })}
                     className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                   />
                 </div>
                 
                 <div className="grid grid-cols-2 gap-3">
                   <div>
                     <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.width_mm")}</label>
                     <NumberInput
                       value={template.width_mm ?? 0}
                       onValueChange={(val) => handleTemplateChange({ ...template, width_mm: val })}
                       className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                     />
                   </div>
                   <div>
                     <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.height_mm")}</label>
                     <NumberInput
                       value={template.height_mm ?? 0}
                       onValueChange={(val) => handleTemplateChange({ ...template, height_mm: val })}
                       className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                     />
                   </div>
                 </div>

                 <div className="grid grid-cols-2 gap-3">
                   <div>
                     <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.padding_x")}</label>
                     <NumberInput
                       value={template.padding_mm_x || 0}
                       onValueChange={(val) => handleTemplateChange({ ...template, padding_mm_x: val })}
                       className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                       step="0.1"
                     />
                   </div>
                   <div>
                     <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.padding_y")}</label>
                     <NumberInput
                       value={template.padding_mm_y || 0}
                       onValueChange={(val) => handleTemplateChange({ ...template, padding_mm_y: val })}
                       className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                       step="0.1"
                     />
                   </div>
                 </div>

                 <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      id="showOffsetBorder"
                      checked={showOffsetBorder}
                      onChange={(e) => setShowOffsetBorder(e.target.checked)}
                      className="rounded border-gray-300 text-blue-600 focus:ring-blue-500 h-4 w-4"
                    />
                    <label htmlFor="showOffsetBorder" className="text-sm text-gray-600 select-none cursor-pointer">
                      {t("settings.label.show_offset_border")}
                    </label>
                 </div>

                 <div>
                   <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.render_dpi")}</label>
                   <NumberInput
                     value={template.render_dpi || 203}
                     onValueChange={(val) => handleTemplateChange({ ...template, render_dpi: val })}
                     className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                     step="1"
                   />
                   <p className="text-xs text-gray-400 mt-1">
                     {t("settings.label.render_dpi_hint")}
                   </p>
                 </div>

                 <div className="pt-4 border-t border-gray-100">
                    <p className="text-xs text-gray-500 mb-4">
                      {t("settings.label.select_element_hint")}
                    </p>

                    <div className="flex items-center justify-between mb-2">
                      <label className="text-sm font-medium text-gray-700">{t("settings.label.test_data_json")}</label>
                      <button
                        onClick={() => {
                          const sampleData = SUPPORTED_LABEL_FIELDS.reduce((acc, field) => {
                            acc[field.key] = field.example;
                            return acc;
                          }, {} as Record<string, string>);
                          handleTemplateChange({ ...template, test_data: JSON.stringify(sampleData, null, 2) });
                        }}
                        className="text-xs text-blue-600 hover:text-blue-700 font-medium hover:underline"
                      >
                        {t("settings.label.fill_sample")}
                      </button>
                    </div>
                    <JsonEditor
                      value={template.test_data || ''}
                      onChange={(val) => handleTemplateChange({ ...template, test_data: val })}
                      className="h-40"
                      placeholder='{"price": "€10.00", "item_name": "Test Item"}'
                    />
                    <p className="text-xs text-gray-400 mt-2">
                      💡 {t("settings.label.test_data_hint")}
                    </p>
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

      <FieldHelperDialog
        isOpen={showHelper}
        onClose={() => setShowHelper(false)}
      />
    </div>
  );
};
