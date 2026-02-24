import React, { useMemo, useRef } from 'react';
import type { LabelField } from '@/core/types/store';
import { LabelFieldType, LabelFieldAlignment, LabelVerticalAlign } from '@/core/types/store';
import {
  Type,
  Image as ImageIcon,
  X,
  AlignLeft,
  AlignCenter,
  AlignRight,
  AlignStartVertical,
  AlignCenterVertical,
  AlignEndVertical,
  Upload,
  Trash2,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface FieldPropertiesPanelProps {
  field: LabelField | null;
  onFieldUpdate: (field: LabelField) => void;
  onClose: () => void;
  /** Map of field_id → pending File for upload */
  pendingFiles: Map<string, File>;
  onFileSelect: (fieldId: string, file: File) => void;
}

export const FieldPropertiesPanel: React.FC<FieldPropertiesPanelProps> = ({
  field,
  onFieldUpdate,
  onClose,
  pendingFiles,
  onFileSelect,
}) => {
  const { t } = useI18n();
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Preview URL: pending blob > saved hash URL (placeholder)
  const previewUrl = useMemo(() => {
    if (field?._pending_blob_url) return field._pending_blob_url;
    // If there's a template hash but no pending, show a placeholder
    if (field?.source_type === 'image' && field?.template) return null; // Will show hash text
    return null;
  }, [field?._pending_blob_url, field?.source_type, field?.template]);

  if (!field) {
    return (
      <div className="bg-white rounded-lg shadow-sm p-6 border border-gray-200 text-center text-gray-500">
        {t('settings.label.select_field_hint')}
      </div>
    );
  }

  const isTextField = field.field_type === LabelFieldType.Text;
  const isImageField =
    field.field_type === LabelFieldType.Image || field.field_type === LabelFieldType.Barcode || field.field_type === LabelFieldType.Qrcode;
  const isSeparatorField = field.field_type === LabelFieldType.Separator;

  const handleUpdate = (updates: Partial<LabelField>) => {
    onFieldUpdate({ ...field, ...updates } as LabelField);
  };

  const handleSelectImage = () => {
    fileInputRef.current?.click();
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !field) return;
    const blobUrl = URL.createObjectURL(file);
    onFileSelect(field.field_id, file);
    handleUpdate({ _pending_blob_url: blobUrl, template: '' });
    // Reset input so same file can be re-selected
    e.target.value = '';
  };

  const handleClearImage = () => {
    if (field._pending_blob_url) URL.revokeObjectURL(field._pending_blob_url);
    handleUpdate({ template: '', _pending_blob_url: undefined });
  };

  const renderAlignIcon = (align: LabelFieldAlignment) => {
    switch (align) {
      case LabelFieldAlignment.Left:
        return <AlignLeft size={16} />;
      case LabelFieldAlignment.Center:
        return <AlignCenter size={16} />;
      case LabelFieldAlignment.Right:
        return <AlignRight size={16} />;
    }
  };

  const renderVerticalAlignIcon = (align: LabelVerticalAlign) => {
    switch (align) {
      case LabelVerticalAlign.Top:
        return <AlignStartVertical size={16} />;
      case LabelVerticalAlign.Middle:
        return <AlignCenterVertical size={16} />;
      case LabelVerticalAlign.Bottom:
        return <AlignEndVertical size={16} />;
    }
  };

  return (
    <div className="bg-white h-full flex flex-col">
      {/* Hidden file input for image upload */}
      <input
        ref={fileInputRef}
        type="file"
        accept="image/png,image/jpeg,image/webp"
        className="hidden"
        onChange={handleFileChange}
      />

      <div className="p-4 border-b border-gray-100 flex items-center justify-between">
        <div className="flex items-center gap-2">
          {isTextField ? (
            <Type size={18} className="text-gray-600" />
          ) : (
            <ImageIcon size={18} className="text-blue-600" />
          )}
          <h3 className="text-lg font-semibold text-gray-800">{t('settings.label.field_properties')}</h3>
        </div>
        <button
          onClick={onClose}
          className="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded transition-colors"
        >
          <X size={18} />
        </button>
      </div>

      <div className="p-4 space-y-4 flex-1 overflow-y-auto">
        {/* Name */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            {t('settings.label.display_label')}
          </label>
          <input
            type="text"
            value={field.name || ''}
            onChange={e => handleUpdate({ name: e.target.value })}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            placeholder={t('settings.label.placeholder')}
          />
        </div>

        {/* Position */}
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.label.x_position')}</label>
            <input
              type="number"
              value={isSeparatorField ? 0 : parseFloat(field.x.toFixed(2))}
              disabled={isSeparatorField}
              onChange={e => !isSeparatorField && handleUpdate({ x: Number(e.target.value) })}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:text-gray-400"
              min={0}
              step={0.1}
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.label.y_position')}</label>
            <input
              type="number"
              value={parseFloat(field.y.toFixed(2))}
              onChange={e => handleUpdate({ y: Number(e.target.value) })}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              min={0}
              step={0.1}
            />
          </div>
        </div>

        {/* Size */}
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.label.width')}</label>
            <input
              type="number"
              value={isSeparatorField ? 0 : parseFloat(field.width.toFixed(2))}
              disabled={isSeparatorField}
              onChange={e => !isSeparatorField && handleUpdate({ width: Number(e.target.value) })}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:text-gray-400"
              min={20}
              step={0.1}
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.label.height')}</label>
            <input
              type="number"
              value={isSeparatorField ? 0 : parseFloat(field.height.toFixed(2))}
              disabled={isSeparatorField}
              onChange={e => !isSeparatorField && handleUpdate({ height: Number(e.target.value) })}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:text-gray-400"
              min={10}
              step={0.1}
            />
          </div>
        </div>

        {/* Text-specific */}
        {isTextField && (
          <>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.label.content_template')}
              </label>
              <textarea
                value={field.template || ''}
                onChange={e => handleUpdate({ template: e.target.value })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent font-mono text-sm"
                placeholder="{product_name}"
                rows={2}
              />
              <p className="mt-1 text-xs text-gray-500">{t('settings.label.text_template_hint')}</p>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.label.font_size')}
                </label>
                <input
                  type="number"
                  value={field.font_size}
                  onChange={e => handleUpdate({ font_size: Number(e.target.value) })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  min={6}
                  max={48}
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.label.font_family')}
                </label>
                <select
                  value={field.font_family || 'Arial'}
                  onChange={e => handleUpdate({ font_family: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                >
                  <option value="Arial">Arial</option>
                  <option value="Times New Roman">Times New Roman</option>
                  <option value="Courier New">Courier New</option>
                  <option value="Verdana">Verdana</option>
                </select>
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.label.font_style')}
              </label>
              <select
                value={field.font_weight || 'normal'}
                onChange={e => handleUpdate({ font_weight: e.target.value })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value="normal">{t('settings.label.style_regular')}</option>
                <option value="bold">{t('settings.label.style_bold')}</option>
              </select>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.label.text_align')}
                </label>
                <div className="flex gap-1">
                  {([LabelFieldAlignment.Left, LabelFieldAlignment.Center, LabelFieldAlignment.Right]).map(align => (
                    <button
                      key={align}
                      onClick={() => handleUpdate({ alignment: align })}
                      className={`flex-1 p-2 rounded-lg border transition-all flex items-center justify-center ${
                        (field.alignment || LabelFieldAlignment.Left) === align
                          ? 'border-blue-500 bg-blue-50 text-blue-600'
                          : 'border-gray-300 text-gray-500 hover:border-gray-400 hover:bg-gray-50'
                      }`}
                    >
                      {renderAlignIcon(align)}
                    </button>
                  ))}
                </div>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.label.vertical_align')}
                </label>
                <div className="flex gap-1">
                  {([LabelVerticalAlign.Top, LabelVerticalAlign.Middle, LabelVerticalAlign.Bottom]).map(align => (
                    <button
                      key={align}
                      onClick={() => handleUpdate({ vertical_align: align })}
                      className={`flex-1 p-2 rounded-lg border transition-all flex items-center justify-center ${
                        (field.vertical_align ?? LabelVerticalAlign.Top) === align
                          ? 'border-blue-500 bg-blue-50 text-blue-600'
                          : 'border-gray-300 text-gray-500 hover:border-gray-400 hover:bg-gray-50'
                      }`}
                    >
                      {renderVerticalAlignIcon(align)}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </>
        )}

        {/* Image/Barcode-specific */}
        {isImageField && (
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('settings.label.source_type')}
              </label>
              <select
                value={field.source_type || 'image'}
                onChange={e =>
                  handleUpdate({
                    source_type: e.target.value as 'productImage' | 'qrCode' | 'barcode' | 'image',
                    template: '',
                  })
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value="image">{t('settings.label.source_type_image')}</option>
                <option value="productImage">{t('settings.label.source_type_product_image')}</option>
                <option value="qrCode">{t('settings.label.source_type_qrcode')}</option>
                <option value="barcode">{t('settings.label.source_type_barcode')}</option>
              </select>
            </div>

            {/* Static Image Upload */}
            {field.source_type === 'image' && (
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  {t('settings.label.static_image')}
                </label>
                {previewUrl ? (
                  <div className="relative group">
                    <img
                      src={previewUrl}
                      alt="Preview"
                      className="w-full h-32 object-contain bg-gray-100 rounded-lg border border-gray-200"
                    />
                    {field._pending_blob_url && (
                      <div className="absolute top-2 left-2 px-2 py-0.5 bg-amber-500 text-white text-xs rounded-full">
                        {t('settings.label.pending_upload')}
                      </div>
                    )}
                    <div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex items-center justify-center gap-2">
                      <button
                        onClick={handleSelectImage}
                        className="p-2 bg-white rounded-lg text-gray-700 hover:bg-gray-100"
                      >
                        <Upload size={18} />
                      </button>
                      <button
                        onClick={handleClearImage}
                        className="p-2 bg-white rounded-lg text-red-600 hover:bg-red-50"
                      >
                        <Trash2 size={18} />
                      </button>
                    </div>
                  </div>
                ) : field.template ? (
                  <div className="relative group">
                    <div className="w-full h-32 bg-gray-100 rounded-lg border border-gray-200 flex items-center justify-center text-xs text-gray-400 font-mono">
                      {field.template.slice(0, 16)}...
                    </div>
                    <div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex items-center justify-center gap-2">
                      <button
                        onClick={handleSelectImage}
                        className="p-2 bg-white rounded-lg text-gray-700 hover:bg-gray-100"
                      >
                        <Upload size={18} />
                      </button>
                      <button
                        onClick={handleClearImage}
                        className="p-2 bg-white rounded-lg text-red-600 hover:bg-red-50"
                      >
                        <Trash2 size={18} />
                      </button>
                    </div>
                  </div>
                ) : (
                  <button
                    onClick={handleSelectImage}
                    className="w-full h-32 border-2 border-dashed border-gray-300 rounded-lg flex flex-col items-center justify-center gap-2 text-gray-500 hover:border-blue-400 hover:text-blue-600 hover:bg-blue-50/50 transition-all"
                  >
                    <Upload size={24} />
                    <span className="text-sm font-medium">{t('settings.label.select_image')}</span>
                  </button>
                )}
              </div>
            )}

            {/* Content Template (for dynamic source_types) */}
            {field.source_type !== 'image' && (
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.label.content_template')}
                </label>
                <input
                  type="text"
                  value={field.template || ''}
                  onChange={e => handleUpdate({ template: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent font-mono text-sm"
                  placeholder={
                    field.source_type === 'productImage'
                      ? '{product_image}'
                      : field.source_type === 'qrCode'
                        ? '{product_code}'
                        : '{barcode}'
                  }
                />
              </div>
            )}

            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="maintain_aspect_ratio"
                checked={field.maintain_aspect_ratio ?? true}
                onChange={e => handleUpdate({ maintain_aspect_ratio: e.target.checked })}
                className="rounded border-gray-300 text-blue-600 focus:ring-blue-500 h-4 w-4"
              />
              <label htmlFor="maintain_aspect_ratio" className="text-sm text-gray-600 select-none cursor-pointer">
                {t('settings.label.maintain_aspect_ratio')}
              </label>
            </div>
          </div>
        )}

        {/* Separator */}
        {isSeparatorField && (
          <div className="bg-gray-50 p-3 rounded-lg text-sm text-gray-500">
            {t('settings.label.separator_hint')}
          </div>
        )}
      </div>
    </div>
  );
};
