import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Server, Wifi, AlertCircle, ChevronRight, Settings, Power, Shield,
  Monitor, RefreshCw, ExternalLink, UserCheck, ShieldAlert, LogOut,
  Upload, FileKey, Lock, Eye, EyeOff, CheckCircle,
} from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { openUrl } from '@tauri-apps/plugin-opener';
import { open } from '@tauri-apps/plugin-dialog';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { useBridgeStore, AppStateHelpers } from '@/core/stores/bridge';
import type { QuotaInfo, ActiveDevice, TenantVerifyData } from '@/core/stores/bridge';
import type { SubscriptionStatus } from '@/core/domain/types/appState';
import { ApiError } from '@/infrastructure/api/tauri-client';
import { ErrorCode } from '@/generated/error-codes';
import { logger } from '@/utils/logger';
import { useI18n } from '@/hooks/useI18n';
import { MAX_NAME_LEN, MAX_PASSWORD_LEN, MAX_URL_LEN, MAX_SHORT_TEXT_LEN } from '@/shared/constants/validation';

const REGISTER_URL = 'https://redcoral.app/register';

type SetupStep = 'credentials' | 'subscription_blocked' | 'p12_blocked' | 'mode' | 'configure' | 'complete';
type ModeChoice = 'server' | 'client' | null;

const BLOCKED_STATUSES = ['inactive', 'expired', 'canceled', 'unpaid'];

const DEFAULT_HTTP_PORT = 9625;
const DEFAULT_MESSAGE_PORT = 9626;

