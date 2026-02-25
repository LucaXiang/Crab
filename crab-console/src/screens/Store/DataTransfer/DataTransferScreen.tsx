import React, { useRef, useState } from 'react';
import { Upload, Download, FileArchive } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { exportCatalog, importCatalog } from '@/infrastructure/api/data-transfer';

export const DataTransferScreen: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const [loading, setLoading] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const handleExport = async () => {
    if (!token) return;
    try {
      setLoading(true);
      await exportCatalog(token, storeId);
    } catch (error) {
      alert(error instanceof Error ? error.message : 'Export failed');
    } finally {
      setLoading(false);
    }
  };

  const handleImport = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !token) return;
    try {
      setLoading(true);
      await importCatalog(token, storeId, file);
      alert(t('data_transfer.import_success'));
    } catch (error) {
      alert(error instanceof Error ? error.message : 'Import failed');
    } finally {
      setLoading(false);
      if (fileRef.current) fileRef.current.value = '';
    }
  };

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <div className="mb-8">
        <h2 className="text-2xl font-bold text-gray-800 mb-2">
          {t('data_transfer.title')}
        </h2>
        <p className="text-gray-500">
          {t('data_transfer.description')}
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Export */}
        <div className="bg-white p-6 rounded-lg border border-gray-200 shadow-sm hover:shadow-md transition-shadow">
          <div className="flex items-center gap-4 mb-4">
            <div className="p-3 bg-blue-50 rounded-full text-blue-600">
              <Download size={24} />
            </div>
            <h3 className="text-lg font-bold text-gray-800">
              {t('data_transfer.export_title')}
            </h3>
          </div>
          <p className="text-gray-600 mb-6 min-h-[3rem]">
            {t('data_transfer.export_desc')}
          </p>
          <button
            onClick={handleExport}
            disabled={loading}
            className="w-full py-3 px-4 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium flex items-center justify-center gap-2 transition-colors disabled:opacity-50"
          >
            {loading ? (
              <span className="animate-spin rounded-full h-5 w-5 border-b-2 border-white" />
            ) : (
              <FileArchive size={20} />
            )}
            {t('data_transfer.export_btn')}
          </button>
        </div>

        {/* Import */}
        <div className="bg-white p-6 rounded-lg border border-gray-200 shadow-sm hover:shadow-md transition-shadow">
          <div className="flex items-center gap-4 mb-4">
            <div className="p-3 bg-green-50 rounded-full text-green-600">
              <Upload size={24} />
            </div>
            <h3 className="text-lg font-bold text-gray-800">
              {t('data_transfer.import_title')}
            </h3>
          </div>
          <p className="text-gray-600 mb-6 min-h-[3rem]">
            {t('data_transfer.import_desc')}
          </p>
          <label className={`w-full py-3 px-4 rounded-lg font-medium flex items-center justify-center gap-2 transition-colors cursor-pointer ${
            loading ? 'bg-green-400 opacity-50' : 'bg-green-600 hover:bg-green-700'
          } text-white`}>
            {loading ? (
              <span className="animate-spin rounded-full h-5 w-5 border-b-2 border-white" />
            ) : (
              <FileArchive size={20} />
            )}
            {t('data_transfer.import_btn')}
            <input
              ref={fileRef}
              type="file"
              accept=".zip"
              onChange={handleImport}
              disabled={loading}
              className="hidden"
            />
          </label>
        </div>
      </div>

      <div className="mt-8 p-4 bg-yellow-50 border border-yellow-100 rounded-lg text-yellow-800 text-sm">
        <p className="font-bold mb-1">{t('data_transfer.warning_title')}</p>
        <ul className="list-disc list-inside space-y-1">
          <li>{t('data_transfer.warning1')}</li>
          <li>{t('data_transfer.warning2')}</li>
        </ul>
      </div>
    </div>
  );
};
