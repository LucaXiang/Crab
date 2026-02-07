import React, { useEffect, useState } from 'react';
import { Server, Printer, Trash2, Edit2, Plus, Wifi, Monitor, ChefHat, Receipt, Tag } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { usePrintDestinationStore } from '@/core/stores/resources';
import { PrinterEditModal } from './PrinterEditModal';
import type { PrintDestination, Printer as PrinterModel } from '@/core/domain/types/api';

interface PrintStationsTabProps {
  systemPrinters: string[];
}

// 获取打印机类型图标
const getPrinterTypeIcon = (printer: PrinterModel) => {
  if (printer.printer_type === 'network') {
    return <Wifi size={12} className="text-blue-500" />;
  }
  return <Monitor size={12} className="text-green-500" />;
};

// 获取打印机显示名称
const getPrinterDisplayName = (printer: PrinterModel) => {
  if (printer.printer_type === 'driver') {
    return printer.driver_name || 'Unknown Driver';
  }
  return printer.ip ? `${printer.ip}:${printer.port || 9100}` : 'Unknown';
};

// 获取用途图标
const getUsageIcon = (dest: PrintDestination) => {
  const name = dest.name.toLowerCase();
  if (name.includes('kitchen') || name.includes('厨房') || name.includes('后厨')) {
    return <ChefHat size={20} className="text-orange-600" />;
  }
  if (name.includes('receipt') || name.includes('收据') || name.includes('小票')) {
    return <Receipt size={20} className="text-blue-600" />;
  }
  if (name.includes('label') || name.includes('标签')) {
    return <Tag size={20} className="text-amber-600" />;
  }
  return <Server size={20} className="text-indigo-600" />;
};

