import React, { useState } from 'react';
import { Printer, Tag, ChefHat, AlertCircle, Settings, Info } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useReceiptPrinter,
  useLabelPrinter,
  useIsLabelPrintEnabled,
  useKitchenPrinter,
  useIsKitchenPrintEnabled,
  useUIActions,
} from '@/core/stores/ui/useUIStore';
import { PrinterSelect } from './PrinterSelect';
import { KitchenPrinterList } from './KitchenPrinterList';

interface HardwareSettingsProps {
  printers: string[];
  loading: boolean;
}

export const HardwareSettings: React.FC<HardwareSettingsProps> = ({ printers, loading }) => {
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
              {isKitchenPrintEnabled ? (t('common.status.enabled')) : (t('common.status.disabled'))}
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
                          {showHierarchyInfo ? (t('common.action.hide')) : (t('common.label.details'))}
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
                {t('common.action.enable')}
              </button>
            </div>
        )}
      </div>
    </div>
  );
};