function formatTimestamp(ts: number): string {
  if (!ts) return '-';
  return new Date(ts).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export const SetupScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const {
    verifyTenant,
    activateServerTenant,
    activateClientTenant,
    startServerMode,
    startClientMode,
    updateServerConfig,
    updateClientConfig,
    fetchAppState,
    exitTenant,
    isLoading,
    error,
  } = useBridgeStore();

  // If tenant is already verified (TenantReady), skip to mode selection.
  // Credentials are only needed at activation time (configure step).
  const appState = useBridgeStore((s) => s.appState);
  const isTenantReady = appState?.type === 'TenantReady';
  const [step, setStep] = useState<SetupStep>(isTenantReady ? 'mode' : 'credentials');
  const [modeChoice, setModeChoice] = useState<ModeChoice>(null);

  // Credentials state (暂存在前端，不缓存到后端)
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [credentialError, setCredentialError] = useState('');
  const [tenantInfo, setTenantInfo] = useState<TenantVerifyData | null>(null);

  // Activation state
  const [activationError, setActivationError] = useState('');
  const [quotaInfo, setQuotaInfo] = useState<QuotaInfo | null>(null);
  const [replacingId, setReplacingId] = useState<string | null>(null);
  // 跟踪已成功激活的模式，避免切换模式时重复激活产生孤立 entity
  const [activatedMode, setActivatedMode] = useState<ModeChoice>(null);

  // Configuration state
  const [configError, setConfigError] = useState('');
  const [httpPort, setHttpPort] = useState(DEFAULT_HTTP_PORT);
  const [messagePort, setMessagePort] = useState(DEFAULT_MESSAGE_PORT);
  const [edgeUrl, setEdgeUrl] = useState('https://edge.example.com');
  const [messageAddr, setMessageAddr] = useState('edge.example.com:9626');

  // Step 1: Verify credentials
  const handleVerify = async (e: React.FormEvent) => {
    e.preventDefault();
    setCredentialError('');

    if (!username.trim() || !password.trim()) {
      setCredentialError(t('auth.activate.error.empty_fields'));
      return;
    }

    try {
      const data = await verifyTenant(username, password);
      setTenantInfo(data);

      if (data.subscription_status && BLOCKED_STATUSES.includes(data.subscription_status)) {
        setStep('subscription_blocked');
        return;
      }

      if (!data.has_p12) {
        setStep('p12_blocked');
        return;
      }

      setStep('mode');
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setCredentialError(msg);
    }
  };

  // Subscription recheck
  const [isRechecking, setIsRechecking] = useState(false);
  const [recheckMessage, setRecheckMessage] = useState<string | null>(null);

  const handleRecheckSubscription = async () => {
    if (!tenantInfo) {
      setStep('credentials');
      return;
    }
    setIsRechecking(true);
    setRecheckMessage(null);
    try {
      const data = await verifyTenant(username, password);
      setTenantInfo(data);
      if (!data.subscription_status || !BLOCKED_STATUSES.includes(data.subscription_status)) {
        setStep(data.has_p12 ? 'mode' : 'p12_blocked');
      } else {
        setRecheckMessage(t('subscriptionBlocked.still_blocked'));
      }
    } catch (err) {
      logger.error('Subscription recheck failed', err);
      setRecheckMessage(t('subscriptionBlocked.still_blocked'));
    } finally {
      setIsRechecking(false);
    }
  };

  const handleExitTenantFromBlocked = async () => {
    setTenantInfo(null);
    setUsername('');
    setPassword('');
    try {
      await exitTenant();
    } catch (err) {
      logger.error('Failed to exit tenant', err);
    }
    setStep('credentials');
  };

  // P12 upload state
  const [p12Password, setP12Password] = useState('');
  const [p12FilePath, setP12FilePath] = useState<string | null>(null);
  const [p12FileName, setP12FileName] = useState<string | null>(null);
  const [isUploadingP12, setIsUploadingP12] = useState(false);
  const [p12UploadError, setP12UploadError] = useState<string | null>(null);
  const [p12UploadSuccess, setP12UploadSuccess] = useState(false);
  const [showP12Password, setShowP12Password] = useState(false);
  const [isCheckingP12, setIsCheckingP12] = useState(false);
  const [p12CheckMessage, setP12CheckMessage] = useState<string | null>(null);

  const handleSelectP12File = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'P12 Certificate', extensions: ['p12', 'pfx'] }],
      });
      if (selected) {
        setP12FilePath(selected);
        const parts = selected.replace(/\\/g, '/').split('/');
        setP12FileName(parts[parts.length - 1]);
        setP12UploadError(null);
      }
    } catch (err) {
      logger.error('File dialog error', err);
    }
  };

  const handleUploadP12 = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!p12FilePath || !p12Password) return;

    setIsUploadingP12(true);
    setP12UploadError(null);

    try {
      await invokeApi('upload_p12', { p12FilePath, p12Password });
      setP12UploadSuccess(true);
      setP12FilePath(null);
      setP12FileName(null);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setP12UploadError(msg);
    } finally {
      setP12Password('');
      setIsUploadingP12(false);
    }
  };

  const handleRecheckP12 = async () => {
    if (!tenantInfo) { setStep('credentials'); return; }
    setIsCheckingP12(true);
    setP12CheckMessage(null);
    try {
      const data = await verifyTenant(username, password);
      setTenantInfo(data);
      if (data.has_p12) {
        setStep('mode');
      } else {
        setP12CheckMessage(t('p12Blocked.still_blocked'));
      }
    } catch (err) {
      logger.error('P12 recheck failed', err);
      setP12CheckMessage(t('p12Blocked.still_blocked'));
    } finally {
      setIsCheckingP12(false);
    }
  };

  // Step 2: Mode selection
  const handleModeSelect = (mode: ModeChoice) => {
    setModeChoice(mode);
    setConfigError('');
    setActivationError('');
    useBridgeStore.setState({ error: null });
    setStep('configure');
  };

  // Step 3: Configure + activate + start
  const handleConfigure = async (e: React.FormEvent) => {
    e.preventDefault();
    setConfigError('');
    setActivationError('');
    setQuotaInfo(null);
    useBridgeStore.setState({ error: null });

    try {
      // 如果当前模式已激活（激活成功但启动失败的重试场景），跳过激活直接启动
      if (activatedMode !== modeChoice) {
        const activateFn = modeChoice === 'server' ? activateServerTenant : activateClientTenant;
        const result = await activateFn();

        if (result.subscription_status && BLOCKED_STATUSES.includes(result.subscription_status)) {
          setTenantInfo((prev) => prev ? { ...prev, subscription_status: result.subscription_status as SubscriptionStatus } : prev);
          setStep('subscription_blocked');
          return;
        }
        setActivatedMode(modeChoice);
      }

      // Configure + start
      if (modeChoice === 'server') {
        await updateServerConfig(httpPort, messagePort);
        await startServerMode();
      } else if (modeChoice === 'client') {
        await updateClientConfig(edgeUrl, messageAddr);
        await startClientMode(edgeUrl, messageAddr);
      }

      setStep('complete');
    } catch (err: unknown) {
      const limitCode = modeChoice === 'server'
        ? ErrorCode.DeviceLimitReached
        : ErrorCode.ClientLimitReached;

      if (err instanceof ApiError && err.code === limitCode) {
        const qi = err.details?.quota_info as QuotaInfo | undefined;
        if (qi) {
          setQuotaInfo(qi);
          return;
        }
      }
      const msg = err instanceof Error ? err.message : String(err);
      setConfigError(msg);
    }
  };

  const handleReplace = async (device: ActiveDevice) => {
    setReplacingId(device.entity_id);
    setActivationError('');
    setConfigError('');

    try {
      const activateFn = modeChoice === 'server' ? activateServerTenant : activateClientTenant;
      const result = await activateFn(device.entity_id);
      setQuotaInfo(null);
      setActivatedMode(modeChoice);

      if (result.subscription_status && BLOCKED_STATUSES.includes(result.subscription_status)) {
        setTenantInfo((prev) => prev ? { ...prev, subscription_status: result.subscription_status as SubscriptionStatus } : prev);
        setStep('subscription_blocked');
        return;
      }

      // Configure + start after replacement
      if (modeChoice === 'server') {
        await updateServerConfig(httpPort, messagePort);
        await startServerMode();
      } else if (modeChoice === 'client') {
        await updateClientConfig(edgeUrl, messageAddr);
        await startClientMode(edgeUrl, messageAddr);
      }

      setStep('complete');
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setConfigError(msg);
    } finally {
      setReplacingId(null);
    }
  };

  const handleComplete = async () => {
    // Clear sensitive credentials from memory
    setUsername('');
    setPassword('');
    try {
      await fetchAppState();
    } catch (err) {
      logger.error('fetchAppState failed in handleComplete', err);
    }
    const state = useBridgeStore.getState().appState;
    const route = AppStateHelpers.getRouteForState(state);
    // 防止循环: 如果 appState 仍指向 /setup（fetchAppState 失败或状态过时），
    // 强制导航到 /login，因为 complete 步骤只有在 server/client 启动成功后才会显示
    if (route === '/setup') {
      navigate('/login', { replace: true });
    } else {
      navigate(route, { replace: true });
    }
  };

  const handleCloseApp = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.destroy();
    } catch (err) {
      logger.error('Failed to close app', err);
    }
  };

  const isServer = modeChoice === 'server';

  const stepLabels = [
    t('setup.step.credentials'),
    t('setup.step.mode'),
    t('setup.step.configure'),
    t('setup.step.complete'),
  ];
  const stepKeys: SetupStep[] = ['credentials', 'mode', 'configure', 'complete'];

  // ==================== Quota Replacement UI ====================
  const limitKey = isServer ? 'auth.activate.device_limit' : 'auth.activate.client_limit';

  if (quotaInfo) {
    return (
      <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
        <button
          onClick={handleCloseApp}
          className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
          title={t('common.dialog.close_app')}
        >
          <Power size={24} />
        </button>

        <div className="w-full max-w-lg mx-auto space-y-6">
          <div className="text-center">
            <div className="inline-flex items-center justify-center w-16 h-16 bg-amber-500/10 rounded-2xl mb-4">
              <Monitor className="text-amber-500" size={32} />
            </div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">
              {t(`${limitKey}.title`)}
            </h1>
            <p className="text-gray-500">
              {t(`${limitKey}.description`, {
                max: String(quotaInfo.max_slots),
                count: String(quotaInfo.active_count),
              })}
            </p>
          </div>

          {(activationError || configError) && (
            <div className="flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
              <AlertCircle size={20} className="shrink-0" />
              <span className="text-sm font-medium">{activationError || configError}</span>
            </div>
          )}

          <div className="space-y-3">
            {quotaInfo.active_devices.map((device) => (
              <div
                key={device.entity_id}
                className="bg-white rounded-xl border border-gray-200 p-4 flex items-center justify-between gap-4"
              >
                <div className="min-w-0 flex-1">
                  <div className="font-medium text-gray-900 truncate text-sm">
                    {device.entity_id}
                  </div>
                  <div className="text-xs text-gray-500 mt-1 space-y-0.5">
                    <div>
                      {t(`${limitKey}.device_id`)}: {device.device_id.slice(0, 12)}...
                    </div>
                    <div>
                      {t(`${limitKey}.activated_at`)}: {formatTimestamp(device.activated_at)}
                    </div>
                    {device.last_refreshed_at && (
                      <div>
                        {t(`${limitKey}.last_refreshed`)}: {formatTimestamp(device.last_refreshed_at)}
                      </div>
                    )}
                  </div>
                </div>
                <button
                  onClick={() => handleReplace(device)}
                  disabled={replacingId !== null}
                  className="shrink-0 px-4 py-2 text-sm font-medium text-white bg-amber-500 rounded-lg hover:bg-amber-600 active:scale-[0.98] transition-all disabled:opacity-50 flex items-center gap-2"
                >
                  {replacingId === device.entity_id ? (
                    <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  ) : (
                    <RefreshCw size={14} />
                  )}
                  <span>
                    {replacingId === device.entity_id
                      ? t(`${limitKey}.replacing`)
                      : t(`${limitKey}.button_replace`)}
                  </span>
                </button>
              </div>
            ))}
          </div>

          <button
            onClick={() => setQuotaInfo(null)}
            disabled={replacingId !== null}
            className="w-full py-3 text-gray-600 font-medium rounded-xl border border-gray-200 hover:bg-gray-50 transition-colors disabled:opacity-50"
          >
            {t(`${limitKey}.button_cancel`)}
          </button>
        </div>
      </div>
    );
  }

  // ==================== Step: Subscription Blocked ====================
  const renderSubscriptionBlockedStep = () => {
    const status = tenantInfo?.subscription_status ?? 'inactive';
    const statusKey = status.charAt(0).toUpperCase() + status.slice(1);
    const statusMessage = t(`subscriptionBlocked.message.${statusKey}`);
    // inactive = 无订阅记录，plan 是后端默认值，不展示
    const planLabel = status !== 'inactive' && tenantInfo?.plan
      ? t(`subscriptionBlocked.planType.${tenantInfo.plan}`)
      : '';

    return (
      <div className="max-w-md mx-auto space-y-6">
        <div className="text-center">
          <div className="inline-flex items-center justify-center w-20 h-20 bg-orange-100 rounded-full mb-4">
            <ShieldAlert className="text-orange-500" size={48} />
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2">
            {t('subscriptionBlocked.title')}
          </h1>
          <p className="text-lg text-gray-600">
            {statusMessage}
          </p>
        </div>

        {/* Plan info */}
        {planLabel && (
          <div className="bg-gray-50 rounded-xl p-4 text-sm text-gray-600 text-center">
            <p>{t('subscriptionBlocked.plan')}: <strong>{planLabel}</strong></p>
          </div>
        )}

        {/* Recheck message */}
        {recheckMessage && (
          <p className="text-sm text-center text-orange-600">{recheckMessage}</p>
        )}

        {/* Actions */}
        <div className="space-y-3">
          <button
            onClick={handleRecheckSubscription}
            disabled={isRechecking}
            className="w-full py-3 bg-blue-500 text-white font-bold rounded-xl hover:bg-blue-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <RefreshCw size={20} className={isRechecking ? 'animate-spin' : ''} />
            {isRechecking
              ? t('subscriptionBlocked.rechecking')
              : t('subscriptionBlocked.button_recheck')}
          </button>

          <button
            onClick={handleExitTenantFromBlocked}
            className="w-full py-3 text-gray-400 hover:text-red-500 font-medium rounded-xl hover:bg-red-50 transition-all flex items-center justify-center gap-2"
          >
            <LogOut size={18} />
            {t('subscriptionBlocked.button_exit_tenant')}
          </button>
        </div>
      </div>
    );
  };

  // ==================== Step: P12 Blocked ====================
  const renderP12BlockedStep = () => {
    const canSubmitP12 = p12Password.trim() !== '' && p12FilePath !== null && !isUploadingP12;

    return (
      <div className="max-w-md mx-auto space-y-6">
        <div className="text-center">
          <div className="inline-flex items-center justify-center w-20 h-20 bg-orange-100 rounded-full mb-4">
            <ShieldAlert className="text-orange-500" size={48} />
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2">
            {t('p12Blocked.title')}
          </h1>
          <p className="text-lg text-gray-600">
            {t('p12Blocked.message.missing')}
          </p>
        </div>

        {/* Upload success */}
        {p12UploadSuccess && (
          <div className="bg-green-50 border border-green-200 rounded-xl p-4">
            <div className="flex items-center gap-2 text-green-700 font-medium">
              <CheckCircle size={18} />
              {t('p12Blocked.upload.success')}
            </div>
          </div>
        )}

        {/* Upload Form */}
        {!p12UploadSuccess && (
          <form onSubmit={handleUploadP12} className="space-y-4">
            <h3 className="text-sm font-semibold text-gray-700 flex items-center gap-2">
              <FileKey size={16} className="text-primary-500" />
              {t('p12Blocked.upload.section_title')}
            </h3>

            {/* File picker */}
            <div className="space-y-1">
              <label className="text-xs font-medium text-gray-600">
                {t('p12Blocked.upload.file_label')}
              </label>
              <button
                type="button"
                onClick={handleSelectP12File}
                disabled={isUploadingP12}
                className={`w-full px-3 py-2.5 text-sm border-2 border-dashed rounded-xl text-left transition-colors flex items-center gap-2 ${
                  p12FileName
                    ? 'border-primary-300 bg-primary-50 text-primary-700'
                    : 'border-gray-200 text-gray-400 hover:border-gray-300 hover:bg-gray-50'
                } disabled:opacity-50`}
              >
                <Upload size={16} />
                {p12FileName || t('p12Blocked.upload.file_placeholder')}
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
                  disabled={isUploadingP12}
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

            <p className="text-xs text-gray-400 text-center">
              {t('p12Blocked.upload.security_notice')}
            </p>

            {p12UploadError && (
              <div className="flex items-center gap-2 text-red-600 bg-red-50 p-3 rounded-xl border border-red-100">
                <AlertCircle size={16} className="shrink-0" />
                <span className="text-sm">{p12UploadError}</span>
              </div>
            )}

            <button
              type="submit"
              disabled={!canSubmitP12}
              className="w-full py-3 bg-primary-500 text-white font-bold rounded-xl hover:bg-primary-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isUploadingP12 ? (
                <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
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
          <button
            onClick={handleRecheckP12}
            disabled={isCheckingP12}
            className="w-full py-3 bg-blue-500 text-white font-bold rounded-xl hover:bg-blue-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <RefreshCw size={20} className={isCheckingP12 ? 'animate-spin' : ''} />
            {isCheckingP12
              ? t('p12Blocked.rechecking')
              : p12UploadSuccess ? t('common.confirm') : t('p12Blocked.button_recheck')}
          </button>

          {p12CheckMessage && (
            <p className="text-sm text-center text-orange-600">{p12CheckMessage}</p>
          )}

          <button
            onClick={handleExitTenantFromBlocked}
            className="w-full py-3 text-gray-400 hover:text-red-500 font-medium rounded-xl hover:bg-red-50 transition-all flex items-center justify-center gap-2"
          >
            <LogOut size={18} />
            {t('p12Blocked.button_exit_tenant')}
          </button>
        </div>
      </div>
    );
  };

  // ==================== Step: Credentials ====================
  const renderCredentialsStep = () => (
    <div className="max-w-md mx-auto space-y-8">
      <div className="text-center">
        <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl mb-4 bg-primary-500/10">
          <UserCheck className="text-primary-500" size={32} />
        </div>
        <h1 className="text-2xl font-bold text-gray-900 mb-2">{t('setup.credentials_title')}</h1>
        <p className="text-gray-500">{t('setup.credentials_desc')}</p>
      </div>

      <form onSubmit={handleVerify} className="space-y-6">
        <div className="space-y-1">
          <label className="text-sm font-medium text-gray-700">{t('auth.activate.username_label')}</label>
          <input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder={t('auth.activate.username_placeholder')}
            maxLength={MAX_NAME_LEN}
            className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
            disabled={isLoading}
          />
        </div>

        <div className="space-y-1">
          <label className="text-sm font-medium text-gray-700">{t('auth.activate.password_label')}</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder={t('auth.activate.password_placeholder')}
            maxLength={MAX_PASSWORD_LEN}
            className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
            disabled={isLoading}
          />
        </div>

        {(credentialError || error) && (
          <div className="flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
            <AlertCircle size={20} className="shrink-0" />
            <span className="text-sm font-medium">{credentialError || error}</span>
          </div>
        )}

        <button
          type="submit"
          disabled={isLoading}
          className="w-full py-3 text-white font-bold rounded-xl active:scale-[0.98] transition-all shadow-lg flex items-center justify-center gap-2 disabled:opacity-70 bg-primary-500 hover:bg-primary-600 shadow-primary-500/25"
        >
          {isLoading ? (
            <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
          ) : (
            <>
              <span>{t('setup.button_verify')}</span>
              <ChevronRight size={20} />
            </>
          )}
        </button>

        {/* Register Link */}
        <div className="text-center">
          <span className="text-sm text-gray-500">{t('auth.activate.no_account')}</span>{' '}
          <button
            type="button"
            onClick={() => openUrl(REGISTER_URL)}
            className="text-sm font-medium text-primary-500 hover:text-primary-600 inline-flex items-center gap-1"
          >
            {t('auth.activate.register')}
            <ExternalLink size={14} />
          </button>
        </div>
      </form>
    </div>
  );

  // ==================== Step: Mode Selection ====================
  const renderModeStep = () => (
    <div className="space-y-8">
      <div className="text-center">
        <h1 className="text-3xl font-bold text-gray-900 mb-2">{t('setup.title')}</h1>
        <p className="text-gray-500">{t('setup.description')}</p>
        {tenantInfo && (
          <div className="mt-4 inline-flex items-center gap-2 px-4 py-2 bg-green-50 text-green-700 rounded-lg text-sm">
            <Shield size={16} />
            <span>{t('setup.tenant_verified', { tenant_id: tenantInfo.tenant_id })}</span>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <button
          onClick={() => handleModeSelect('server')}
          className="group p-8 rounded-2xl border-2 border-gray-200 hover:border-primary-500 bg-white hover:bg-primary-50 transition-all text-left"
        >
          <div className="flex items-center gap-4 mb-4">
            <div className="w-14 h-14 rounded-xl bg-primary-500/10 flex items-center justify-center group-hover:bg-primary-500/20">
              <Server className="text-primary-500" size={28} />
            </div>
            <div>
              <h3 className="text-xl font-semibold text-gray-900">{t('setup.server_mode.title')}</h3>
              <p className="text-sm text-gray-500">{t('setup.server_mode.subtitle')}</p>
            </div>
          </div>
          <p className="text-gray-600 text-sm mb-4">{t('setup.server_mode.description')}</p>
          {tenantInfo && (
            <p className="text-xs text-gray-400 mb-2">
              {t('setup.slots_remaining', { count: String(tenantInfo.server_slots_remaining) })}
            </p>
          )}
          <div className="flex items-center text-primary-500 text-sm font-medium">
            <span>{t('setup.server_mode.select')}</span>
            <ChevronRight size={16} className="ml-1 group-hover:translate-x-1 transition-transform" />
          </div>
        </button>

        <button
          onClick={() => handleModeSelect('client')}
          className="group p-8 rounded-2xl border-2 border-gray-200 hover:border-blue-500 bg-white hover:bg-blue-50 transition-all text-left"
        >
          <div className="flex items-center gap-4 mb-4">
            <div className="w-14 h-14 rounded-xl bg-blue-500/10 flex items-center justify-center group-hover:bg-blue-500/20">
              <Wifi className="text-blue-500" size={28} />
            </div>
            <div>
              <h3 className="text-xl font-semibold text-gray-900">{t('setup.client_mode.title')}</h3>
              <p className="text-sm text-gray-500">{t('setup.client_mode.subtitle')}</p>
            </div>
          </div>
          <p className="text-gray-600 text-sm mb-4">{t('setup.client_mode.description')}</p>
          {tenantInfo && (
            <p className="text-xs text-gray-400 mb-2">
              {t('setup.slots_remaining', { count: String(tenantInfo.client_slots_remaining) })}
            </p>
          )}
          <div className="flex items-center text-blue-500 text-sm font-medium">
            <span>{t('setup.client_mode.select')}</span>
            <ChevronRight size={16} className="ml-1 group-hover:translate-x-1 transition-transform" />
          </div>
        </button>
      </div>

      {/* Back to credentials */}
      <div className="text-center">
        <button
          type="button"
          onClick={async () => {
            setTenantInfo(null);
            setUsername('');
            setPassword('');
            setActivatedMode(null);
            useBridgeStore.setState({ error: null });
            try {
              await exitTenant();
            } catch (err) {
              logger.error('Failed to exit tenant', err);
            }
            setStep('credentials');
          }}
          className="text-sm text-gray-500 hover:text-gray-700"
        >
          {t('setup.button_back')}
        </button>
      </div>
    </div>
  );

  // ==================== Step: Configure ====================
  const renderConfigureStep = () => (
    <div className="max-w-md mx-auto space-y-8">
      <div className="text-center">
        <div className={`inline-flex items-center justify-center w-16 h-16 rounded-2xl mb-4 ${isServer ? 'bg-primary-500/10' : 'bg-blue-500/10'}`}>
          <Settings className={isServer ? 'text-primary-500' : 'text-blue-500'} size={32} />
        </div>
        <h1 className="text-2xl font-bold text-gray-900 mb-2">
          {isServer ? t('setup.configure_server_title') : t('setup.configure_client_title')}
        </h1>
        <p className="text-gray-500">
          {isServer ? t('setup.configure_server_desc') : t('setup.configure_client_desc')}
        </p>
      </div>

      <form onSubmit={handleConfigure} className="space-y-6">
        {isServer ? (
          <>
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">{t('setup.server.http_port_label')}</label>
              <input
                type="number"
                value={httpPort}
                onChange={(e) => setHttpPort(parseInt(e.target.value) || DEFAULT_HTTP_PORT)}
                placeholder="9625"
                min={1024}
                max={65535}
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">{t('setup.server.http_port_help')}</p>
            </div>
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">{t('setup.server.message_port_label')}</label>
              <input
                type="number"
                value={messagePort}
                onChange={(e) => setMessagePort(parseInt(e.target.value) || DEFAULT_MESSAGE_PORT)}
                placeholder="9626"
                min={1024}
                max={65535}
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">{t('setup.server.message_port_help')}</p>
            </div>
          </>
        ) : (
          <>
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">{t('setup.client.edge_url_label')}</label>
              <input
                type="url"
                value={edgeUrl}
                onChange={(e) => setEdgeUrl(e.target.value)}
                placeholder="https://edge.example.com"
                maxLength={MAX_URL_LEN}
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">{t('setup.client.edge_url_help')}</p>
            </div>
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">{t('setup.client.message_addr_label')}</label>
              <input
                type="text"
                value={messageAddr}
                onChange={(e) => setMessageAddr(e.target.value)}
                placeholder="edge.example.com:9626"
                maxLength={MAX_SHORT_TEXT_LEN}
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">{t('setup.client.message_addr_help')}</p>
            </div>
          </>
        )}

        {(configError || error) && (
          <div className="flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
            <AlertCircle size={20} className="shrink-0" />
            <span className="text-sm font-medium">{configError || error}</span>
          </div>
        )}

        <div className="flex gap-4">
          <button
            type="button"
            onClick={() => { setStep('mode'); setConfigError(''); useBridgeStore.setState({ error: null }); }}
            disabled={isLoading}
            className="px-6 py-3 text-gray-600 bg-gray-100 rounded-xl hover:bg-gray-200 transition-colors"
          >
            {t('setup.button_back')}
          </button>
          <button
            type="submit"
            disabled={isLoading}
            className={`flex-1 py-3 text-white font-bold rounded-xl active:scale-[0.98] transition-all shadow-lg flex items-center justify-center gap-2 disabled:opacity-70 ${
              isServer
                ? 'bg-primary-500 hover:bg-primary-600 shadow-primary-500/25'
                : 'bg-blue-500 hover:bg-blue-600 shadow-blue-500/25'
            }`}
          >
            {isLoading ? (
              <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            ) : (
              <>
                <span>{isServer ? t('setup.button_start_server') : t('setup.button_start_client')}</span>
                <ChevronRight size={20} />
              </>
            )}
          </button>
        </div>
      </form>
    </div>
  );

  // ==================== Step: Complete ====================
  const renderCompleteStep = () => (
    <div className="max-w-md mx-auto text-center space-y-8">
      <div className="inline-flex items-center justify-center w-20 h-20 bg-green-100 rounded-full">
        <svg className="w-10 h-10 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
        </svg>
      </div>

      <div>
        <h1 className="text-2xl font-bold text-gray-900 mb-2">{t('setup.complete_title')}</h1>
        <p className="text-gray-500">
          {t('setup.complete_description', {
            mode: isServer ? t('setup.complete_mode_server') : t('setup.complete_mode_client'),
          })}
        </p>
      </div>

      <button
        onClick={handleComplete}
        className="w-full py-4 bg-primary-500 text-white font-bold rounded-xl hover:bg-primary-600 active:scale-[0.98] transition-all shadow-lg shadow-primary-500/25 flex items-center justify-center gap-2"
      >
        <span>{t('setup.button_continue')}</span>
        <ChevronRight size={20} />
      </button>
    </div>
  );

  // ==================== Main Layout ====================
  return (
    <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
      <button
        onClick={handleCloseApp}
        className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
        title={t('common.dialog.close_app')}
      >
        <Power size={24} />
      </button>

      <div className="w-full max-w-4xl">
        {/* Progress indicator */}
        {(() => {
          // subscription_blocked / p12_blocked 在视觉上等同于 credentials→mode 之间
          const isBlocked = step === 'subscription_blocked' || step === 'p12_blocked';
          const visualStep = isBlocked ? 'mode' : step;
          const visualIndex = stepKeys.indexOf(visualStep);

          return (
            <div className="flex items-center justify-center gap-2 mb-12">
              {stepKeys.map((s, i) => (
                <React.Fragment key={s}>
                  <div className="flex flex-col items-center">
                    <div
                      className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors ${
                        isBlocked && s === 'mode'
                          ? 'bg-orange-500 text-white'
                          : visualStep === s
                            ? 'bg-primary-500 text-white'
                            : visualIndex > i
                              ? 'bg-green-500 text-white'
                              : 'bg-gray-200 text-gray-500'
                      }`}
                    >
                      {isBlocked && s === 'mode' ? (
                        <ShieldAlert size={16} />
                      ) : visualIndex > i ? (
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                        </svg>
                      ) : (
                        i + 1
                      )}
                    </div>
                    <span className="text-xs text-gray-500 mt-1">{stepLabels[i]}</span>
                  </div>
                  {i < stepKeys.length - 1 && (
                    <div
                      className={`w-12 h-1 rounded mb-5 ${
                        visualIndex > i ? 'bg-green-500' : 'bg-gray-200'
                      }`}
                    />
                  )}
                </React.Fragment>
              ))}
            </div>
          );
        })()}

        {/* Step content */}
        {step === 'credentials' && renderCredentialsStep()}
        {step === 'subscription_blocked' && renderSubscriptionBlockedStep()}
        {step === 'p12_blocked' && renderP12BlockedStep()}
        {step === 'mode' && renderModeStep()}
        {step === 'configure' && renderConfigureStep()}
        {step === 'complete' && renderCompleteStep()}
      </div>
    </div>
  );
};
