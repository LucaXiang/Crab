import React, { useEffect, useState } from 'react';
import {
  Printer, AlertCircle, ChefHat, Tag, Trash2, Edit2, Plus, Save, X, Info, Settings,
  Copy, LayoutTemplate, Check
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useReceiptPrinter,
  useLabelPrinter,
  useIsLabelPrintEnabled,
  useKitchenPrinter,
  useIsKitchenPrintEnabled,
  useUIActions,
  useActiveLabelTemplateId
} from '@/core/stores/ui/useUIStore';
import { useKitchenPrinterStore } from '@/core/stores/resources';
import {
  LabelTemplate,
  DEFAULT_LABEL_TEMPLATES,
} from '../../types/labelTemplate';
import { LabelEditorScreen } from './components/LabelEditorScreen';
import { ConfirmDialog } from '@/presentation/components/ui/ConfirmDialog';
import { toast } from '@/presentation/components/Toast';

// --- Shared Components ---

const TabButton = ({ 
  active, 
  onClick, 
  icon: Icon, 
  label 
}: { 
  active: boolean; 
  onClick: () => void; 
  icon: React.ElementType; 
  label: string; 
}) => (
  <button
    onClick={onClick}
    className={`flex items-center gap-2 px-5 py-2.5 rounded-xl text-sm font-bold transition-all ${
      active 
        ? 'bg-gray-900 text-white shadow-lg shadow-gray-200' 
        : 'bg-white text-gray-600 hover:bg-gray-50 border border-gray-200'
    }`}
  >
    <Icon size={18} />
    {label}
  </button>
);

// --- Printer Settings Section ---

const PrinterEditModal = ({ 
  isOpen, 
  onClose, 
  initialData, 
  onSave, 
  systemPrinters 
}: { 
  isOpen: boolean; 
  onClose: () => void; 
  initialData?: any; 
  onSave: (data: any) => Promise<void>; 
  systemPrinters: string[];
}) => {
  const { t } = useI18n();
  const [formData, setFormData] = useState({ name: '', printerName: '', description: '' });

  useEffect(() => {
    if (isOpen) {
      setFormData({
        name: initialData?.name || '',
        printerName: initialData?.printerName || '',
        description: initialData?.description || ''
      });
    }
  }, [isOpen, initialData]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/20 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="bg-white rounded-2xl shadow-xl w-full max-w-md overflow-hidden animate-in zoom-in-95 duration-200 border border-gray-100">
        <div className="px-6 py-4 border-b border-gray-100 flex justify-between items-center bg-gray-50/50">
          <h3 className="font-bold text-gray-800 flex items-center gap-2">
            {initialData ? <Edit2 size={18} className="text-blue-500" /> : <Plus size={18} className="text-blue-500" />}
            {initialData ? (t('settings.printer.kitchenStation.action.edit')) : (t('settings.printer.kitchenStation.action.add'))}
          </h3>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full text-gray-400 hover:text-gray-600 transition-colors">
            <X size={20} />
          </button>
        </div>
        
        <div className="p-6 space-y-4">
          <div>
            <label className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5 block">{t('common.name')}</label>
            <input
              className="w-full border border-gray-200 rounded-xl px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500 transition-all bg-gray-50 focus:bg-white"
              placeholder={t('common.namePlaceholder')}
              value={formData.name}
              onChange={e => setFormData({ ...formData, name: e.target.value })}
              autoFocus
            />
          </div>
          
          <div>
            <label className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5 block">{t('settings.printer.form.targetPrinter')}</label>
            <div className="relative">
              <select
                className="w-full border border-gray-200 rounded-xl px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500 transition-all bg-gray-50 focus:bg-white appearance-none"
                value={formData.printerName}
                onChange={e => setFormData({ ...formData, printerName: e.target.value })}
              >
                <option value="">{t('settings.printer.form.selectSystemPrinter')}</option>
                {systemPrinters.map(p => (
                  <option key={p} value={p}>{p}</option>
                ))}
              </select>
              <Printer className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none" size={16} />
            </div>
          </div>
          
          <div>
            <label className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5 block">{t('common.description')}</label>
            <input
              className="w-full border border-gray-200 rounded-xl px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500 transition-all bg-gray-50 focus:bg-white"
              placeholder={t('common.descriptionPlaceholder')}
              value={formData.description}
              onChange={e => setFormData({ ...formData, description: e.target.value })}
            />
          </div>
        </div>

        <div className="px-6 py-4 bg-gray-50 border-t border-gray-100 flex justify-end gap-3">
          <button 
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-600 hover:bg-gray-200 rounded-xl transition-colors"
          >
            {t('common.cancel')}
          </button>
          <button 
            onClick={() => {
              if (formData.name) {
                onSave(formData);
              }
            }}
            disabled={!formData.name}
            className="px-4 py-2 text-sm font-bold bg-blue-600 text-white hover:bg-blue-700 rounded-xl shadow-lg shadow-blue-200 transition-all flex items-center gap-2 disabled:opacity-50 disabled:shadow-none"
          >
            <Save size={16} />
            {t('common.save')}
          </button>
        </div>
      </div>
    </div>
  );
};

