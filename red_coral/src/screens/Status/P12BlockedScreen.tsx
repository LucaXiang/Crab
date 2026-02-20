import React, { useState } from 'react';
import { ShieldAlert, Power, RefreshCw, LogOut, Upload, FileKey, Lock, Eye, EyeOff, CheckCircle, AlertCircle } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import { useAppState, useBridgeStore, AppStateHelpers } from '@/core/stores/bridge';
import { logger } from '@/utils/logger';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { t } from '@/infrastructure/i18n';
import type { AppState } from '@/core/stores/bridge/useBridgeStore';

interface P12UploadResult {
  fingerprint: string;
  common_name: string;
  organization: string | null;
  tax_id: string | null;
  issuer: string;
  expires_at: number | null;
}

export const P12BlockedScreen: React.FC = () => {
  const navigate = useNavigate();
  const appState = useAppState();
  const exitTenant = useBridgeStore((s) => s.exitTenant);
  const [isChecking, setIsChecking] = useState(false);
  const [checkMessage, setCheckMessage] = useState<string | null>(null);
  const [showExitConfirm, setShowExitConfirm] = useState(false);

  // Upload form state
  const [p12Password, setP12Password] = useState('');
  const [p12FilePath, setP12FilePath] = useState<string | null>(null);
  const [p12FileName, setP12FileName] = useState<string | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [uploadSuccess, setUploadSuccess] = useState<P12UploadResult | null>(null);
  const [showP12Password, setShowP12Password] = useState(false);

  if (appState?.type !== 'ServerP12Blocked') {
    const target = AppStateHelpers.getRouteForState(appState);
    navigate(target, { replace: true });
    return null;
  }

  const { info } = appState.data;
  const isMissing = info.reason.code === 'Missing';

  const handleCloseApp = async () => {
    const appWindow = getCurrentWindow();
    await appWindow.close();
  };

  const handleExitTenant = async () => {
    try {
      await exitTenant();
      const newState = useBridgeStore.getState().appState;
      const route = AppStateHelpers.getRouteForState(newState);
      navigate(route, { replace: true });
    } catch (error) {
      logger.error('Exit tenant failed', error);
    }
  };

  const handleCheckP12 = async () => {
    setIsChecking(true);
    setCheckMessage(null);
    try {
      const newState = await invokeApi<AppState>('check_subscription');
      useBridgeStore.setState({ appState: newState });

      if (newState.type === 'ServerP12Blocked') {
        setCheckMessage(t('p12Blocked.still_blocked'));
      }
    } catch {
      setCheckMessage(t('p12Blocked.still_blocked'));
    } finally {
      setIsChecking(false);
    }
  };

  const handleSelectFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'P12 Certificate', extensions: ['p12', 'pfx'] }],
      });
      if (selected) {
        setP12FilePath(selected);
        const parts = selected.replace(/\\/g, '/').split('/');
        setP12FileName(parts[parts.length - 1]);
        setUploadError(null);
      }
    } catch (error) {
      logger.error('File dialog error', error);
    }
  };

  const handleUpload = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!p12FilePath || !p12Password) return;

    setIsUploading(true);
    setUploadError(null);
    setUploadSuccess(null);

    try {
      const result = await invokeApi<P12UploadResult>('upload_p12', {
        p12FilePath,
        p12Password,
      });

      setUploadSuccess(result);
      // Clear sensitive fields
      setP12Password('');
      setP12FilePath(null);
      setP12FileName(null);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setUploadError(msg);
    } finally {
      setIsUploading(false);
    }
  };

  const canSubmit = p12Password.trim() !== '' && p12FilePath !== null && !isUploading;

  return (
    <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
      {/* Close button */}
      <button
        onClick={handleCloseApp}
        className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
        title={t('common.dialog.close_app')}
      >
        <Power size={24} />
      </button>

      <div className="max-w-md w-full bg-white rounded-2xl shadow-lg p-8">
        {/* Icon + Title */}
        <div className="text-center mb-6">
          <div className="inline-flex items-center justify-center w-20 h-20 bg-orange-100 rounded-full mb-4">
            <ShieldAlert className="text-orange-500" size={48} />
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2">
            {t('p12Blocked.title')}
          </h1>
          <p className="text-lg text-gray-600">
            {isMissing
              ? t('p12Blocked.message.missing')
              : t('p12Blocked.message.expired')}
          </p>
        </div>

        {/* Status badge */}
        <div className="flex items-center justify-center mb-6">
          <span className={`px-3 py-1 rounded-full text-sm font-medium ${
            isMissing ? 'bg-orange-100 text-orange-700' : 'bg-red-100 text-red-700'
          }`}>
            {isMissing ? t('p12Blocked.status.missing') : t('p12Blocked.status.expired')}
          </span>
        </div>

        {/* Upload success */}
        {uploadSuccess && (
          <div className="bg-green-50 border border-green-200 rounded-xl p-4 mb-6">
            <div className="flex items-center gap-2 text-green-700 font-medium mb-1">
              <CheckCircle size={18} />
              {t('p12Blocked.upload.success')}
            </div>
            <p className="text-sm text-green-600">
              {t('p12Blocked.upload.success_detail', {
                issuer: uploadSuccess.issuer,
                common_name: uploadSuccess.common_name,
              })}
            </p>
          </div>
        )}

        {/* Upload Form — only file + password, no account credentials needed */}
        {!uploadSuccess && (
          <form onSubmit={handleUpload} className="space-y-4 mb-6">
            <h3 className="text-sm font-semibold text-gray-700 flex items-center gap-2">
              <FileKey size={16} className="text-primary-500" />
              {t('p12Blocked.upload.section_title')}
            </h3>

            {/* P12 File Picker */}
            <div className="space-y-1">
              <label className="text-xs font-medium text-gray-600">
                {t('p12Blocked.upload.file_label')}
              </label>
              <button
                type="button"
                onClick={handleSelectFile}
                disabled={isUploading}
                className={`w-full px-3 py-2.5 text-sm border-2 border-dashed rounded-xl text-left transition-colors flex items-center gap-2 ${
                  p12FileName
                    ? 'border-primary-300 bg-primary-50 text-primary-700'
                    : 'border-gray-200 text-gray-400 hover:border-gray-300 hover:bg-gray-50'
                } disabled:opacity-50`}
              >
                <Upload size={16} />
                {p12FileName
                  ? `${t('p12Blocked.upload.file_selected')}: ${p12FileName}`
                  : t('p12Blocked.upload.file_placeholder')}
              </button>
            </div>

            {/* P12 Password */}
            <div className="space-y-1">
              <label className="text-xs font-medium text-gray-600 flex items-center gap-1">
                <Lock size={12} />
                {t('p12Blocked.upload.p12_password_label')}
              </label>
              <div className="relative">
                <input
                  type={showP12Password ? 'text' : 'password'}
                  value={p12Password}
                  onChange={(e) => setP12Password(e.target.value)}
                  placeholder={t('p12Blocked.upload.p12_password_placeholder')}
                  className="w-full px-3 py-2.5 pr-10 text-sm border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
                  disabled={isUploading}
                />
                <button
                  type="button"
                  onClick={() => setShowP12Password(!showP12Password)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
                >
                  {showP12Password ? <EyeOff size={16} /> : <Eye size={16} />}
                </button>
              </div>
            </div>

            {/* Security notice */}
            <p className="text-xs text-gray-400 text-center">
              {t('p12Blocked.upload.security_notice')}
            </p>

            {/* Upload error */}
            {uploadError && (
              <div className="flex items-center gap-2 text-red-600 bg-red-50 p-3 rounded-xl border border-red-100">
                <AlertCircle size={16} className="shrink-0" />
                <span className="text-sm">{uploadError}</span>
              </div>
            )}

            {/* Submit */}
            <button
              type="submit"
              disabled={!canSubmit}
              className="w-full py-3 bg-primary-500 text-white font-bold rounded-xl hover:bg-primary-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isUploading ? (
                <>
                  <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  {t('p12Blocked.upload.uploading')}
                </>
              ) : (
                <>
                  <Upload size={20} />
                  {t('p12Blocked.upload.button_submit')}
                </>
              )}
            </button>
          </form>
        )}

        {/* Actions */}
        <div className="space-y-3">
          {uploadSuccess ? (
            /* Upload succeeded — confirm to proceed */
            <button
              onClick={handleCheckP12}
              disabled={isChecking}
              className="w-full py-3 bg-primary-500 text-white font-bold rounded-xl hover:bg-primary-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isChecking ? (
                <>
                  <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  {t('p12Blocked.rechecking')}
                </>
              ) : (
                <>
                  <CheckCircle size={20} />
                  {t('common.confirm')}
                </>
              )}
            </button>
          ) : (
            /* No upload yet — recheck button */
            <button
              onClick={handleCheckP12}
              disabled={isChecking}
              className="w-full py-3 bg-blue-500 text-white font-bold rounded-xl hover:bg-blue-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <RefreshCw size={20} className={isChecking ? 'animate-spin' : ''} />
              {isChecking
                ? t('p12Blocked.rechecking')
                : t('p12Blocked.button_recheck')}
            </button>
          )}

          {/* Check result */}
          {checkMessage && (
            <p className="text-sm text-center text-orange-600">{checkMessage}</p>
          )}

          {/* Exit tenant */}
          <div className="pt-3 border-t border-gray-100">
            {!showExitConfirm ? (
              <button
                onClick={() => setShowExitConfirm(true)}
                className="w-full py-3 text-gray-400 hover:text-red-500 font-medium rounded-xl hover:bg-red-50 transition-all flex items-center justify-center gap-2"
              >
                <LogOut size={18} />
                {t('p12Blocked.button_exit_tenant')}
              </button>
            ) : (
              <div className="space-y-2">
                <p className="text-sm text-center text-gray-500">
                  {t('p12Blocked.confirm_exit_tenant')}
                </p>
                <div className="flex gap-2">
                  <button
                    onClick={() => setShowExitConfirm(false)}
                    className="flex-1 py-2 bg-gray-100 text-gray-600 font-medium rounded-xl hover:bg-gray-200 transition-all"
                  >
                    {t('common.cancel')}
                  </button>
                  <button
                    onClick={handleExitTenant}
                    className="flex-1 py-2 bg-red-500 text-white font-medium rounded-xl hover:bg-red-600 transition-all"
                  >
                    {t('common.confirm')}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
