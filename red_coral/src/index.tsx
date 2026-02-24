import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';
import { getCurrentWindow } from '@tauri-apps/api/window';
import GlobalErrorBoundary from './presentation/components/GlobalErrorBoundary';
import { reportError } from '@/utils/reportError';
import { initUIScale } from '@/core/stores/ui';

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error("Could not find root element to mount to");
}

// 初始化 UI 缩放 (从 localStorage 读取并应用到 CSS 变量)
initUIScale();

const root = ReactDOM.createRoot(rootElement);

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

window.addEventListener('error', (ev) => {
  void reportError(
    ev.message || 'window.onerror',
    ev.error ?? new Error(ev.message || 'window.onerror'),
    'window.error',
    { source: 'webview_uncaught', userActionOverride: null }
  );
});

window.addEventListener('unhandledrejection', (ev: PromiseRejectionEvent) => {
  const reason: unknown = ev.reason;
  const message = typeof reason === 'string' ? reason
    : (reason instanceof Error ? reason.message : 'unhandledrejection');
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
