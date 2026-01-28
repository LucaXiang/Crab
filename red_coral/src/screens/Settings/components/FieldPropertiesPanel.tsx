import { useMemo } from 'react';
import { LabelField, TextAlign, VerticalAlign } from '@/core/domain/types/print';
import { Type, Image as ImageIcon, X, AlignLeft, AlignCenter, AlignRight, AlignStartVertical, AlignCenterVertical, AlignEndVertical, Upload, Trash2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { NumberInput } from '@/presentation/components/ui/NumberInput';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
import { useImageUrl } from '@/core/hooks/useImageUrl';
import { convertFileSrc } from '@tauri-apps/api/core';

interface FieldPropertiesPanelProps {
  field: LabelField | null;
  onFieldUpdate: (field: LabelField) => void;
  onClose: () => void;
}

export const FieldPropertiesPanel: React.FC<FieldPropertiesPanelProps> = ({
  field,
  onFieldUpdate,
  onClose,
}) => {
  const { t } = useI18n();

  // Get image URL for preview
  // Priority: pending local path > uploaded hash
  const imageHash = field?.sourceType === 'image' && !field?._pendingImagePath ? field?.template : null;
  const [hashUrl] = useImageUrl(imageHash);

  // Convert local path to asset URL for preview
  const previewUrl = useMemo(() => {
    if (field?._pendingImagePath) {
      return convertFileSrc(field._pendingImagePath);
    }
    return hashUrl;
  }, [field?._pendingImagePath, hashUrl]);

  if (!field) {
    return (
      <div className="bg-white rounded-lg shadow-sm p-6 border border-gray-200 text-center text-gray-500">
        {t("settings.label.select_field_hint")}
      </div>
    );
  }

  const isTextField = field.type === 'text';
  const isImageField = field.type === 'image' || field.type === 'barcode' || field.type === 'qrcode';
  const isSeparatorField = field.type === 'separator';

  const handleUpdate = (updates: Partial<LabelField>) => {
    onFieldUpdate({ ...field, ...updates } as LabelField);
  };

  const handleSelectImage = async () => {
    if (!field) return;
    try {
      const file = await dialogOpen({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });
      if (!file || Array.isArray(file)) return;

      // Save local path for preview, actual upload happens on template save
      handleUpdate({ _pendingImagePath: file, template: '' });
    } catch (e) {
      console.error('Failed to select image:', e);
    }
  };

  const handleClearImage = () => {
    handleUpdate({ template: '', _pendingImagePath: undefined });
  };

  const renderAlignIcon = (align: TextAlign) => {
    switch (align) {
      case 'left': return <AlignLeft size={16} />;
      case 'center': return <AlignCenter size={16} />;
      case 'right': return <AlignRight size={16} />;
    }
  };

  const renderVerticalAlignIcon = (align: VerticalAlign) => {
    switch (align) {
      case 'top': return <AlignStartVertical size={16} />;
      case 'middle': return <AlignCenterVertical size={16} />;
      case 'bottom': return <AlignEndVertical size={16} />;
    }
  };

  return (
    <div className="bg-white h-full flex flex-col">
      <div className="p-4 border-b border-gray-100 flex items-center justify-between">
        <div className="flex items-center gap-2">
          {isTextField ? (
            <Type size={18} className="text-gray-600" />
          ) : (
            <ImageIcon size={18} className="text-blue-600" />
          )}
          <h3 className="text-lg font-semibold text-gray-800">{t("settings.label.field_properties")}</h3>
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
          <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.display_label")}</label>
          <input
            type="text"
            value={field.name || ''}
            onChange={(e) => handleUpdate({ name: e.target.value })}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            placeholder={t("settings.label.placeholder")}
          />
        </div>

        {/* Position */}
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.x_position")}</label>
            <NumberInput
              value={isSeparatorField ? 0 : parseFloat(field.x.toFixed(2))}
              disabled={isSeparatorField}
              onValueChange={(val) => (!isSeparatorField) && handleUpdate({ x: val })}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:text-gray-400"
              min="0"
              step="0.1"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.y_position")}</label>
            <NumberInput
              value={parseFloat(field.y.toFixed(2))}
              onValueChange={(val) => handleUpdate({ y: val })}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              min="0"
              step="0.1"
            />
          </div>
        </div>

        {/* Size */}
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.label.width')}</label>
            <NumberInput
              value={isSeparatorField ? 0 : parseFloat(field.width.toFixed(2))}
              disabled={isSeparatorField}
              onValueChange={(val) => (!isSeparatorField) && handleUpdate({ width: val })}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:text-gray-400"
              min="20"
              step="0.1"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.label.height')}</label>
            <NumberInput
              value={isSeparatorField ? 0 : parseFloat(field.height.toFixed(2))}
              disabled={isSeparatorField}
              onValueChange={(val) => (!isSeparatorField) && handleUpdate({ height: val })}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:text-gray-400"
              min="10"
              step="0.1"
            />
          </div>
        </div>

        {/* Text-specific properties */}
        {isTextField && (
          <>
            {/* Content Template */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t("settings.label.content_template")}
              </label>
              <textarea
                value={field.template || ''}
                onChange={(e) => handleUpdate({ template: e.target.value })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent font-mono text-sm"
                placeholder="{product_name}"
                rows={2}
              />
              <p className="mt-1 text-xs text-gray-500">
                {t("settings.label.text_template_hint")}
              </p>
            </div>

            <div className="grid grid-cols-2 gap-3">
               <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.font_size")}</label>
                  <NumberInput
                    value={field.fontSize}
                    onValueChange={(val) => handleUpdate({ fontSize: val })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    min="6"
                    max="48"
                  />
               </div>
               <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.font_family")}</label>
                  <select
                    value={field.fontFamily || 'Arial'}
                    onChange={(e) => handleUpdate({ fontFamily: e.target.value })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  >
                    <option value="Arial">{t('fonts.arial')}</option>
                    <option value="Microsoft YaHei">{t('fonts.microsoft_ya_hei')}</option>
                    <option value="Segoe UI">{t('fonts.segoe_u_i')}</option>
                    <option value="Times New Roman">{t('fonts.times_new_roman')}</option>
                    <option value="Courier New">{t('fonts.courier_new')}</option>
                  </select>
               </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.font_style")}</label>
              <select
                value={field.fontWeight || 'normal'}
                onChange={(e) => handleUpdate({ fontWeight: e.target.value })}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value="normal">{t("settings.label.style_regular")}</option>
                <option value="bold">{t("settings.label.style_bold")}</option>
              </select>
            </div>

            <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.text_align")}</label>
                  <div className="flex gap-1">
                    {(['left', 'center', 'right'] as TextAlign[]).map((align) => (
                      <button
                        key={align}
                        onClick={() => handleUpdate({ alignment: align })}
                        title={t(`settings.align${align.charAt(0).toUpperCase() + align.slice(1)}`) || align}
                        className={`flex-1 p-2 rounded-lg border transition-all flex items-center justify-center ${
                          (field.alignment || 'left') === align
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
                  <label className="block text-sm font-medium text-gray-700 mb-1">{t("settings.label.vertical_align")}</label>
                  <div className="flex gap-1">
                    {(['top', 'middle', 'bottom'] as VerticalAlign[]).map((align) => (
                      <button
                        key={align}
                        onClick={() => handleUpdate({ verticalAlign: align })}
                        title={t(`settings.align${align.charAt(0).toUpperCase() + align.slice(1)}`) || align}
                        className={`flex-1 p-2 rounded-lg border transition-all flex items-center justify-center ${
                          (field.verticalAlign ?? 'top') === align
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

        {/* Image/Barcode-specific properties */}
        {isImageField && (
          <div className="space-y-4">
             {/* Source Type */}
             <div>
               <label className="block text-sm font-medium text-gray-700 mb-1">
                 {t("settings.label.source_type")}
               </label>
               <select
                 value={field.sourceType || 'image'}
                 onChange={(e) => handleUpdate({ sourceType: e.target.value as 'productImage' | 'qrCode' | 'barcode' | 'image', template: '' })}
                 className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
               >
                 <option value="image">{t("settings.label.source_type_image")}</option>
                 <option value="productImage">{t("settings.label.source_type_product_image")}</option>
                 <option value="qrCode">{t("settings.label.source_type_qrcode")}</option>
                 <option value="barcode">{t("settings.label.source_type_barcode")}</option>
               </select>
             </div>

             {/* Static Image Upload (only for 'image' sourceType) */}
             {field.sourceType === 'image' && (
               <div>
                 <label className="block text-sm font-medium text-gray-700 mb-2">
                   {t("settings.label.static_image")}
                 </label>
                 {previewUrl ? (
                   <div className="relative group">
                     <img
                       src={previewUrl}
                       alt="Preview"
                       className="w-full h-32 object-contain bg-gray-100 rounded-lg border border-gray-200"
                     />
                     {field._pendingImagePath && (
                       <div className="absolute top-2 left-2 px-2 py-0.5 bg-amber-500 text-white text-xs rounded-full">
                         {t("settings.label.pending_upload")}
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
                         className="p-2 bg-white rounded-lg text-primary-600 hover:bg-primary-50"
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
                     <span className="text-sm font-medium">{t("settings.label.select_image")}</span>
                   </button>
                 )}
                 <p className="mt-1 text-xs text-gray-500">
                   {t("settings.label.static_image_hint")}
                 </p>
               </div>
             )}

             {/* Content Template (for dynamic sourceTypes) */}
             {field.sourceType !== 'image' && (
               <div>
                 <label className="block text-sm font-medium text-gray-700 mb-1">
                   {t("settings.label.content_template")}
                 </label>
                 <input
                    type="text"
                    value={field.template || ''}
                    onChange={(e) => handleUpdate({ template: e.target.value })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent font-mono text-sm"
                    placeholder={field.sourceType === 'productImage' ? '{product_image}' : field.sourceType === 'qrCode' ? '{product_code}' : '{barcode}'}
                 />
                 <p className="mt-1 text-xs text-gray-500">
                    {t("settings.label.image_template_hint")}
                 </p>
               </div>
             )}

             {/* Maintain Aspect Ratio */}
             <div className="flex items-center gap-2">
               <input
                 type="checkbox"
                 id="maintainAspectRatio"
                 checked={field.maintainAspectRatio ?? true}
                 onChange={(e) => handleUpdate({ maintainAspectRatio: e.target.checked })}
                 className="rounded border-gray-300 text-blue-600 focus:ring-blue-500 h-4 w-4"
               />
               <label htmlFor="maintainAspectRatio" className="text-sm text-gray-600 select-none cursor-pointer">
                 {t("settings.label.maintain_aspect_ratio")}
               </label>
             </div>
          </div>
        )}

        {/* Separator-specific properties */}
        {isSeparatorField && (
           <div className="bg-gray-50 p-3 rounded-lg text-sm text-gray-500">
             {t("settings.label.separator_hint")}
           </div>
        )}
      </div>
    </div>
  );
};
