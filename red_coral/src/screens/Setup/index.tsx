import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Server, Wifi, AlertCircle, ChevronRight, Settings, Power, Shield,
  Monitor, RefreshCw, ExternalLink, UserCheck,
} from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useBridgeStore, AppStateHelpers } from '@/core/stores/bridge';
import type { QuotaInfo, ActiveDevice, TenantVerifyData } from '@/core/stores/bridge';
import { ApiError } from '@/infrastructure/api/tauri-client';
import { ErrorCode } from '@/generated/error-codes';
import { logger } from '@/utils/logger';
import { useI18n } from '@/hooks/useI18n';
import { MAX_NAME_LEN, MAX_PASSWORD_LEN, MAX_URL_LEN, MAX_SHORT_TEXT_LEN } from '@/shared/constants/validation';

const REGISTER_URL = 'https://auth.redcoral.app/register';

type SetupStep = 'credentials' | 'mode' | 'configure' | 'complete';
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

  // Configuration state
  const [configError, setConfigError] = useState('');
  const [httpPort, setHttpPort] = useState(DEFAULT_HTTP_PORT);
  const [messagePort, setMessagePort] = useState(DEFAULT_MESSAGE_PORT);
  const [edgeUrl, setEdgeUrl] = useState('https://edge.example.com');
  const [messageAddr, setMessageAddr] = useState('edge.example.com:9626');

  const hasCredentials = username.trim() !== '' && password.trim() !== '';

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
        navigate('/status/subscription-blocked', { replace: true });
        return;
      }

      setStep('mode');
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setCredentialError(msg);
    }
  };

  // Step 2: Mode selection
  const handleModeSelect = (mode: ModeChoice) => {
    setModeChoice(mode);
    setStep('configure');
  };

  // Step 3: Configure + activate + start
  const handleConfigure = async (e: React.FormEvent) => {
    e.preventDefault();
    setConfigError('');
    setActivationError('');
    setQuotaInfo(null);

    // Ensure we have a token (verify if needed, e.g. TenantReady without credentials)
    let token = tenantInfo?.token;
    if (!token) {
      if (!username.trim() || !password.trim()) {
        setConfigError(t('auth.activate.error.empty_fields'));
        return;
      }
      try {
        const data = await verifyTenant(username, password);
        setTenantInfo(data);
        token = data.token;
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setConfigError(msg);
        return;
      }
    }

    try {
      // Activate (download cert)
      const activateFn = modeChoice === 'server' ? activateServerTenant : activateClientTenant;
      const result = await activateFn(token);

      if (result.subscription_status && BLOCKED_STATUSES.includes(result.subscription_status)) {
        navigate('/status/subscription-blocked', { replace: true });
        return;
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
      const token = tenantInfo?.token;
      if (!token) {
        setConfigError('Session expired, please re-enter credentials');
        return;
      }
      const activateFn = modeChoice === 'server' ? activateServerTenant : activateClientTenant;
      const result = await activateFn(token, device.entity_id);
      setQuotaInfo(null);

      if (result.subscription_status && BLOCKED_STATUSES.includes(result.subscription_status)) {
        navigate('/status/subscription-blocked', { replace: true });
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
    await fetchAppState();
    const state = useBridgeStore.getState().appState;
    const route = AppStateHelpers.getRouteForState(state);
    navigate(route, { replace: true });
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
            await exitTenant();
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
        {/* Credentials for activation (needed if no token yet, e.g. TenantReady skip) */}
        {!tenantInfo?.token && (
          <>
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
          </>
        )}

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
            onClick={() => { setStep('mode'); setConfigError(''); }}
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
        <div className="flex items-center justify-center gap-2 mb-12">
          {stepKeys.map((s, i) => (
            <React.Fragment key={s}>
              <div className="flex flex-col items-center">
                <div
                  className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors ${
                    step === s
                      ? 'bg-primary-500 text-white'
                      : stepKeys.indexOf(step) > i
                        ? 'bg-green-500 text-white'
                        : 'bg-gray-200 text-gray-500'
                  }`}
                >
                  {stepKeys.indexOf(step) > i ? (
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
                    stepKeys.indexOf(step) > i ? 'bg-green-500' : 'bg-gray-200'
                  }`}
                />
              )}
            </React.Fragment>
          ))}
        </div>

        {/* Step content */}
        {step === 'credentials' && renderCredentialsStep()}
        {step === 'mode' && renderModeStep()}
        {step === 'configure' && renderConfigureStep()}
        {step === 'complete' && renderCompleteStep()}
      </div>
    </div>
  );
};
