import React, { useState } from 'react';
import { Printer, Tag, ChefHat, AlertCircle, Settings, Info, DollarSign, Play } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import {
  useReceiptPrinter,
  useLabelPrinter,
  useKitchenPrinter,
  useCashDrawerPrinter,
  usePrinterActions,
  useAutoOpenCashDrawerAfterReceipt,
} from '@/core/stores/ui';
import { openCashDrawer } from '@/infrastructure/print/printService';
import { PrinterSelect } from './PrinterSelect';
import { KitchenPrinterList } from './KitchenPrinterList';

interface HardwareSettingsProps {
  printers: string[];
  loading: boolean;
}

export const HardwareSettings: React.FC<HardwareSettingsProps> = ({ printers, loading }) => {
  const { t } = useI18n();
  const { setReceiptPrinter, setLabelPrinter, setKitchenPrinter, setCashDrawerPrinter, setAutoOpenCashDrawerAfterReceipt } = usePrinterActions();

  const receiptPrinter = useReceiptPrinter();
  const labelPrinter = useLabelPrinter();
  const kitchenPrinter = useKitchenPrinter();
  const cashDrawerPrinter = useCashDrawerPrinter();
  const autoOpenCashDrawerAfterReceipt = useAutoOpenCashDrawerAfterReceipt();
  const [showHierarchyInfo, setShowHierarchyInfo] = useState(false);
  const [testingCashDrawer, setTestingCashDrawer] = useState(false);

  // 钱箱使用的打印机：如果设置了专用钱箱打印机则使用，否则使用收据打印机
  const effectiveCashDrawerPrinter = cashDrawerPrinter || receiptPrinter;

  const handleTestCashDrawer = async () => {
    if (!effectiveCashDrawerPrinter) return;
    setTestingCashDrawer(true);
    try {
      await openCashDrawer(effectiveCashDrawerPrinter);
      toast.success(t('settings.printer.cash_drawer.test_success'));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(`${t('settings.printer.cash_drawer.test_failed')}: ${message}`);
    } finally {
      setTestingCashDrawer(false);
    }
  };

  // 打印功能启用状态：由"是否配置了打印机"决定
  // 设计决策：
  // - 简单直观：有打印机 = 启用，无打印机 = 禁用
  // - 与服务端 PrintDestination 逻辑一致（通过 is_active 字段控制）
  // - 避免额外的开关状态管理，减少用户困惑
  const isLabelPrintEnabled = !!labelPrinter;
  const isKitchenPrintEnabled = !!kitchenPrinter;

  return (
    <div className="grid grid-cols-1 xl:grid-cols-3 gap-8 items-start animate-in fade-in duration-300">
      {/* Left Column: Main Station Printers */}
      <div className="xl:col-span-1 space-y-6">
        <div className="flex items-center gap-2 text-gray-800 font-bold text-lg mb-2">
          <Settings size={20} className="text-gray-400" />
          {t('settings.printer.form.main_station')}
        </div>

        <div className="space-y-4">
          <PrinterSelect
            label={t('settings.printer.form.receipt_printer')}
            description={t('settings.printer.form.receipt_printer_desc')}
            icon={Printer}
            value={receiptPrinter}
            onChange={setReceiptPrinter}
            printers={printers}
            loading={loading}
            badge={<span className="text-[0.625rem] bg-gray-100 text-gray-600 px-1.5 rounded uppercase font-bold tracking-wider">{t('settings.printer.badge.pos')}</span>}
          />

          {/* Cash Drawer Section */}
          <div className="bg-white rounded-xl border border-gray-200 p-4 space-y-4 shadow-sm hover:border-green-300 transition-all duration-300">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2.5 bg-green-50 text-green-600 rounded-lg">
                  <DollarSign size={20} />
                </div>
                <div>
                   <div className="font-bold text-gray-800">{t('settings.printer.form.cash_drawer_printer')}</div>
                   <div className="text-xs text-gray-500 mt-0.5">{t('settings.printer.form.cash_drawer_printer_desc')}</div>
                </div>
              </div>
            </div>

            <div className="space-y-3">
              {/* Use Receipt Printer Checkbox */}
              <label className="flex items-center gap-2 cursor-pointer group">
                <input
                  type="checkbox"
                  className="w-4 h-4 text-green-600 border-gray-300 rounded focus:ring-green-500"
                  checked={!cashDrawerPrinter}
                  onChange={(e) => {
                    if (e.target.checked) {
                      setCashDrawerPrinter(null);
                    }
                  }}
                />
                <span className="text-sm text-gray-600 group-hover:text-gray-900">
                  {t('settings.printer.cash_drawer.use_receipt_printer')}
                </span>
              </label>

              {/* Custom Printer Select (only if not using receipt printer) */}
              {cashDrawerPrinter !== null && (
                <div className="animate-in fade-in slide-in-from-top-1 duration-200">
                  <select
                    value={cashDrawerPrinter || ''}
                    onChange={(e) => setCashDrawerPrinter(e.target.value || null)}
                    className="w-full border border-gray-200 rounded-xl p-2.5 pl-3 pr-10 bg-gray-50 text-sm font-medium text-gray-700 focus:outline-none focus:ring-2 focus:ring-green-100 focus:border-green-500 transition-all cursor-pointer hover:bg-white appearance-none"
                  >
                    <option value="">{t('settings.printer.form.select_printer_placeholder')}</option>
                    {printers.map((p) => (
                      <option key={p} value={p}>{p}</option>
                    ))}
                  </select>
                </div>
              )}

              {/* Auto Open After Receipt Option */}
              <label className="flex items-center gap-2 cursor-pointer group">
                <input
                  type="checkbox"
                  className="w-4 h-4 text-green-600 border-gray-300 rounded focus:ring-green-500"
                  checked={autoOpenCashDrawerAfterReceipt}
                  onChange={(e) => setAutoOpenCashDrawerAfterReceipt(e.target.checked)}
                />
                <span className="text-sm text-gray-600 group-hover:text-gray-900">
                  {t('settings.printer.cash_drawer.auto_open_after_receipt')}
                </span>
              </label>

              {/* Test Button */}
              <button
                onClick={handleTestCashDrawer}
                disabled={!effectiveCashDrawerPrinter || testingCashDrawer || loading}
                className="w-full flex items-center justify-center gap-2 px-4 py-2.5 bg-green-50 text-green-700 rounded-xl font-medium text-sm hover:bg-green-100 disabled:opacity-50 disabled:cursor-not-allowed transition-all"
              >
                {testingCashDrawer ? (
                  <>
                    <div className="w-4 h-4 border-2 border-green-300 border-t-green-600 rounded-full animate-spin" />
                    {t('settings.printer.cash_drawer.testing')}
                  </>
                ) : (
                  <>
                    <Play size={16} />
                    {t('settings.printer.cash_drawer.test')}
                  </>
                )}
              </button>

              {/* Show which printer will be used */}
              {effectiveCashDrawerPrinter && (
                <div className="text-xs text-gray-500 text-center">
                  {effectiveCashDrawerPrinter}
                </div>
              )}
            </div>
          </div>

          {/* Label Printer Section with Toggle */}
          <div className="bg-white rounded-xl border border-gray-200 p-4 space-y-4 shadow-sm hover:border-blue-300 transition-all duration-300">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2.5 bg-amber-50 text-amber-600 rounded-lg">
                  <Tag size={20} />
                </div>
                <div>
                   <div className="font-bold text-gray-800">{t('settings.printer.label_printing')}</div>
                   <div className="text-xs text-gray-500 mt-0.5">{t('settings.printer.form.label_printer_desc')}</div>
                </div>
              </div>

              <label className="relative inline-flex items-center cursor-pointer group">
                <input
                  type="checkbox"
                  className="sr-only peer"
                  checked={isLabelPrintEnabled}
                  onChange={(e) => {
                    if (!e.target.checked) setLabelPrinter(null);
                  }}
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
                         {t('settings.printer.message.loading_printers')}
                      </div>
                    ) : printers.length === 0 ? (
                      <div className="w-full border border-amber-200 rounded-xl p-2.5 bg-amber-50 text-amber-600 text-sm flex items-center gap-2">
                        <AlertCircle size={16} /> {t('settings.printer.message.no_printers')}
                      </div>
                    ) : (
                      <>
                        <label className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1.5 flex items-center justify-between">
                          {t('settings.printer.form.target_printer')}
                          <span className="text-[0.625rem] bg-amber-100 text-amber-700 px-1.5 rounded uppercase font-bold tracking-wider">{t('settings.printer.badge.label')}</span>
                        </label>
                        <select
                          value={labelPrinter || ''}
                          onChange={(e) => setLabelPrinter(e.target.value || null)}
                          className="w-full border border-gray-200 rounded-xl p-2.5 pl-3 pr-10 bg-gray-50 text-sm font-medium text-gray-700 focus:outline-none focus:ring-2 focus:ring-amber-100 focus:border-amber-500 transition-all cursor-pointer hover:bg-white appearance-none"
                        >
                          <option value="">{t('settings.printer.form.select_printer_placeholder')}</option>
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
                    <div className="mt-2 text-xs text-primary-600 flex items-center gap-1.5 bg-primary-50 p-2 rounded-lg border border-primary-100 animate-pulse">
                      <AlertCircle size={14} />
                      {t('settings.printer.message.printer_unavailable')}
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
            {t('settings.printer.kitchen_printing.title')}
          </div>

          {/* Toggle Switch */}
          <label className="relative inline-flex items-center cursor-pointer group">
            <input
              type="checkbox"
              className="sr-only peer"
              checked={isKitchenPrintEnabled}
              onChange={(e) => {
                if (!e.target.checked) setKitchenPrinter(null);
              }}
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
                        <h4 className="text-sm font-bold text-blue-900">{t('settings.printer.routing_system.title')}</h4>
                        <span className="text-[0.625rem] text-blue-500 uppercase font-bold tracking-wider border border-blue-200 px-1.5 rounded bg-white">
                          {showHierarchyInfo ? (t('common.action.hide')) : (t('common.label.details'))}
                        </span>
                    </div>
                    <p className="text-xs text-blue-700 mt-1">
                      {t('settings.printer.routing_system.summary')}
                    </p>
                  </div>
                </div>

                {/* Collapsible Info */}
                {showHierarchyInfo && (
                  <div className="mt-4 pl-11 pr-2 pb-2 text-xs text-blue-800 space-y-3 animate-in fade-in duration-200">
                    <div className="p-3 bg-white/60 rounded-xl border border-blue-100">
                      <p className="font-bold mb-1 text-blue-900">{t('settings.printer.routing_system.hierarchy')}</p>
                      <div className="flex items-center gap-2 text-blue-600/80">
                          <span className="font-mono bg-blue-50 px-1 rounded">{t('settings.printer.routing_system.level_product')}</span>
                          <span className="text-gray-400">→</span>
                          <span className="font-mono bg-blue-50 px-1 rounded">{t('settings.printer.routing_system.level_category')}</span>
                          <span className="text-gray-400">→</span>
                          <span className="font-mono bg-blue-50 px-1 rounded">{t('settings.printer.routing_system.level_global')}</span>
                      </div>
                    </div>
                    <div className="p-3 bg-white/60 rounded-xl border border-blue-100">
                        <p className="font-bold mb-1 text-blue-900">{t('settings.printer.routing_system.priority')}</p>
                        <p className="opacity-80">
                          {t('settings.printer.routing_system.switch_hierarchy')}
                        </p>
                    </div>
                  </div>
                )}
              </div>

              <div className="p-6 space-y-8">
                <PrinterSelect
                  label={t('settings.printer.form.default_global_printer')}
                  description={t('settings.printer.form.default_global_printer_desc')}
                  icon={Printer}
                  value={kitchenPrinter}
                  onChange={setKitchenPrinter}
                  printers={printers}
                  loading={loading}
                  badge={<span className="text-[0.625rem] bg-indigo-100 text-indigo-700 px-1.5 rounded uppercase font-bold tracking-wider">{t('settings.printer.badge.fallback')}</span>}
                />

                <div className="relative">
                  <div className="absolute inset-0 flex items-center" aria-hidden="true">
                    <div className="w-full border-t border-gray-100"></div>
                  </div>
                  <div className="relative flex justify-center">
                    <span className="bg-white px-3 text-xs font-medium text-gray-400 uppercase tracking-wider">{t('settings.printer.routing_system.stations')}</span>
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
              <h3 className="text-lg font-bold text-gray-900 mb-2">{t('settings.printer.kitchen_printing.disabled')}</h3>
              <p className="text-gray-500 max-w-md mx-auto mb-6">
                {t('settings.printer.kitchen_printing.enable_to_configure')}
              </p>
              <p className="text-sm text-gray-400">
                {t('settings.printer.kitchen_printing.select_printer_to_enable')}
              </p>
            </div>
        )}
      </div>
    </div>
  );
};
