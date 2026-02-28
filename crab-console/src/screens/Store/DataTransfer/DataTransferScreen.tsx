import React, { useRef, useState } from 'react';
import { Upload, Download, FileArchive } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { exportCatalog, importCatalog } from '@/infrastructure/api/data-transfer';
import { Spinner } from '@/presentation/components/ui/Spinner';

export const DataTransferScreen: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [confirmFile, setConfirmFile] = useState<File | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const handleExport = async () => {
    if (!token) return;
    setError(null); setSuccess(null);
    try {
      setLoading(true);
      await exportCatalog(token, storeId);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('data_transfer.export_error'));
    } finally {
      setLoading(false);
    }
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setConfirmFile(file);
  };

  const handleImportConfirm = async () => {
    if (!confirmFile || !token) return;
    setError(null); setSuccess(null);
    try {
      setLoading(true);
      await importCatalog(token, storeId, confirmFile);
      setSuccess(t('data_transfer.import_success'));
    } catch (err) {
      setError(err instanceof Error ? err.message : t('data_transfer.import_error'));
    } finally {
      setLoading(false);
      setConfirmFile(null);
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

      {error && (
        <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-700">{error}</div>
      )}
      {success && (
        <div className="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-sm text-green-700">{success}</div>
      )}

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
            {loading ? <Spinner className="w-5 h-5" /> : <FileArchive size={20} />}
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
            {loading ? <Spinner className="w-5 h-5" /> : <FileArchive size={20} />}
            {t('data_transfer.import_btn')}
            <input
              ref={fileRef}
              type="file"
              accept=".zip"
              onChange={handleFileSelect}
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

      {/* Import confirmation modal */}
      {confirmFile && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 max-w-md mx-4">
            <h3 className="text-lg font-semibold text-gray-900 mb-2">{t('data_transfer.import_confirm_title')}</h3>
            <p className="text-sm text-gray-600 mb-2">{t('data_transfer.import_confirm_desc')}</p>
            <p className="text-sm text-gray-500 mb-4">
              {confirmFile.name} ({(confirmFile.size / 1024).toFixed(1)} KB)
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => { setConfirmFile(null); if (fileRef.current) fileRef.current.value = ''; }}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleImportConfirm}
                disabled={loading}
                className="px-4 py-2 text-sm font-medium text-white bg-green-600 rounded-md hover:bg-green-700 disabled:opacity-50"
              >
                {t('common.action.confirm')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