const PrinterSelect = ({
  label,
  icon: Icon,
  value,
  onChange,
  printers,
  loading,
  description,
  badge
}: {
  label: string,
  icon: React.ElementType,
  value: string | null,
  onChange: (val: string | null) => void,
  printers: string[],
  loading: boolean,
  description?: string,
  badge?: React.ReactNode
}) => {
  const { t } = useI18n();
  const isSelectedAvailable = value ? printers.includes(value) : false;

  return (
    <div className="group bg-white rounded-xl border border-gray-200 p-4 hover:border-blue-300 transition-all duration-300 shadow-sm hover:shadow-md">
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-3">
          <div className="p-2.5 bg-blue-50 text-blue-600 rounded-lg group-hover:scale-110 transition-transform duration-300">
            <Icon size={20} />
          </div>
          <div>
            <div className="font-bold text-gray-800 flex items-center gap-2">
              {label}
              {badge}
            </div>
            {description && (
              <p className="text-xs text-gray-500 mt-0.5">{description}</p>
            )}
          </div>
        </div>
      </div>

      <div className="relative">
        {loading ? (
          <div className="w-full border border-gray-100 rounded-xl p-2.5 bg-gray-50 text-gray-400 text-sm flex items-center gap-2">
             <div className="w-4 h-4 border-2 border-gray-200 border-t-blue-500 rounded-full animate-spin" />
             {t('settings.printer.message.loadingPrinters')}
          </div>
        ) : printers.length === 0 ? (
          <div className="w-full border border-amber-200 rounded-xl p-2.5 bg-amber-50 text-amber-600 text-sm flex items-center gap-2">
            <AlertCircle size={16} /> {t('settings.printer.message.noPrinters')}
          </div>
        ) : (
          <>
            <select
              value={value || ''}
              onChange={(e) => onChange(e.target.value || null)}
              className="w-full border border-gray-200 rounded-xl p-2.5 pl-3 pr-10 bg-gray-50 text-sm font-medium text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500 transition-all cursor-pointer hover:bg-white appearance-none"
            >
              <option value="">{t('settings.printer.form.selectPrinterPlaceholder')}</option>
              {printers.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
            <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-gray-400">
              <Settings size={14} />
            </div>
          </>
        )}
      </div>
      
      {!isSelectedAvailable && value && !loading && (
        <div className="mt-2 text-xs text-red-600 flex items-center gap-1.5 bg-red-50 p-2 rounded-lg border border-red-100 animate-pulse">
          <AlertCircle size={14} /> 
          {t('settings.printer.message.printerUnavailable')}
        </div>
      )}
    </div>
  );
};

const KitchenPrinterList = ({ systemPrinters }: { systemPrinters: string[] }) => {
  const { t } = useI18n();
  const {
    kitchenPrinters,
    loadKitchenPrinters,
    createKitchenPrinter,
    updateKitchenPrinter,
    deleteKitchenPrinter,
    isLoading
  } = useKitchenPrinterStore();

  const [modalOpen, setModalOpen] = useState(false);
  const [editingItem, setEditingItem] = useState<any>(null);

  useEffect(() => {
    loadKitchenPrinters();
  }, []);

  const handleSave = async (data: any) => {
    try {
      if (editingItem) {
        await updateKitchenPrinter({ id: editingItem.id, ...data });
      } else {
        await createKitchenPrinter(data);
      }
      setModalOpen(false);
      setEditingItem(null);
    } catch (e) {
      console.error(e);
    }
  };

  const openCreate = () => {
    setEditingItem(null);
    setModalOpen(true);
  };

  const openEdit = (item: any) => {
    setEditingItem(item);
    setModalOpen(true);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-base font-bold text-gray-800 flex items-center gap-2">
          {t('settings.printer.kitchenStation.title')}
          <span className="text-xs font-bold text-blue-600 bg-blue-50 px-2 py-0.5 rounded-full border border-blue-100">
            {kitchenPrinters.length}
          </span>
        </h3>
        <button
          onClick={openCreate}
          className="flex items-center gap-1.5 text-xs font-bold bg-gray-900 text-white px-3 py-2 rounded-lg hover:bg-black transition-all shadow-md active:scale-95"
        >
          <Plus size={14} />
          {t("settings.printer.addStation")}
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {kitchenPrinters.map(kp => (
          <div key={kp.id} className="group bg-white border border-gray-200 rounded-xl p-4 hover:border-blue-400 hover:shadow-md transition-all duration-300 relative overflow-hidden">
            <div className="absolute top-0 right-0 p-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1 bg-linear-to-l from-white via-white to-transparent pl-4">
              <button 
                onClick={() => openEdit(kp)} 
                className="p-1.5 hover:bg-blue-50 rounded-lg text-gray-400 hover:text-blue-600 transition-colors"
              >
                <Edit2 size={14} />
              </button>
              <button 
                onClick={() => deleteKitchenPrinter(kp.id)} 
                className="p-1.5 hover:bg-red-50 rounded-lg text-gray-400 hover:text-red-600 transition-colors"
              >
                <Trash2 size={14} />
              </button>
            </div>

            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <div className="w-8 h-8 rounded-full bg-indigo-50 flex items-center justify-center text-indigo-600">
                  <ChefHat size={16} />
                </div>
                <div>
                  <div className="font-bold text-sm text-gray-900">{kp.name}</div>
                  <div className="text-[10px] text-gray-400 font-medium uppercase tracking-wide">
                     {kp.description || (t('settings.printer.kitchenStation.defaultName'))}
                  </div>
                </div>
              </div>
              
              <div className="bg-gray-50 rounded-lg px-3 py-2 flex items-center gap-2 text-xs border border-gray-100">
                <Printer size={12} className={kp.printer_name ? "text-gray-500" : "text-red-400"} />
                <span className={`font-medium ${kp.printer_name ? "text-gray-700" : "text-red-500"}`}>
                  {kp.printer_name || (t('settings.printer.message.noPrinter'))}
                </span>
              </div>
            </div>
          </div>
        ))}

        {!isLoading && kitchenPrinters.length === 0 && (
          <div 
            onClick={openCreate}
            className="md:col-span-2 flex flex-col items-center justify-center py-8 bg-gray-50 rounded-xl border-2 border-dashed border-gray-200 hover:border-blue-300 hover:bg-blue-50/30 transition-all cursor-pointer group"
          >
            <div className="w-12 h-12 bg-white rounded-full shadow-sm flex items-center justify-center mb-2 group-hover:scale-110 transition-transform">
               <Plus size={24} className="text-gray-400 group-hover:text-blue-500" />
            </div>
            <p className="text-sm text-gray-500 font-medium group-hover:text-blue-600">
              {t('settings.printer.kitchenStation.noData')}
            </p>
          </div>
        )}
      </div>

      <PrinterEditModal
        isOpen={modalOpen}
        onClose={() => setModalOpen(false)}
        initialData={editingItem}
        onSave={handleSave}
        systemPrinters={systemPrinters}
      />
    </div>
  );
};

const HardwareSettings = ({ printers, loading }: { printers: string[], loading: boolean }) => {
  const { t } = useI18n();
  const { setReceiptPrinter, setLabelPrinter, setKitchenPrinter, setIsKitchenPrintEnabled, setIsLabelPrintEnabled } = useUIActions();

  const receiptPrinter = useReceiptPrinter();
  const labelPrinter = useLabelPrinter();
  const isLabelPrintEnabled = useIsLabelPrintEnabled();
  const kitchenPrinter = useKitchenPrinter();
  const isKitchenPrintEnabled = useIsKitchenPrintEnabled();
  const [showHierarchyInfo, setShowHierarchyInfo] = useState(false);

  return (
    <div className="grid grid-cols-1 xl:grid-cols-3 gap-8 items-start animate-in fade-in duration-300">
      {/* Left Column: Main Station Printers */}
      <div className="xl:col-span-1 space-y-6">
        <div className="flex items-center gap-2 text-gray-800 font-bold text-lg mb-2">
          <Settings size={20} className="text-gray-400" />
          {t('settings.printer.form.mainStation')}
        </div>
        
        <div className="space-y-4">
          <PrinterSelect
            label={t('settings.printer.form.receiptPrinter')}
            description={t('settings.printer.form.receiptPrinterDesc')}
            icon={Printer}
            value={receiptPrinter}
            onChange={setReceiptPrinter}
            printers={printers}
            loading={loading}
            badge={<span className="text-[10px] bg-gray-100 text-gray-600 px-1.5 rounded uppercase font-bold tracking-wider">{t('settings.printer.badge.pos')}</span>}
          />
          
          {/* Label Printer Section with Toggle */}
          <div className="bg-white rounded-xl border border-gray-200 p-4 space-y-4 shadow-sm hover:border-blue-300 transition-all duration-300">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2.5 bg-amber-50 text-amber-600 rounded-lg">
                  <Tag size={20} />
                </div>
                <div>
                   <div className="font-bold text-gray-800">{t('settings.printer.labelPrinting')}</div>
                   <div className="text-xs text-gray-500 mt-0.5">{t('settings.printer.form.labelPrinterDesc')}</div>
                </div>
              </div>
              
              <label className="relative inline-flex items-center cursor-pointer group">
                <input
                  type="checkbox"
                  className="sr-only peer"
                  checked={isLabelPrintEnabled}
                  onChange={(e) => setIsLabelPrintEnabled(e.target.checked)}
                />
                <div className="w-9 h-5 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-amber-100 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-amber-500 shadow-sm transition-colors"></div>
              </label>
            </div>

            {isLabelPrintEnabled && (
               <div className="animate-in fade-in slide-in-from-top-1 duration-200 pt-2 border-t border-gray-100">
                  <div className="relative">
                    {loading ? (
                      <div className="w-full border border-gray-100 rounded-xl p-2.5 bg-gray-50 text-gray-400 text-sm flex items-center gap-2">
                         <div className="w-4 h-4 border-2 border-gray-200 border-t-blue-500 rounded-full animate-spin" />
                         {t('settings.printer.message.loadingPrinters')}
                      </div>
                    ) : printers.length === 0 ? (
                      <div className="w-full border border-amber-200 rounded-xl p-2.5 bg-amber-50 text-amber-600 text-sm flex items-center gap-2">
                        <AlertCircle size={16} /> {t('settings.printer.message.noPrinters')}
                      </div>
                    ) : (
                      <>
                        <label className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5 flex items-center justify-between">
                          {t('settings.printer.form.targetPrinter')}
                          <span className="text-[10px] bg-amber-100 text-amber-700 px-1.5 rounded uppercase font-bold tracking-wider">{t('settings.printer.badge.label')}</span>
                        </label>
                        <select
                          value={labelPrinter || ''}
                          onChange={(e) => setLabelPrinter(e.target.value || null)}
                          className="w-full border border-gray-200 rounded-xl p-2.5 pl-3 pr-10 bg-gray-50 text-sm font-medium text-gray-700 focus:outline-none focus:ring-2 focus:ring-amber-100 focus:border-amber-500 transition-all cursor-pointer hover:bg-white appearance-none"
                        >
                          <option value="">{t('settings.printer.form.selectPrinterPlaceholder')}</option>
                          {printers.map((p) => (
                            <option key={p} value={p}>
                              {p}
                            </option>
                          ))}
                        </select>
                        <div className="absolute right-3 bottom-3 pointer-events-none text-gray-400">
                          <Settings size={14} />
                        </div>
                      </>
                    )}
                  </div>
                  
                  {labelPrinter && !loading && !printers.includes(labelPrinter) && (
                    <div className="mt-2 text-xs text-red-600 flex items-center gap-1.5 bg-red-50 p-2 rounded-lg border border-red-100 animate-pulse">
                      <AlertCircle size={14} /> 
                      {t('settings.printer.message.printerUnavailable')}
                    </div>
                  )}
               </div>
            )}
          </div>
        </div>
      </div>

      {/* Right Column: Kitchen Printing (Spans 2 columns) */}
      <div className="xl:col-span-2 space-y-6">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2 text-gray-800 font-bold text-lg">
            <ChefHat size={20} className="text-gray-400" />
            {t('settings.printer.kitchenPrinting.title')}
          </div>
          
          {/* Toggle Switch */}
          <label className="relative inline-flex items-center cursor-pointer group">
            <input
              type="checkbox"
              className="sr-only peer"
              checked={isKitchenPrintEnabled}
              onChange={(e) => setIsKitchenPrintEnabled(e.target.checked)}
            />
            <span className="mr-3 text-sm font-medium text-gray-600 group-hover:text-gray-900 transition-colors">
              {isKitchenPrintEnabled ? (t('common.enabled')) : (t('common.disabled'))}
            </span>
            <div className="relative w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-100 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600 shadow-sm"></div>
          </label>
        </div>
        
        {isKitchenPrintEnabled ? (
          <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden animate-in fade-in slide-in-from-bottom-4 duration-300">
              {/* Info Banner */}
              <div className="bg-blue-50/50 border-b border-blue-100 p-4">
                <div 
                  className="flex items-start gap-3 cursor-pointer select-none"
                  onClick={() => setShowHierarchyInfo(!showHierarchyInfo)}
                >
                  <div className="p-1.5 bg-blue-100 text-blue-600 rounded-lg shrink-0 mt-0.5">
                    <Info size={16} />
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                        <h4 className="text-sm font-bold text-blue-900">{t('settings.printer.routingSystem.title')}</h4>
                        <span className="text-[10px] text-blue-500 uppercase font-bold tracking-wider border border-blue-200 px-1.5 rounded bg-white">
                          {showHierarchyInfo ? (t('common.hide')) : (t('common.details'))}
                        </span>
                    </div>
                    <p className="text-xs text-blue-700 mt-1">
                      {t('settings.printer.routingSystem.summary')}
                    </p>
                  </div>
                </div>
                
                {/* Collapsible Info */}
                {showHierarchyInfo && (
                  <div className="mt-4 pl-11 pr-2 pb-2 text-xs text-blue-800 space-y-3 animate-in fade-in duration-200">
                    <div className="p-3 bg-white/60 rounded-xl border border-blue-100">
                      <p className="font-bold mb-1 text-blue-900">{t('settings.printer.routingSystem.hierarchy')}</p>
                      <div className="flex items-center gap-2 text-blue-600/80">
                          <span className="font-mono bg-blue-50 px-1 rounded">{t('settings.printer.routingSystem.levelProduct')}</span>
                          <span className="text-gray-400">→</span>
                          <span className="font-mono bg-blue-50 px-1 rounded">{t('settings.printer.routingSystem.levelCategory')}</span>
                          <span className="text-gray-400">→</span>
                          <span className="font-mono bg-blue-50 px-1 rounded">{t('settings.printer.routingSystem.levelGlobal')}</span>
                      </div>
                    </div>
                    <div className="p-3 bg-white/60 rounded-xl border border-blue-100">
                        <p className="font-bold mb-1 text-blue-900">{t('settings.printer.routingSystem.priority')}</p>
                        <p className="opacity-80">
                          {t('settings.printer.routingSystem.switchHierarchy')}
                        </p>
                    </div>
                  </div>
                )}
              </div>

              <div className="p-6 space-y-8">
                <PrinterSelect
                  label={t('settings.printer.form.defaultGlobalPrinter')}
                  description={t('settings.printer.form.defaultGlobalPrinterDesc')}
                  icon={Printer}
                  value={kitchenPrinter}
                  onChange={setKitchenPrinter}
                  printers={printers}
                  loading={loading}
                  badge={<span className="text-[10px] bg-indigo-100 text-indigo-700 px-1.5 rounded uppercase font-bold tracking-wider">{t('settings.printer.badge.fallback')}</span>}
                />

                <div className="relative">
                  <div className="absolute inset-0 flex items-center" aria-hidden="true">
                    <div className="w-full border-t border-gray-100"></div>
                  </div>
                  <div className="relative flex justify-center">
                    <span className="bg-white px-3 text-xs font-medium text-gray-400 uppercase tracking-wider">{t('settings.printer.routingSystem.stations')}</span>
                  </div>
                </div>
                
                <KitchenPrinterList systemPrinters={printers} />
              </div>
          </div>
        ) : (
            <div className="bg-gray-50 rounded-2xl border-2 border-dashed border-gray-200 p-12 text-center transition-all hover:bg-gray-50/80">
              <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mx-auto mb-4 text-gray-300">
                <ChefHat size={32} />
              </div>
              <h3 className="text-lg font-bold text-gray-900 mb-2">{t('settings.printer.kitchenPrinting.disabled')}</h3>
              <p className="text-gray-500 max-w-md mx-auto mb-6">
                {t('settings.printer.kitchenPrinting.enableToConfigure')}
              </p>
              <button 
                onClick={() => setIsKitchenPrintEnabled(true)}
                className="px-5 py-2.5 bg-gray-900 text-white rounded-xl font-bold hover:bg-black transition-all shadow-lg shadow-gray-200 active:scale-95"
              >
                {t('common.enable')}
              </button>
            </div>
        )}
      </div>
    </div>
  );
};

// --- Label Templates Section ---

const STORAGE_KEY = 'label_templates';

const LabelTemplateManager = () => {
  const { t } = useI18n();
  const [templates, setTemplates] = useState<LabelTemplate[]>([]);
  const [showNewTemplateDialog, setShowNewTemplateDialog] = useState(false);
  const [templateName, setTemplateName] = useState('');
  const [templateWidth, setTemplateWidth] = useState(40);
  const [templateHeight, setTemplateHeight] = useState(30);
  
  const activeTemplateId = useActiveLabelTemplateId();
  const { setActiveLabelTemplateId } = useUIActions();

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
      // Set default as active if none selected
      if (!activeTemplateId) {
        setActiveLabelTemplateId(defaultTemplate.id);
      }
    } else if (!activeTemplateId && templates.length > 0) {
       // Ensure one is selected if we have templates but no selection
       // Check if templates state is populated, wait, we just loaded storedTemplates.
       // storedTemplates is a string here.
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
    
    // Open in editor immediately
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
      toast.error(t('settings.printer.alert.deleteLastTemplate'));
      return;
    }
    
    setConfirmDialog({
      isOpen: true,
      title: t('settings.printer.alert.confirmDelete'),
      description: t('settings.printer.alert.confirmDeleteDesc'),
      onConfirm: () => {
        const updatedTemplates = templates.filter((t) => t.id !== templateId);
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
    const updatedTemplates = templates.map((t) =>
      t.id === updatedTemplate.id ? { ...updatedTemplate, updatedAt: new Date().toISOString() } : t
    );
    setTemplates(updatedTemplates);
    saveTemplatesToStorage(updatedTemplates);
    setEditingTemplate(updatedTemplate); // Update local state to reflect changes
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
                  title={t('settings.printer.template.action.duplicate')}
                >
                  <Copy size={16} />
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDeleteTemplate(template.id);
                  }}
                  className="p-2 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                  title={t('common.delete')}
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
              {t('settings.printer.template.action.editDesign')}
            </button>
          </div>
        );
        })}

        {/* New Template Card (Placeholder style) */}
        <button
          onClick={() => setShowNewTemplateDialog(true)}
          className="bg-gray-50 rounded-2xl border-2 border-dashed border-gray-200 p-5 hover:bg-gray-100 hover:border-gray-300 transition-all flex flex-col items-center justify-center text-gray-400 gap-3 min-h-[200px]"
        >
          <div className="w-12 h-12 rounded-full bg-white flex items-center justify-center shadow-sm">
            <Plus size={24} />
          </div>
          <span className="font-medium">{t('settings.printer.template.action.create')}</span>
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
                  placeholder={t('settings.printer.template.form.namePlaceholder')}
                  className="w-full border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500"
                  autoFocus
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5">{t('settings.printer.template.form.widthMm')}</label>
                  <input
                    type="number"
                    value={templateWidth}
                    onChange={(e) => setTemplateWidth(parseFloat(e.target.value) || 40)}
                    className="w-full border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500"
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5">{t('settings.printer.template.form.heightMm')}</label>
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
                {t('common.cancel')}
              </button>
              <button
                onClick={handleCreateTemplate}
                disabled={!templateName.trim()}
                className="flex-1 px-4 py-2 text-sm font-bold bg-blue-600 text-white rounded-xl hover:bg-blue-700 transition-colors shadow-lg shadow-blue-200 disabled:opacity-50 disabled:shadow-none"
              >
                {t('common.create')}
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
        confirmText={t('common.confirm')}
        cancelText={t('common.cancel')}
        variant="danger"
      />
    </div>
  );
};

