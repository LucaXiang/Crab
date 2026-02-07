import React, { useEffect, useState } from 'react';
import { Printer, ChefHat, Trash2, Edit2, Plus } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { usePrintDestinationStore } from '@/core/stores/resources';
import { PrinterEditModal } from './PrinterEditModal';

interface KitchenPrinterListProps {
  systemPrinters: string[];
}

export const KitchenPrinterList: React.FC<KitchenPrinterListProps> = ({ systemPrinters }) => {
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
            ? [{
                printer_type: 'driver' as const,
                printer_format: 'escpos' as const,
                driver_name: data.driverName,
                priority: 1,
                is_active: true,
              }]
            : []
          : data.ip
            ? [{
                printer_type: 'network' as const,
                printer_format: 'escpos' as const,
                ip: data.ip,
                port: data.port || 9100,
                priority: 1,
                is_active: true,
              }]
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

  const openEdit = (item: typeof items[0]) => {
    const activePrinter = item.printers?.find(p => p.is_active);
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

  // Get display name for printers in a destination
  const getPrinterDisplay = (dest: typeof items[0]) => {
    if (!dest.printers || dest.printers.length === 0) {
      return null;
    }
    const activePrinters = dest.printers.filter(p => p.is_active);
    if (activePrinters.length === 0) {
      return null;
    }
    // Return the first active printer's name
    const first = activePrinters[0];
    if (first.printer_type === 'driver') {
      return first.driver_name;
    }
    return first.ip ? `${first.ip}:${first.port || 9100}` : null;
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-base font-bold text-gray-800 flex items-center gap-2">
          {t('settings.printer.kitchen_station.title')}
          <span className="text-xs font-bold text-blue-600 bg-blue-50 px-2 py-0.5 rounded-full border border-blue-100">
            {items.length}
          </span>
        </h3>
        <button
          onClick={openCreate}
          className="flex items-center gap-1.5 text-xs font-bold bg-gray-900 text-white px-3 py-2 rounded-lg hover:bg-black transition-all shadow-md active:scale-95"
        >
          <Plus size={14} />
          {t('settings.printer.kitchen_station.add_station')}
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {items.map(dest => {
          const printerDisplay = getPrinterDisplay(dest);
          return (
            <div key={dest.id} className="group bg-white border border-gray-200 rounded-xl p-4 hover:border-blue-400 hover:shadow-md transition-all duration-300 relative overflow-hidden">
              <div className="absolute top-0 right-0 p-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1 bg-linear-to-l from-white via-white to-transparent pl-4">
                <button
                  onClick={() => openEdit(dest)}
                  className="p-1.5 hover:bg-blue-50 rounded-lg text-gray-400 hover:text-blue-600 transition-colors"
                >
                  <Edit2 size={14} />
                </button>
                <button
                  onClick={() => remove(dest.id)}
                  className="p-1.5 hover:bg-primary-50 rounded-lg text-gray-400 hover:text-primary-600 transition-colors"
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
                    <div className="font-bold text-sm text-gray-900">{dest.name}</div>
                    <div className="text-[0.625rem] text-gray-400 font-medium uppercase tracking-wide">
                       {dest.description || (t('settings.printer.kitchen_station.default_name'))}
                    </div>
                  </div>
                </div>

                <div className="bg-gray-50 rounded-lg px-3 py-2 flex items-center gap-2 text-xs border border-gray-100">
                  <Printer size={12} className={printerDisplay ? "text-gray-500" : "text-primary-400"} />
                  <span className={`font-medium ${printerDisplay ? "text-gray-700" : "text-primary-500"}`}>
                    {printerDisplay || (t('settings.printer.message.no_printer'))}
                  </span>
                </div>
              </div>
            </div>
          );
        })}

        {!isLoading && items.length === 0 && (
          <div
            onClick={openCreate}
            className="md:col-span-2 flex flex-col items-center justify-center py-8 bg-gray-50 rounded-xl border-2 border-dashed border-gray-200 hover:border-blue-300 hover:bg-blue-50/30 transition-all cursor-pointer group"
          >
            <div className="w-12 h-12 bg-white rounded-full shadow-sm flex items-center justify-center mb-2 group-hover:scale-110 transition-transform">
               <Plus size={24} className="text-gray-400 group-hover:text-blue-500" />
            </div>
            <p className="text-sm text-gray-500 font-medium group-hover:text-blue-600">
              {t('settings.printer.kitchen_station.no_data')}
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
