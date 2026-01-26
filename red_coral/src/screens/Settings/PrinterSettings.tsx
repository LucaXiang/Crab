import React, { useEffect, useState } from 'react';
import { Monitor, Server, LayoutTemplate } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { TabButton, LocalPrintersTab, PrintStationsTab, LabelTemplateManager } from './components/printer';

type TabType = 'local' | 'stations' | 'templates';

export const PrinterSettings: React.FC = React.memo(() => {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = useState<TabType>('local');

  // Data loading for local printers
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
    <div className="max-w-[100rem] mx-auto space-y-6 pb-10">
      {/* Header Section */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-gray-200 pb-6">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 tracking-tight">
            {t('settings.printer.title')}
          </h2>
          <p className="text-gray-500 mt-1">{t('settings.printer.description')}</p>
        </div>

        {/* Tab Navigation */}
        <div className="flex bg-gray-100 p-1 rounded-xl">
          <TabButton
            active={activeTab === 'local'}
            onClick={() => setActiveTab('local')}
            icon={Monitor}
            label={t('settings.printer.tab.local')}
          />
          <TabButton
            active={activeTab === 'stations'}
            onClick={() => setActiveTab('stations')}
            icon={Server}
            label={t('settings.printer.tab.stations')}
          />
          <TabButton
            active={activeTab === 'templates'}
            onClick={() => setActiveTab('templates')}
            icon={LayoutTemplate}
            label={t('settings.printer.tab.templates')}
          />
        </div>
      </div>

      {/* Content Area */}
      <div className="min-h-[31.25rem]">
        {activeTab === 'local' && <LocalPrintersTab printers={printers} loading={loading} />}
        {activeTab === 'stations' && <PrintStationsTab systemPrinters={printers} />}
        {activeTab === 'templates' && <LabelTemplateManager />}
      </div>
    </div>
  );
});

PrinterSettings.displayName = 'PrinterSettings';