export const PrintStationsTab: React.FC<PrintStationsTabProps> = ({ systemPrinters }) => {
  const { t } = useI18n();
  const items = usePrintDestinationStore((state) => state.items);
  const isLoading = usePrintDestinationStore((state) => state.isLoading);
  const { fetchAll, create, update, remove } = usePrintDestinationStore.getState();

  const [modalOpen, setModalOpen] = useState(false);
  const [editingItem, setEditingItem] = useState<{
    id?: number;
    name?: string;
    description?: string;
    printerType?: 'driver' | 'network';
    driverName?: string;
    ip?: string;
    port?: number;
  } | null>(null);

  useEffect(() => {
    fetchAll();
  }, []);

  const handleSave = async (data: {
    name: string;
    description: string;
    printerType: 'driver' | 'network';
    driverName: string;
    ip: string;
    port: number;
  }) => {
    try {
      // Build printers array based on type
      const printers =
        data.printerType === 'driver'
          ? data.driverName
            ? [
                {
                  printer_type: 'driver' as const,
                  printer_format: 'escpos' as const,
                  driver_name: data.driverName,
                  priority: 1,
                  is_active: true,
                },
              ]
            : []
          : data.ip
            ? [
                {
                  printer_type: 'network' as const,
                  printer_format: 'escpos' as const,
                  ip: data.ip,
                  port: data.port || 9100,
                  priority: 1,
                  is_active: true,
                },
              ]
            : [];

      if (editingItem?.id) {
        await update(editingItem.id, { name: data.name, description: data.description, printers });
      } else {
        await create({ name: data.name, description: data.description, printers });
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

  const openEdit = (item: PrintDestination) => {
    const activePrinter = item.printers?.find((p) => p.is_active);
    const printerType = activePrinter?.printer_type === 'network' ? 'network' : 'driver';

    setEditingItem({
      id: item.id,
      name: item.name,
      description: item.description,
      printerType,
      driverName: activePrinter?.printer_type === 'driver' ? activePrinter.driver_name : undefined,
      ip: activePrinter?.printer_type === 'network' ? activePrinter.ip : undefined,
      port: activePrinter?.printer_type === 'network' ? activePrinter.port : undefined,
    });
    setModalOpen(true);
  };

  return (
    <div className="space-y-6 animate-in fade-in duration-300">
      {/* 说明卡片 */}
      <div className="bg-indigo-50 border border-indigo-100 rounded-xl p-4 flex items-start gap-3">
        <Server size={20} className="text-indigo-600 mt-0.5 shrink-0" />
        <div>
          <h4 className="font-semibold text-indigo-900 mb-1">{t('settings.printer.print_stations.title')}</h4>
          <p className="text-sm text-indigo-700">{t('settings.printer.print_stations.description')}</p>
        </div>
      </div>

      {/* 头部操作栏 */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-sm text-gray-500">{t('common.label.total')}</span>
          <span className="text-lg font-bold text-gray-900">{items.length}</span>
          <span className="text-sm text-gray-500">{t('settings.printer.print_stations.unit')}</span>
        </div>
        <button
          onClick={openCreate}
          className="flex items-center gap-2 px-4 py-2.5 bg-indigo-600 text-white rounded-xl font-medium text-sm hover:bg-indigo-700 transition-all shadow-lg shadow-indigo-200"
        >
          <Plus size={18} />
          {t('settings.printer.print_stations.add')}
        </button>
      </div>

      {/* 站点列表 */}
      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
        {items.map((dest) => {
          const activePrinters = dest.printers?.filter((p) => p.is_active) || [];
          const hasActivePrinter = activePrinters.length > 0;

          return (
            <div
              key={dest.id}
              className={`group bg-white rounded-2xl border p-5 transition-all hover:shadow-lg ${
                hasActivePrinter
                  ? 'border-gray-200 hover:border-indigo-300'
                  : 'border-amber-200 bg-amber-50/30'
              }`}
            >
              {/* 头部 */}
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center gap-3">
                  <div
                    className={`w-10 h-10 rounded-xl flex items-center justify-center ${
                      hasActivePrinter ? 'bg-indigo-50' : 'bg-amber-100'
                    }`}
                  >
                    {getUsageIcon(dest)}
                  </div>
                  <div>
                    <h4 className="font-bold text-gray-900">{dest.name}</h4>
                    {dest.description && (
                      <p className="text-xs text-gray-500 mt-0.5">{dest.description}</p>
                    )}
                  </div>
                </div>

                {/* 操作按钮 */}
                <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button
                    onClick={() => openEdit(dest)}
                    className="p-2 hover:bg-blue-50 rounded-lg text-gray-400 hover:text-blue-600 transition-colors"
                  >
                    <Edit2 size={16} />
                  </button>
                  <button
                    onClick={() => remove(dest.id)}
                    className="p-2 hover:bg-primary-50 rounded-lg text-gray-400 hover:text-primary-600 transition-colors"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>

              {/* 绑定的打印机 */}
              <div className="space-y-2">
                <div className="text-xs font-semibold text-gray-400 uppercase tracking-wider">
                  {t('settings.printer.print_stations.bound_printers')}
                </div>

                {activePrinters.length > 0 ? (
                  <div className="space-y-1.5">
                    {activePrinters.map((printer, idx) => (
                      <div
                        key={idx}
                        className="flex items-center gap-2 bg-gray-50 rounded-lg px-3 py-2 text-sm"
                      >
                        {getPrinterTypeIcon(printer)}
                        <span className="font-medium text-gray-700">
                          {getPrinterDisplayName(printer)}
                        </span>
                        <span className="text-xs text-gray-400 ml-auto">
                          {printer.printer_type === 'driver'
                            ? t('settings.printer.print_stations.type_local')
                            : t('settings.printer.print_stations.type_network')}
                        </span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="flex items-center gap-2 bg-amber-50 border border-amber-200 rounded-lg px-3 py-2 text-sm text-amber-700">
                    <Printer size={14} />
                    {t('settings.printer.message.no_printer')}
                  </div>
                )}
              </div>

              {/* 状态标签 */}
              <div className="mt-4 pt-3 border-t border-gray-100 flex items-center justify-between">
                <span
                  className={`text-xs font-medium px-2 py-1 rounded-full ${
                    dest.is_active
                      ? 'bg-green-50 text-green-700'
                      : 'bg-gray-100 text-gray-500'
                  }`}
                >
                  {dest.is_active ? t('common.status.active') : t('common.status.inactive')}
                </span>
              </div>
            </div>
          );
        })}

        {/* 空状态 / 添加卡片 */}
        <button
          onClick={openCreate}
          className="flex flex-col items-center justify-center min-h-[200px] bg-gray-50 rounded-2xl border-2 border-dashed border-gray-200 hover:border-indigo-300 hover:bg-indigo-50/30 transition-all group"
        >
          <div className="w-12 h-12 bg-white rounded-full shadow-sm flex items-center justify-center mb-3 group-hover:scale-110 transition-transform">
            <Plus size={24} className="text-gray-400 group-hover:text-indigo-500" />
          </div>
          <span className="text-sm font-medium text-gray-500 group-hover:text-indigo-600">
            {t('settings.printer.print_stations.add')}
          </span>
        </button>
      </div>

      {/* 加载状态 */}
      {isLoading && items.length === 0 && (
        <div className="flex items-center justify-center py-12">
          <div className="w-8 h-8 border-2 border-gray-200 border-t-indigo-600 rounded-full animate-spin" />
        </div>
      )}

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
