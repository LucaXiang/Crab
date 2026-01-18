import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ArrowLeft, Save, Layers, Type, Image as ImageIcon, Trash2, GripVertical, Settings, Minus, Printer, HelpCircle } from 'lucide-react';
import { LabelTemplate, LabelField, SUPPORTED_LABEL_FIELDS } from '../../../types/labelTemplate';
import { convertTemplateToRust } from '../../../services/print';
import { LabelTemplateEditor } from './LabelTemplateEditor';
import { FieldPropertiesPanel } from './FieldPropertiesPanel';
import { ConfirmDialog } from '@/presentation/components/ui/ConfirmDialog';
import { FieldHelperDialog } from './FieldHelperDialog';
import { useI18n } from '../../../hooks/useI18n';
import { useLabelPrinter } from '@/core/stores/ui/useLabelPrinter';
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
  } = useSortable({ id: field.id });

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
        <div className="text-[10px] text-gray-400 font-mono truncate">
          {renderInfo(field)}
        </div>
      </div>
      <button
        onClick={(e) => {
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
      
      const oldIndex = reversedFields.findIndex((f) => f.id === active.id);
      const newIndex = reversedFields.findIndex((f) => f.id === over?.id);
      
      const newReversedFields = arrayMove(reversedFields, oldIndex, newIndex);
      
      // Update template with the re-reversed array
      handleTemplateChange({
        ...template,
        fields: newReversedFields.reverse()
      });
    }
  };

  const selectedField = template.fields.find(f => f.id === selectedFieldId) || null;

  const handleTemplateChange = (updatedTemplate: LabelTemplate) => {
    setTemplate(updatedTemplate);
    setIsDirty(true);
  };

  const handleFieldUpdate = (updatedField: LabelField) => {
    const updatedFields = template.fields.map(f => 
      f.id === updatedField.id ? updatedField : f
    );
    handleTemplateChange({ ...template, fields: updatedFields });
  };

  const handleAddField = (type: 'text' | 'image' | 'separator') => {
    const generateId = () => `field_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    let newField: LabelField;

    if (type === 'text') {
      newField = {
        id: generateId(),
        type: 'text',
        name: t("settings.label.field.newText"),
        x: 10, y: 10, width: 100, height: 20,
        fontSize: 12, fontWeight: 'normal', alignment: 'left',
        template: t("settings.label.field.defaultText"),
        dataSource: '',
        visible: true,
      };
    } else if (type === 'image') {
      newField = {
        id: generateId(),
        type: 'image',
        name: t("settings.label.field.newImage"),
        x: 10, y: 10, width: 80, height: 80,
        fontSize: 12,
        maintainAspectRatio: true, dataKey: '',
        sourceType: 'image',
        dataSource: '',
        visible: true,
      };
    } else {
       newField = {
         id: generateId(),
         type: 'separator',
         name: t("settings.label.field.defaultSeparator"),
         x: 8, y: 50, width: 100, height: 2,
         fontSize: 12,
         dataSource: '',
         visible: true,
       };
    }

    handleTemplateChange({ ...template, fields: [...template.fields, newField] });
    setSelectedFieldId(newField.id);
  };

  const handleDeleteField = (fieldId: string) => {
    handleTemplateChange({
      ...template,
      fields: template.fields.filter(f => f.id !== fieldId)
    });
    if (selectedFieldId === fieldId) {
      setSelectedFieldId(null);
    }
  };

  const saveAndExit = () => {
    onSave(template);
    setIsDirty(false);
  };

  const handlePrintTest = async () => {
      try {
        if (!labelPrinter) {
          alert(t("settings.label.selectPrinterFirst"));
          return;
        }

        // Parse test data
        let testData: any = {};
        try {
          if (template.testData) {
            testData = JSON.parse(template.testData);
          }
        } catch (e) {
          console.warn("Invalid test data JSON:", e);
          alert(t("settings.label.invalidJson"));
          return;
        }

        // Generate/Load Base64 images for all Image fields
        const imageFields = template.fields.filter(f => f.type === 'image' || f.type === 'barcode' || f.type === 'qrcode');
        for (const field of imageFields) {
          const sourceType = (field.sourceType || 'image').toLowerCase();

          if (sourceType === 'qrcode' || sourceType === 'barcode') {
            // Normalize sourceType for comparison
            const normalizedType = sourceType as 'qrcode' | 'barcode';
            let content = field.template || field.dataKey || '';
            content = content.replace(/\{(\w+)\}/g, (_, key) => {
              return testData[key] !== undefined ? String(testData[key]) : `{${key}}`;
            });

            if (!content) continue;

            try {
              if (normalizedType === 'qrcode') {
                const QRCode = (await import('qrcode')).default;
                const dataUri = await QRCode.toDataURL(content, {
                  margin: 1,
                  errorCorrectionLevel: 'M'
                });
                testData[field.dataKey || field.name] = dataUri;
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
                testData[field.dataKey || field.name] = dataUri;
              }
            } catch (genError) {
              console.warn(`Failed to generate ${sourceType} for field ${field.id}:`, genError);
            }
          } else if (sourceType === 'image' || sourceType === 'productimage') {
            // Load regular image and convert to Base64
            const imagePath = field.template || field.dataKey || '';
            if (!imagePath) continue;

            // Apply variable injection to image path
            let resolvedPath = imagePath.replace(/\{(\w+)\}/g, (_, key) => {
              return testData[key] !== undefined ? String(testData[key]) : `{${key}}`;
            });

            try {
              // Check if it's already a data URI
              if (resolvedPath.startsWith('data:')) {
                testData[field.dataKey || field.name] = resolvedPath;
              } else if (resolvedPath.startsWith('http://') || resolvedPath.startsWith('https://')) {
                // URL: fetch and convert to Base64
                const response = await fetch(resolvedPath);
                const blob = await response.blob();
                const dataUri = await new Promise<string>((resolve) => {
                  const reader = new FileReader();
                  reader.onloadend = () => resolve(reader.result as string);
                  reader.readAsDataURL(blob);
                });
                testData[field.dataKey || field.name] = dataUri;
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
                testData[field.dataKey || field.name] = dataUri;
              }
              console.log(`Loaded image for ${field.dataKey || field.name}:`, (testData[field.dataKey || field.name] as string)?.substring(0, 50) + '...');
            } catch (loadError) {
              console.warn(`Failed to load image for field ${field.id} (${resolvedPath}):`, loadError);
              // Continue without this image
            }
          }
        }

        // Map template to Rust structure (camelCase to snake_case)
        const rustTemplate = convertTemplateToRust(template);

        const ticketData = {
          printer_name: labelPrinter,
          data: testData,
          template: rustTemplate,
          label_width_mm: (template.widthMm ?? 0) + (template.paddingMmX ?? 0), // Auto-expand paper width by offset
          label_height_mm: (template.heightMm ?? 0) + (template.paddingMmY ?? 0), // Auto-expand paper height by offset
          override_dpi: template.renderDpi
        };

        console.log("Printing test label:", ticketData);
        console.log("Test data with generated images:", testData);

        // Backend command print_label_cmd(ticket: LabelTicketData) expects 'ticket' argument
        await invoke('print_label_cmd', { ticket: ticketData });
        setDialogConfig({
          isOpen: true,
          title: t('common.success'),
          description: (t("settings.label.testSent")).replace('{printer}', String(labelPrinter.selectedPrinterId ?? '')),
          variant: 'info'
        });
      } catch (e) {
        console.error("Failed to print test label:", e);
        const errorMsg = String(e);
        setDialogConfig({
          isOpen: true,
          title: t('common.error'),
          description: (t("settings.label.testFailed")).replace('{error}', errorMsg),
          variant: 'danger'
        });
      }
  };
  
  const renderLayerIcon = (field: LabelField) => {
      switch (field.type) {
          case 'text': return <Type size={14} />;
          case 'image': return <ImageIcon size={14} />;
          case 'separator': return <Minus size={14} />;
          default: return null;
      }
  };

  const renderLayerInfo = (field: LabelField) => {
      switch (field.type) {
          case 'text': return field.template || field.name;
          case 'image': return field.dataKey || field.name;
          case 'barcode': return field.dataKey || field.name;
          case 'qrcode': return field.dataKey || field.name;
          case 'separator': return t("settings.label.horizontalLine");
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
             title={t('settings.label.toggleLayers')}
           >
             <Layers size={20} />
             <span className="text-sm font-medium hidden md:inline">{t('settings.label.layers')}</span>
           </button>

          <div className="flex flex-col ml-2">
            <h1 className="text-lg font-bold text-gray-800">{template.name}</h1>
            <span className="text-xs text-gray-500 font-mono">
              {template.widthMm}mm × {template.heightMm}mm
            </span>
          </div>
        </div>

        <div className="flex items-center gap-3">
           <button
             onClick={() => setShowProperties(!showProperties)}
             className={`p-2 rounded-lg transition-all flex items-center gap-2 ${
               showProperties ? 'bg-gray-100 text-gray-900' : 'text-gray-500 hover:bg-gray-50'
             }`}
             title={t('settings.label.toggleProperties')}
           >
             <Settings size={20} />
             <span className="text-sm font-medium hidden md:inline">{t('settings.common.properties')}</span>
           </button>

          <div className="h-6 w-px bg-gray-200 mx-2" />

          <button
            onClick={handlePrintTest}
            className="p-2 text-gray-500 hover:text-gray-900 hover:bg-gray-50 rounded-lg transition-colors"
            title={t('settings.label.printTestLabel')}
          >
            <Printer size={20} />
          </button>

          <button
            onClick={() => setShowHelper(true)}
            className="p-2 text-gray-500 hover:text-gray-900 hover:bg-gray-50 rounded-lg transition-colors"
            title={t('settings.supportedFields')}
          >
            <HelpCircle size={20} />
          </button>

          <button 
            onClick={saveAndExit}
            disabled={!isDirty}
            className={`flex items-center gap-2 px-4 py-2 bg-gray-900 text-white rounded-lg transition-colors font-medium shadow-lg shadow-gray-200 ${
              !isDirty ? 'opacity-50 cursor-not-allowed' : 'hover:bg-black'
            }`}
          >
            <Save size={18} />
            {t('common.save')}
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
              onFieldSelect={(field) => setSelectedFieldId(field?.id || null)}
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
              className="flex flex-col items-center justify-center gap-1 p-2 bg-white border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 text-[10px] font-medium transition-all"
            >
              <Type size={16} />
              {t("settings.label.field.text")}
            </button>
            <button
              onClick={() => handleAddField('image')}
              className="flex flex-col items-center justify-center gap-1 p-2 bg-white border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 text-[10px] font-medium transition-all"
            >
              <ImageIcon size={16} />
              {t("settings.label.field.image")}
            </button>
            <button
              onClick={() => handleAddField('separator')}
              className="flex flex-col items-center justify-center gap-1 p-2 bg-white border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 text-[10px] font-medium transition-all"
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
                items={template.fields.slice().reverse().map(f => f.id)}
                strategy={verticalListSortingStrategy}
              >
                {template.fields.slice().reverse().map((field) => (
                  <SortableLayerItem
                    key={field.id}
                    field={field}
                    isSelected={selectedFieldId === field.id}
                    onSelect={() => setSelectedFieldId(field.id)}
                    onDelete={() => handleDeleteField(field.id)}
                    renderIcon={renderLayerIcon}
                    renderInfo={renderLayerInfo}
                  />
                ))}
              </SortableContext>
            </DndContext>
            
            {template.fields.length === 0 && (
              <div className="text-center py-8 text-gray-400 text-xs">
                {t("settings.label.noLayers")}
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
                   <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.templateName")}</label>
                   <input
                     type="text"
                     value={template.name}
                     onChange={(e) => handleTemplateChange({ ...template, name: e.target.value })}
                     className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                   />
                 </div>
                 
                 <div className="grid grid-cols-2 gap-3">
                   <div>
                     <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.widthMm")}</label>
                     <NumberInput
                       value={template.widthMm ?? 0}
                       onValueChange={(val) => handleTemplateChange({ ...template, widthMm: val })}
                       className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                     />
                   </div>
                   <div>
                     <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.heightMm")}</label>
                     <NumberInput
                       value={template.heightMm ?? 0}
                       onValueChange={(val) => handleTemplateChange({ ...template, heightMm: val })}
                       className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                     />
                   </div>
                 </div>

                 <div className="grid grid-cols-2 gap-3">
                   <div>
                     <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.paddingX")}</label>
                     <NumberInput
                       value={template.paddingMmX || 0}
                       onValueChange={(val) => handleTemplateChange({ ...template, paddingMmX: val })}
                       className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                       step="0.1"
                     />
                   </div>
                   <div>
                     <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.paddingY")}</label>
                     <NumberInput
                       value={template.paddingMmY || 0}
                       onValueChange={(val) => handleTemplateChange({ ...template, paddingMmY: val })}
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
                      {t("settings.label.showOffsetBorder")}
                    </label>
                 </div>

                 <div>
                   <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.renderDpi")}</label>
                   <NumberInput
                     value={template.renderDpi || 203}
                     onValueChange={(val) => handleTemplateChange({ ...template, renderDpi: val })}
                     className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                     step="1"
                   />
                   <p className="text-xs text-gray-400 mt-1">
                     {t("settings.label.renderDpiHint")}
                   </p>
                 </div>

                 <div className="pt-4 border-t border-gray-100">
                    <p className="text-xs text-gray-500 mb-4">
                      {t("settings.label.selectElementHint")}
                    </p>

                    <div className="flex items-center justify-between mb-2">
                      <label className="text-sm font-medium text-gray-700">{t("settings.label.testDataJson")}</label>
                      <button
                        onClick={() => {
                          const sampleData = SUPPORTED_LABEL_FIELDS.reduce((acc, field) => {
                            acc[field.key] = field.example;
                            return acc;
                          }, {} as Record<string, string>);
                          handleTemplateChange({ ...template, testData: JSON.stringify(sampleData, null, 2) });
                        }}
                        className="text-xs text-blue-600 hover:text-blue-700 font-medium hover:underline"
                      >
                        {t("settings.label.fillSample")}
                      </button>
                    </div>
                    <JsonEditor
                      value={template.testData || ''}
                      onChange={(val) => handleTemplateChange({ ...template, testData: val })}
                      className="h-40"
                      placeholder='{"price": "€10.00", "item_name": "Test Item"}'
                    />
                    <p className="text-xs text-gray-400 mt-2">
                      💡 {t("settings.label.testDataHint")}
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
        confirmText={t('common.ok')}
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
