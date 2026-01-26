import React, { useState } from 'react';
import { Printer, Tag, DollarSign, Play, AlertCircle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import {
  useReceiptPrinter,
  useLabelPrinter,
  useCashDrawerPrinter,
  usePrinterActions,
  useAutoOpenCashDrawerAfterReceipt,
} from '@/core/stores/ui';
import { openCashDrawer } from '@/infrastructure/print/printService';

interface LocalPrintersTabProps {
  printers: string[];
  loading: boolean;
}

interface PrinterCardProps {
  title: string;
  description: string;
  icon: React.ElementType;
  iconColor: string;
  bgColor: string;
  borderColor: string;
  value: string | null;
  onChange: (value: string | null) => void;
  printers: string[];
  loading: boolean;
  badge?: React.ReactNode;
  children?: React.ReactNode;
}

const PrinterCard: React.FC<PrinterCardProps> = ({
  title,
  description,
  icon: Icon,
  iconColor,
  bgColor,
  borderColor,
  value,
  onChange,
  printers,
  loading,
  badge,
  children,
}) => {
  const { t } = useI18n();

  return (
    <div className={`bg-white rounded-2xl border ${borderColor} p-5 shadow-sm hover:shadow-md transition-all`}>
      <div className="flex items-start gap-4 mb-4">
        <div className={`p-3 ${bgColor} rounded-xl`}>
          <Icon size={24} className={iconColor} />
        </div>
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <h3 className="font-bold text-gray-900">{title}</h3>
            {badge}
          </div>
          <p className="text-sm text-gray-500 mt-1">{description}</p>
        </div>
      </div>

      <div className="space-y-3">
        {loading ? (
          <div className="w-full border border-gray-100 rounded-xl p-3 bg-gray-50 text-gray-400 text-sm flex items-center gap-2">
            <div className="w-4 h-4 border-2 border-gray-200 border-t-blue-500 rounded-full animate-spin" />
            {t('settings.printer.message.loading_printers')}
          </div>
        ) : printers.length === 0 ? (
          <div className="w-full border border-amber-200 rounded-xl p-3 bg-amber-50 text-amber-600 text-sm flex items-center gap-2">
            <AlertCircle size={16} />
            {t('settings.printer.message.no_printers')}
          </div>
        ) : (
          <select
            value={value || ''}
            onChange={(e) => onChange(e.target.value || null)}
            className="w-full border border-gray-200 rounded-xl p-3 bg-gray-50 text-sm font-medium text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-100 focus:border-blue-500 transition-all cursor-pointer hover:bg-white"
          >
            <option value="">{t('settings.printer.form.select_printer_placeholder')}</option>
            {printers.map((p) => (
              <option key={p} value={p}>{p}</option>
            ))}
          </select>
        )}

        {value && !loading && !printers.includes(value) && (
          <div className="text-xs text-red-600 flex items-center gap-1.5 bg-red-50 p-2 rounded-lg border border-red-100">
            <AlertCircle size={14} />
            {t('settings.printer.message.printer_unavailable')}
          </div>
        )}

        {children}
      </div>
    </div>
  );
};

export const LocalPrintersTab: React.FC<LocalPrintersTabProps> = ({ printers, loading }) => {
  const { t } = useI18n();
  const { setReceiptPrinter, setLabelPrinter, setCashDrawerPrinter, setAutoOpenCashDrawerAfterReceipt } = usePrinterActions();

  const receiptPrinter = useReceiptPrinter();
  const labelPrinter = useLabelPrinter();
  const cashDrawerPrinter = useCashDrawerPrinter();
  const autoOpenCashDrawerAfterReceipt = useAutoOpenCashDrawerAfterReceipt();

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

  return (
    <div className="space-y-6 animate-in fade-in duration-300">
      {/* 说明卡片 */}
      <div className="bg-blue-50 border border-blue-100 rounded-xl p-4">
        <p className="text-sm text-blue-700">
          {t('settings.printer.local_printers.description')}
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
        {/* 收据打印机 */}
        <PrinterCard
          title={t('settings.printer.form.receipt_printer')}
          description={t('settings.printer.form.receipt_printer_desc')}
          icon={Printer}
          iconColor="text-blue-600"
          bgColor="bg-blue-50"
          borderColor="border-gray-200 hover:border-blue-300"
          value={receiptPrinter}
          onChange={setReceiptPrinter}
          printers={printers}
          loading={loading}
          badge={
            <span className="text-[0.625rem] bg-blue-100 text-blue-700 px-2 py-0.5 rounded-full font-bold uppercase">
              POS
            </span>
          }
        />

        {/* 标签打印机 */}
        <PrinterCard
          title={t('settings.printer.label_printing')}
          description={t('settings.printer.form.label_printer_desc')}
          icon={Tag}
          iconColor="text-amber-600"
          bgColor="bg-amber-50"
          borderColor="border-gray-200 hover:border-amber-300"
          value={labelPrinter}
          onChange={setLabelPrinter}
          printers={printers}
          loading={loading}
          badge={
            <span className="text-[0.625rem] bg-amber-100 text-amber-700 px-2 py-0.5 rounded-full font-bold uppercase">
              {t('settings.printer.badge.label')}
            </span>
          }
        />

        {/* 钱箱打印机 */}
        <PrinterCard
          title={t('settings.printer.form.cash_drawer_printer')}
          description={t('settings.printer.form.cash_drawer_printer_desc')}
          icon={DollarSign}
          iconColor="text-green-600"
          bgColor="bg-green-50"
          borderColor="border-gray-200 hover:border-green-300"
          value={cashDrawerPrinter}
          onChange={setCashDrawerPrinter}
          printers={printers}
          loading={loading}
        >
          {/* 使用收据打印机选项 */}
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

          {/* 打印后自动开钱箱 */}
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

          {/* 测试按钮 */}
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

          {/* 当前使用的打印机 */}
          {effectiveCashDrawerPrinter && (
            <div className="text-xs text-gray-500 text-center bg-gray-50 py-2 rounded-lg">
              {t('settings.printer.local_printers.current')}: {effectiveCashDrawerPrinter}
            </div>
          )}
        </PrinterCard>
      </div>
    </div>
  );
};
