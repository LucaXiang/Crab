import React, { useState } from 'react';
import { Upload, Download, FileArchive } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { save, open } from '@tauri-apps/plugin-dialog';
import { toast } from '@/presentation/components/Toast';
import { createTauriClient, invokeApi } from '@/infrastructure/api';
import { logger } from '@/utils/logger';
import { getErrorMessage } from '@/utils/error';

const getApi = () => createTauriClient();
import { useProductStore, useCategoryStore, useZoneStore, useTableStore } from '@/core/stores/resources';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';

export const DataTransfer: React.FC = () => {
  const { t } = useI18n();
  const [loading, setLoading] = useState(false);
  const refreshData = useSettingsStore((state) => state.refreshData);

  const handleExport = async () => {
    try {
      const path = await save({
        filters: [{
          name: 'ZIP Archive',
          extensions: ['zip']
        }],
        defaultPath: 'pos_data_backup.zip',
      });

      if (!path) return;

      setLoading(true);
      await invokeApi('export_data', { path });
      toast.success(t('settings.data_transfer.export.success'));
    } catch (error) {
      logger.error('Export failed', error);
      toast.error(getErrorMessage(error));
    } finally {
      setLoading(false);
    }
  };

  const handleImport = async () => {
    let selectedPath: string | null = null;
    try {
      selectedPath = await open({
        multiple: false,
        directory: false,
        filters: [{
          name: 'ZIP Archive',
          extensions: ['zip']
        }]
      });

      if (!selectedPath) return;

      setLoading(true);
      await invokeApi('import_data', { path: selectedPath });

      // Clear all caches to force reload
      useProductStore.getState().fetchAll(true);
      useCategoryStore.getState().fetchAll(true);
      useZoneStore.getState().fetchAll(true);
      useTableStore.getState().fetchAll(true);

      // Increment dataVersion to trigger reload in all components
      refreshData();
      toast.success(t('settings.data_transfer.import.success'));
    } catch (error) {
      logger.error('Import failed', error, { component: 'DataTransfer' });
      toast.error(getErrorMessage(error));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <div className="mb-8">
        <h2 className="text-2xl font-bold text-gray-800 mb-2">
          {t('settings.data_transfer.title')}
        </h2>
        <p className="text-gray-500">
          {t('settings.data_transfer.description')}
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Export Section */}
        <div className="bg-white p-6 rounded-lg border border-gray-200 shadow-sm hover:shadow-md transition-shadow">
          <div className="flex items-center gap-4 mb-4">
            <div className="p-3 bg-blue-50 rounded-full text-blue-600">
              <Download size={24} />
            </div>
            <h3 className="text-lg font-bold text-gray-800">
              {t('settings.data_transfer.export.title')}
            </h3>
          </div>
          <p className="text-gray-600 mb-6 min-h-[3rem]">
            {t('settings.data_transfer.export.description')}
          </p>
          <button
            onClick={handleExport}
            disabled={loading}
            className="w-full py-3 px-4 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium flex items-center justify-center gap-2 transition-colors disabled:opacity-50"
          >
            {loading ? (
              <span className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></span>
            ) : (
              <FileArchive size={20} />
            )}
            {t('settings.data_transfer.export.button')}
          </button>
        </div>

        {/* Import Section */}
        <div className="bg-white p-6 rounded-lg border border-gray-200 shadow-sm hover:shadow-md transition-shadow">
          <div className="flex items-center gap-4 mb-4">
            <div className="p-3 bg-green-50 rounded-full text-green-600">
              <Upload size={24} />
            </div>
            <h3 className="text-lg font-bold text-gray-800">
              {t('settings.data_transfer.import.title')}
            </h3>
          </div>
          <p className="text-gray-600 mb-6 min-h-[3rem]">
            {t('settings.data_transfer.import.description')}
          </p>
          <button
            onClick={handleImport}
            disabled={loading}
            className="w-full py-3 px-4 bg-green-600 hover:bg-green-700 text-white rounded-lg font-medium flex items-center justify-center gap-2 transition-colors disabled:opacity-50"
          >
            {loading ? (
              <span className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></span>
            ) : (
              <FileArchive size={20} />
            )}
            {t('settings.data_transfer.import.button')}
          </button>
        </div>
      </div>

      <div className="mt-8 p-4 bg-yellow-50 border border-yellow-100 rounded-lg text-yellow-800 text-sm">
        <div className="flex gap-2">
            <span className="font-bold">{t('settings.data_transfer.warning.title')}:</span>
            <ul className="list-disc list-inside space-y-1">
                <li>{t('settings.data_transfer.warning.item1')}</li>
                <li>{t('settings.data_transfer.warning.item2')}</li>
                <li>{t('settings.data_transfer.warning.item3')}</li>
            </ul>
        </div>
      </div>
    </div>
  );
};
