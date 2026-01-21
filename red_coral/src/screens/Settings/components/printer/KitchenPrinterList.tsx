import React, { useEffect, useState } from 'react';
import { Printer, ChefHat, Trash2, Edit2, Plus } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useKitchenPrinterStore } from '@/core/stores/resources';
import { PrinterEditModal } from './PrinterEditModal';

interface KitchenPrinterListProps {
  systemPrinters: string[];
}

export const KitchenPrinterList: React.FC<KitchenPrinterListProps> = ({ systemPrinters }) => {
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
  const [editingItem, setEditingItem] = useState<{ id: string; name?: string; printerName?: string; description?: string } | null>(null);

  useEffect(() => {
    loadKitchenPrinters();
  }, []);

  const handleSave = async (data: { name: string; printerName: string; description: string }) => {
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

  const openEdit = (item: { id: string; name?: string; printerName?: string; description?: string }) => {
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
          {t('settings.printer.kitchenStation.addStation')}
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
