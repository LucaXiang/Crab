import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Server, Wifi, AlertCircle, ChevronRight, Settings, Power } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useBridgeStore, AppStateHelpers } from '@/core/stores/bridge';
import { useI18n } from '@/hooks/useI18n';
import { friendlyError } from '@/utils/error/friendlyError';

type SetupStep = 'mode' | 'configure' | 'complete';
type ModeChoice = 'server' | 'client' | null;

// Default ports for Server mode
const DEFAULT_HTTP_PORT = 9625;
const DEFAULT_MESSAGE_PORT = 9626;

export const SetupScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const {
    startServerMode,
    startClientMode,
    updateServerConfig,
    updateClientConfig,
    fetchAppState,
    isLoading,
    error,
  } = useBridgeStore();

  const [step, setStep] = useState<SetupStep>('mode');
  const [modeChoice, setModeChoice] = useState<ModeChoice>(null);

  // 配置错误信息
  const [configError, setConfigError] = useState('');

  // Server mode config
  const [httpPort, setHttpPort] = useState(DEFAULT_HTTP_PORT);
  const [messagePort, setMessagePort] = useState(DEFAULT_MESSAGE_PORT);

  // Client mode config
  const authUrl = 'http://127.0.0.1:3001';
  const [edgeUrl, setEdgeUrl] = useState('https://edge.example.com');
  const [messageAddr, setMessageAddr] = useState('edge.example.com:9626');

  const handleModeSelect = (mode: ModeChoice) => {
    setModeChoice(mode);
    setStep('configure');
  };

  const handleConfigure = async (e: React.FormEvent) => {
    e.preventDefault();
    setConfigError('');

    try {
      if (modeChoice === 'server') {
        // Save server config and start server mode
        await updateServerConfig(httpPort, messagePort);
        await startServerMode();
      } else if (modeChoice === 'client') {
        // Save client config and start client mode
        await updateClientConfig(edgeUrl, messageAddr, authUrl);
        await startClientMode(edgeUrl, messageAddr);
      }
      setStep('complete');
    } catch (err: unknown) {
      const raw = err instanceof Error ? err.message : String(err);
      setConfigError(friendlyError(raw));
    }
  };

  const handleComplete = async () => {
    await fetchAppState();
    const appState = useBridgeStore.getState().appState;
    const route = AppStateHelpers.getRouteForState(appState);
    navigate(route, { replace: true });
  };

  const handleCloseApp = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.destroy();
    } catch (err) {
      console.error('Failed to close app:', err);
    }
  };

  const stepLabels = [t('setup.step.mode'), t('setup.step.configure'), t('setup.step.complete')];
  const stepKeys: SetupStep[] = ['mode', 'configure', 'complete'];

  const renderModeStep = () => (
    <div className="space-y-8">
      <div className="text-center">
        <h1 className="text-3xl font-bold text-gray-900 mb-2">{t('setup.title')}</h1>
        <p className="text-gray-500">{t('setup.description')}</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Server Mode */}
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
          <p className="text-gray-600 text-sm mb-4">
            {t('setup.server_mode.description')}
          </p>
          <div className="flex items-center text-primary-500 text-sm font-medium">
            <span>{t('setup.server_mode.select')}</span>
            <ChevronRight size={16} className="ml-1 group-hover:translate-x-1 transition-transform" />
          </div>
        </button>

        {/* Client Mode */}
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
          <p className="text-gray-600 text-sm mb-4">
            {t('setup.client_mode.description')}
          </p>
          <div className="flex items-center text-blue-500 text-sm font-medium">
            <span>{t('setup.client_mode.select')}</span>
            <ChevronRight size={16} className="ml-1 group-hover:translate-x-1 transition-transform" />
          </div>
        </button>
      </div>
    </div>
  );

  const renderConfigureStep = () => (
    <div className="max-w-md mx-auto space-y-8">
      <div className="text-center">
        <div
          className={`inline-flex items-center justify-center w-16 h-16 rounded-2xl mb-4 ${
            modeChoice === 'server' ? 'bg-primary-500/10' : 'bg-blue-500/10'
          }`}
        >
          <Settings className={modeChoice === 'server' ? 'text-primary-500' : 'text-blue-500'} size={32} />
        </div>
        <h1 className="text-2xl font-bold text-gray-900 mb-2">
          {modeChoice === 'server' ? t('setup.configure_server_title') : t('setup.configure_client_title')}
        </h1>
        <p className="text-gray-500">
          {modeChoice === 'server' ? t('setup.configure_server_desc') : t('setup.configure_client_desc')}
        </p>
      </div>

      <form onSubmit={handleConfigure} className="space-y-6">
        {modeChoice === 'server' ? (
          <>
            {/* HTTP Port */}
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

            {/* Message Port */}
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
            {/* Edge Server URL */}
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">{t('setup.client.edge_url_label')}</label>
              <input
                type="url"
                value={edgeUrl}
                onChange={(e) => setEdgeUrl(e.target.value)}
                placeholder="https://edge.example.com"
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">{t('setup.client.edge_url_help')}</p>
            </div>

            {/* Message Server Address */}
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">{t('setup.client.message_addr_label')}</label>
              <input
                type="text"
                value={messageAddr}
                onChange={(e) => setMessageAddr(e.target.value)}
                placeholder="edge.example.com:9626"
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">{t('setup.client.message_addr_help')}</p>
            </div>
          </>
        )}

        {/* 错误信息 */}
        {(configError || error) && (
          <div className="flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
            <AlertCircle size={20} className="shrink-0" />
            <span className="text-sm font-medium">{configError || error}</span>
          </div>
        )}

        {/* Buttons */}
        <div className="flex gap-4">
          <button
            type="button"
            onClick={() => setStep('mode')}
            disabled={isLoading}
            className="px-6 py-3 text-gray-600 bg-gray-100 rounded-xl hover:bg-gray-200 transition-colors"
          >
            {t('setup.button_back')}
          </button>
          <button
            type="submit"
            disabled={isLoading}
            className={`flex-1 py-3 text-white font-bold rounded-xl active:scale-[0.98] transition-all shadow-lg flex items-center justify-center gap-2 disabled:opacity-70 ${
              modeChoice === 'server'
                ? 'bg-primary-500 hover:bg-primary-600 shadow-primary-500/25'
                : 'bg-blue-500 hover:bg-blue-600 shadow-blue-500/25'
            }`}
          >
            {isLoading ? (
              <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            ) : (
              <>
                <span>{modeChoice === 'server' ? t('setup.button_start_server') : t('setup.button_start_client')}</span>
                <ChevronRight size={20} />
              </>
            )}
          </button>
        </div>
      </form>
    </div>
  );

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
            mode: modeChoice === 'server' ? t('setup.complete_mode_server') : t('setup.complete_mode_client'),
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

  return (
    <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
      {/* 关闭按钮 */}
      <button
        onClick={handleCloseApp}
        className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
        title={t('common.dialog.close_app')}
      >
        <Power size={24} />
      </button>

      <div className="w-full max-w-4xl">
        {/* 进度指示器 */}
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

        {/* 步骤内容 */}
        {step === 'mode' && renderModeStep()}
        {step === 'configure' && renderConfigureStep()}
        {step === 'complete' && renderCompleteStep()}
      </div>
    </div>
  );
};

export default SetupScreen;
