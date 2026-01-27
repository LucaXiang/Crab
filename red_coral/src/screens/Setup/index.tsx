import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Server, Wifi, AlertCircle, ChevronRight, Shield, Power, Settings } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useBridgeStore } from '@/core/stores/bridge';

type SetupStep = 'activate' | 'mode' | 'configure' | 'complete';
type ModeChoice = 'server' | 'client' | null;

// Default ports for Server mode
const DEFAULT_HTTP_PORT = 9625;
const DEFAULT_MESSAGE_PORT = 9626;

export const SetupScreen: React.FC = () => {
  const navigate = useNavigate();
  const {
    activateTenant,
    startServerMode,
    startClientMode,
    updateServerConfig,
    updateClientConfig,
    isLoading,
    error,
  } = useBridgeStore();

  const [step, setStep] = useState<SetupStep>('activate');
  const [modeChoice, setModeChoice] = useState<ModeChoice>(null);

  // Activation form state
  const authUrl = 'http://127.0.0.1:3001';
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [activationError, setActivationError] = useState('');

  // Server mode config
  const [httpPort, setHttpPort] = useState(DEFAULT_HTTP_PORT);
  const [messagePort, setMessagePort] = useState(DEFAULT_MESSAGE_PORT);

  // Client mode config
  const [edgeUrl, setEdgeUrl] = useState('https://edge.example.com');
  const [messageAddr, setMessageAddr] = useState('edge.example.com:9626');

  const handleActivate = async (e: React.FormEvent) => {
    e.preventDefault();
    setActivationError('');

    if (!username.trim() || !password.trim()) {
      setActivationError('Please enter username and password');
      return;
    }

    try {
      await activateTenant(authUrl, username, password);
      // After activation, go to mode selection (don't start any mode)
      setStep('mode');
    } catch (err: unknown) {
      setActivationError(err instanceof Error ? err.message : 'Activation failed');
    }
  };

  const handleModeSelect = (mode: ModeChoice) => {
    setModeChoice(mode);
    setStep('configure');
  };

  const handleConfigure = async (e: React.FormEvent) => {
    e.preventDefault();
    setActivationError('');

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
      setActivationError(err instanceof Error ? err.message : 'Failed to start mode');
    }
  };

  const handleComplete = () => {
    navigate('/login', { replace: true });
  };

  const handleCloseApp = async () => {
    const appWindow = getCurrentWindow();
    await appWindow.close();
  };

  const stepLabels = ['Activate', 'Mode', 'Configure', 'Complete'];
  const stepKeys: SetupStep[] = ['activate', 'mode', 'configure', 'complete'];

  const renderActivateStep = () => (
    <div className="max-w-md mx-auto space-y-8">
      <div className="text-center">
        <div className="inline-flex items-center justify-center w-16 h-16 bg-[#FF5E5E]/10 rounded-2xl mb-4">
          <Shield className="text-[#FF5E5E]" size={32} />
        </div>
        <h1 className="text-2xl font-bold text-gray-900 mb-2">Activate Device</h1>
        <p className="text-gray-500">Enter your tenant credentials to activate this device</p>
      </div>

      <form onSubmit={handleActivate} className="space-y-6">
        {/* Username */}
        <div className="space-y-1">
          <label className="text-sm font-medium text-gray-700">Tenant Username</label>
          <input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="Enter your tenant username"
            className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-[#FF5E5E]/20 focus:border-[#FF5E5E]"
            disabled={isLoading}
          />
        </div>

        {/* Password */}
        <div className="space-y-1">
          <label className="text-sm font-medium text-gray-700">Password</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="Enter your password"
            className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-[#FF5E5E]/20 focus:border-[#FF5E5E]"
            disabled={isLoading}
          />
        </div>

        {/* Error Message */}
        {(activationError || error) && (
          <div className="flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
            <AlertCircle size={20} className="shrink-0" />
            <span className="text-sm font-medium">{activationError || error}</span>
          </div>
        )}

        {/* Submit Button */}
        <button
          type="submit"
          disabled={isLoading}
          className="w-full py-3 bg-[#FF5E5E] text-white font-bold rounded-xl hover:bg-[#E54545] active:scale-[0.98] transition-all shadow-lg shadow-[#FF5E5E]/25 flex items-center justify-center gap-2 disabled:opacity-70"
        >
          {isLoading ? (
            <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
          ) : (
            <>
              <span>Activate Device</span>
              <ChevronRight size={20} />
            </>
          )}
        </button>
      </form>
    </div>
  );

  const renderModeStep = () => (
    <div className="space-y-8">
      <div className="text-center">
        <h1 className="text-3xl font-bold text-gray-900 mb-2">Choose Operation Mode</h1>
        <p className="text-gray-500">Select how you want to run the application</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Server Mode */}
        <button
          onClick={() => handleModeSelect('server')}
          className="group p-8 rounded-2xl border-2 border-gray-200 hover:border-[#FF5E5E] bg-white hover:bg-red-50 transition-all text-left"
        >
          <div className="flex items-center gap-4 mb-4">
            <div className="w-14 h-14 rounded-xl bg-[#FF5E5E]/10 flex items-center justify-center group-hover:bg-[#FF5E5E]/20">
              <Server className="text-[#FF5E5E]" size={28} />
            </div>
            <div>
              <h3 className="text-xl font-semibold text-gray-900">Server Mode</h3>
              <p className="text-sm text-gray-500">Run locally with built-in server</p>
            </div>
          </div>
          <p className="text-gray-600 text-sm mb-4">
            Best for standalone POS terminals. Data is stored locally and synced when online.
          </p>
          <div className="flex items-center text-[#FF5E5E] text-sm font-medium">
            <span>Select this mode</span>
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
              <h3 className="text-xl font-semibold text-gray-900">Client Mode</h3>
              <p className="text-sm text-gray-500">Connect to remote server</p>
            </div>
          </div>
          <p className="text-gray-600 text-sm mb-4">
            Connect to an existing Edge Server. Requires network connection to the server.
          </p>
          <div className="flex items-center text-blue-500 text-sm font-medium">
            <span>Select this mode</span>
            <ChevronRight size={16} className="ml-1 group-hover:translate-x-1 transition-transform" />
          </div>
        </button>
      </div>

      {/* Back Button */}
      <div className="flex justify-center">
        <button
          type="button"
          onClick={() => setStep('activate')}
          className="px-6 py-3 text-gray-600 bg-gray-100 rounded-xl hover:bg-gray-200 transition-colors"
        >
          Back to Activation
        </button>
      </div>
    </div>
  );

  const renderConfigureStep = () => (
    <div className="max-w-md mx-auto space-y-8">
      <div className="text-center">
        <div
          className={`inline-flex items-center justify-center w-16 h-16 rounded-2xl mb-4 ${
            modeChoice === 'server' ? 'bg-[#FF5E5E]/10' : 'bg-blue-500/10'
          }`}
        >
          <Settings className={modeChoice === 'server' ? 'text-[#FF5E5E]' : 'text-blue-500'} size={32} />
        </div>
        <h1 className="text-2xl font-bold text-gray-900 mb-2">
          Configure {modeChoice === 'server' ? 'Server' : 'Client'} Mode
        </h1>
        <p className="text-gray-500">
          {modeChoice === 'server'
            ? 'Set the ports for the local server'
            : 'Enter the connection details for the remote server'}
        </p>
      </div>

      <form onSubmit={handleConfigure} className="space-y-6">
        {modeChoice === 'server' ? (
          <>
            {/* HTTP Port */}
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">HTTP Port</label>
              <input
                type="number"
                value={httpPort}
                onChange={(e) => setHttpPort(parseInt(e.target.value) || DEFAULT_HTTP_PORT)}
                placeholder="9625"
                min={1024}
                max={65535}
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-[#FF5E5E]/20 focus:border-[#FF5E5E]"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">Port for HTTP API (default: 9625)</p>
            </div>

            {/* Message Port */}
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">Message Bus Port</label>
              <input
                type="number"
                value={messagePort}
                onChange={(e) => setMessagePort(parseInt(e.target.value) || DEFAULT_MESSAGE_PORT)}
                placeholder="9626"
                min={1024}
                max={65535}
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-[#FF5E5E]/20 focus:border-[#FF5E5E]"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">Port for real-time messaging (default: 9626)</p>
            </div>
          </>
        ) : (
          <>
            {/* Edge Server URL */}
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">Edge Server URL</label>
              <input
                type="url"
                value={edgeUrl}
                onChange={(e) => setEdgeUrl(e.target.value)}
                placeholder="https://edge.example.com"
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">HTTPS URL of the Edge Server</p>
            </div>

            {/* Message Server Address */}
            <div className="space-y-1">
              <label className="text-sm font-medium text-gray-700">Message Server Address</label>
              <input
                type="text"
                value={messageAddr}
                onChange={(e) => setMessageAddr(e.target.value)}
                placeholder="edge.example.com:9626"
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                disabled={isLoading}
              />
              <p className="text-xs text-gray-400">Host:Port for real-time messaging</p>
            </div>
          </>
        )}

        {/* Error Message */}
        {(activationError || error) && (
          <div className="flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
            <AlertCircle size={20} className="shrink-0" />
            <span className="text-sm font-medium">{activationError || error}</span>
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
            Back
          </button>
          <button
            type="submit"
            disabled={isLoading}
            className={`flex-1 py-3 text-white font-bold rounded-xl active:scale-[0.98] transition-all shadow-lg flex items-center justify-center gap-2 disabled:opacity-70 ${
              modeChoice === 'server'
                ? 'bg-[#FF5E5E] hover:bg-[#E54545] shadow-[#FF5E5E]/25'
                : 'bg-blue-500 hover:bg-blue-600 shadow-blue-500/25'
            }`}
          >
            {isLoading ? (
              <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            ) : (
              <>
                <span>Start {modeChoice === 'server' ? 'Server' : 'Client'} Mode</span>
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
        <h1 className="text-2xl font-bold text-gray-900 mb-2">Setup Complete!</h1>
        <p className="text-gray-500">
          Your device is activated and running in{' '}
          <span className="font-medium">{modeChoice === 'server' ? 'Server' : 'Client'}</span> mode. You can
          now log in with your employee credentials.
        </p>
      </div>

      <button
        onClick={handleComplete}
        className="w-full py-4 bg-[#FF5E5E] text-white font-bold rounded-xl hover:bg-[#E54545] active:scale-[0.98] transition-all shadow-lg shadow-[#FF5E5E]/25 flex items-center justify-center gap-2"
      >
        <span>Continue to Login</span>
        <ChevronRight size={20} />
      </button>
    </div>
  );

  return (
    <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
      {/* Close Button */}
      <button
        onClick={handleCloseApp}
        className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
        title="Close Application"
      >
        <Power size={24} />
      </button>

      <div className="w-full max-w-4xl">
        {/* Progress Indicator */}
        <div className="flex items-center justify-center gap-2 mb-12">
          {stepKeys.map((s, i) => (
            <React.Fragment key={s}>
              <div className="flex flex-col items-center">
                <div
                  className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors ${
                    step === s
                      ? 'bg-[#FF5E5E] text-white'
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

        {/* Step Content */}
        {step === 'activate' && renderActivateStep()}
        {step === 'mode' && renderModeStep()}
        {step === 'configure' && renderConfigureStep()}
        {step === 'complete' && renderCompleteStep()}
      </div>
    </div>
  );
};

export default SetupScreen;
