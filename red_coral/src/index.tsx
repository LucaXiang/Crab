import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';
import { attachConsole } from '@tauri-apps/plugin-log';
import { getCurrentWindow } from '@tauri-apps/api/window';
import GlobalErrorBoundary from './presentation/components/GlobalErrorBoundary';
import { reportError } from '@/utils/reportError';

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error("Could not find root element to mount to");
}

const root = ReactDOM.createRoot(rootElement);
void attachConsole();

if ('__TAURI__' in window) {
  const win = getCurrentWindow();
  void win.setFullscreen(true);
  window.addEventListener('focus', () => {
    void win.setFullscreen(true);
  });
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') {
      void win.setFullscreen(true);
    }
  });
}

window.addEventListener('error', (ev) => {
  void reportError(
    ev.message || 'window.onerror',
    ev.error ?? new Error(ev.message || 'window.onerror'),
    'window.error',
    { source: 'webview_uncaught', userActionOverride: null }
  );
});

window.addEventListener('unhandledrejection', (ev: PromiseRejectionEvent) => {
  const reason = ev.reason as any;
  const message = typeof reason === 'string' ? reason : reason?.message || 'unhandledrejection';
  void reportError(
    message,
    reason,
    'window.unhandledrejection',
    { source: 'webview_unhandled_rejection', userActionOverride: null }
  );
});

root.render(
  <React.StrictMode>
    <GlobalErrorBoundary>
      <App />
    </GlobalErrorBoundary>
  </React.StrictMode>
);
