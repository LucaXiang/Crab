import React, { useEffect, useState } from 'react';
import { Printer, LayoutTemplate } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { TabButton, HardwareSettings, LabelTemplateManager } from './components/printer';

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
        const { listPrinters } = await import('@/infrastructure/print/printService');
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