// --- Main Page Component ---

export const PrinterSettings: React.FC = React.memo(() => {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = useState<'hardware' | 'templates'>('hardware');
  
  // Data loading for hardware tab
  const [printers, setPrinters] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let mounted = true;
    const load = async () => {
      try {
        setLoading(true);
        const { listPrinters } = await import('../../services/printService');
        const result = await listPrinters();
        if (mounted) setPrinters(result);
      } finally {
        if (mounted) setLoading(false);
      }
    };
    load();
    return () => {
      mounted = false;
    };
  }, []);

  return (
    <div className="max-w-[1600px] mx-auto space-y-6 pb-10">
      
      {/* Header Section */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-gray-200 pb-6">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 tracking-tight">{t('settings.printer.title')}</h2>
          <p className="text-gray-500 mt-1">{t('settings.printer.description')}</p>
        </div>
        
        {/* Tab Navigation */}
        <div className="flex bg-gray-100 p-1 rounded-xl">
           <TabButton 
             active={activeTab === 'hardware'} 
             onClick={() => setActiveTab('hardware')} 
             icon={Printer} 
             label={t('settings.printer.hardware')} 
           />
           <TabButton 
             active={activeTab === 'templates'} 
             onClick={() => setActiveTab('templates')} 
             icon={LayoutTemplate} 
             label={t('settings.label.templates')} 
           />
        </div>
      </div>

      {/* Content Area */}
      <div className="min-h-[500px]">
        {activeTab === 'hardware' ? (
          <HardwareSettings printers={printers} loading={loading} />
        ) : (
          <LabelTemplateManager />
        )}
      </div>
    </div>
  );
});

PrinterSettings.displayName = 'PrinterSettings';
