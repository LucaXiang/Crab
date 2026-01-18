import React, { useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { useBridgeStore } from '@/core/stores/bridge';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

// Components
import { ToastContainer } from '@/presentation/components/Toast';
import { ProtectedRoute } from '@/presentation/components/ProtectedRoute';
import { PermissionEscalationProvider } from '@/presentation/components/auth/PermissionEscalationProvider';
import { NotificationProvider } from '@/presentation/components/notifications';

// Screens
import { LoginScreen } from '@/screens/Login';
import { POSScreen } from '@/screens/POS';
import { SetupScreen } from '@/screens/Setup';
import { TenantSelectScreen } from '@/screens/TenantSelect';

// Initial route component that handles first-run detection and mode auto-start
const InitialRoute: React.FC = () => {
  const {
    isFirstRun,
    tenants,
    fetchTenants,
    checkFirstRun,
    getCurrentTenant,
    fetchModeInfo,
    startServerMode,
  } = useBridgeStore();
  const [isChecking, setIsChecking] = useState(true);
  const [hasCurrentTenant, setHasCurrentTenant] = useState(false);

  useEffect(() => {
    const init = async () => {
      const isFirst = await checkFirstRun();
      await fetchTenants();
      const current = await getCurrentTenant();
      setHasCurrentTenant(!!current);

      // If not first run and has tenants, auto-start Server mode
      // (Since we only support Server mode for now)
      if (!isFirst && current) {
        await fetchModeInfo();
        const info = useBridgeStore.getState().modeInfo;
        if (info?.mode === 'Disconnected') {
          console.log('Auto-starting Server mode...');
          try {
            await startServerMode();
            await fetchModeInfo();
          } catch (err) {
            console.error('Failed to auto-start Server mode:', err);
          }
        }
      }

      setIsChecking(false);
    };
    init();
  }, [checkFirstRun, fetchTenants, getCurrentTenant, fetchModeInfo, startServerMode]);

  if (isChecking) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="w-8 h-8 border-4 border-[#FF5E5E]/30 border-t-[#FF5E5E] rounded-full animate-spin" />
      </div>
    );
  }

  // First run - no tenants activated
  if (isFirstRun || tenants.length === 0) {
    return <Navigate to="/setup" replace />;
  }

  // Multiple tenants but none selected - show tenant selection
  if (tenants.length > 1 && !hasCurrentTenant) {
    return <Navigate to="/tenant-select" replace />;
  }

  // Single tenant or tenant already selected - go to login
  return <Navigate to="/login" replace />;
};

const App: React.FC = () => {
  const performanceMode = useSettingsStore((state) => state.performanceMode);

  // Check for first run and clear storage if needed
  useEffect(() => {
    // Listen for the clear event from backend
    const unlistenPromise = listen('clear-local-storage', () => {
      console.log('First run detected: clearing localStorage');
      localStorage.clear();
      sessionStorage.clear();
      // Reload to apply clean state
      window.location.reload();
    });

    return () => {
      unlistenPromise.then(unlisten => unlisten());
    };
  }, []);

  // Optimize startup: Show window only after React is mounted to avoid black screen
  useEffect(() => {
    const showWindow = async () => {
      // Small delay to ensure initial render paint is complete
      await new Promise(resolve => setTimeout(resolve, 50));
      try {
        await getCurrentWindow().show();
        // Ensure focus
        await getCurrentWindow().setFocus();
      } catch (err) {
        // Ignore errors if window is already visible or API fails
        console.debug('Failed to show window:', err);
      }
    };

    if ('__TAURI__' in window) {
      showWindow();
    }
  }, []);

  useEffect(() => {
    if (performanceMode) {
      document.body.classList.add('performance-mode');
    } else {
      document.body.classList.remove('performance-mode');
    }
  }, [performanceMode]);

  // Shift+R to force reload (dev helper)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.shiftKey && e.key.toLowerCase() === 'r' && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        window.location.reload();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Disable browser default shortcuts and interactions
  useEffect(() => {
    if (!import.meta.env.PROD) {
      return undefined;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
        const key = e.key.toLowerCase();
        const ctrlOrMeta = e.ctrlKey || e.metaKey;
        const shift = e.shiftKey;
        const alt = e.altKey;

        // 1. Refresh: F5, Ctrl+R, Ctrl+Shift+R
        if (key === 'f5' || (ctrlOrMeta && key === 'r')) {
          e.preventDefault();
          return;
        }

        // 2. Developer Tools: F12, Ctrl+Shift+I, Ctrl+Shift+J, Ctrl+Shift+C
        if (
          key === 'f12' ||
          (ctrlOrMeta && shift && (key === 'i' || key === 'j' || key === 'c'))
        ) {
          e.preventDefault();
          return;
        }

        // 3. Find: Ctrl+F, Ctrl+G
        if (ctrlOrMeta && (key === 'f' || key === 'g')) {
          e.preventDefault();
          return;
        }

        // 4. Print: Ctrl+P
        if (ctrlOrMeta && key === 'p') {
          e.preventDefault();
          return;
        }

        // 5. Save: Ctrl+S
        if (ctrlOrMeta && key === 's') {
          e.preventDefault();
          return;
        }

        // 6. Zoom: Ctrl++, Ctrl+-, Ctrl+0
        if (
          ctrlOrMeta &&
          (key === '+' || key === '-' || key === '=' || key === '0')
        ) {
          e.preventDefault();
          return;
        }

        // 7. Navigation: Alt+Left, Alt+Right (Browser Back/Forward)
        if (alt && (key === 'arrowleft' || key === 'arrowright')) {
          e.preventDefault();
          return;
        }

        // 8. New Tab/Window: Ctrl+T, Ctrl+N
        if (ctrlOrMeta && (key === 't' || key === 'n')) {
          e.preventDefault();
          return;
        }
      };

      // Disable Zoom via Scroll (Ctrl + Scroll)
      const handleWheel = (e: WheelEvent) => {
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault();
        }
      };

      // Disable Context Menu (Right Click)
      const handleContextMenu = (e: MouseEvent) => {
        // Allow context menu only on inputs if needed, or completely disable
        // For a POS, usually completely disable is safer unless specific areas need it
        if (
          import.meta.env.PROD ||
          !e.target ||
          (e.target as HTMLElement).tagName !== 'INPUT'
        ) {
          e.preventDefault();
        }
      };

      window.addEventListener('keydown', handleKeyDown);
      window.addEventListener('wheel', handleWheel, { passive: false });
      window.addEventListener('contextmenu', handleContextMenu);

      return () => {
        window.removeEventListener('keydown', handleKeyDown);
        window.removeEventListener('wheel', handleWheel);
        window.removeEventListener('contextmenu', handleContextMenu);
      };
  }, []);

  return (
    <BrowserRouter>
      <NotificationProvider>
        {/* Global Toast Container */}
        <ToastContainer />
        <PermissionEscalationProvider />

        <Routes>
        {/* Setup Routes */}
        <Route path="/setup" element={<SetupScreen />} />
        <Route path="/tenant-select" element={<TenantSelectScreen />} />

        {/* Public Routes */}
        <Route path="/login" element={<LoginScreen />} />

        {/* Protected Routes */}
        <Route
          path="/pos"
          element={
            <ProtectedRoute>
              <POSScreen />
            </ProtectedRoute>
          }
        />

        {/* Default Route - handles first-run detection */}
        <Route path="/" element={<InitialRoute />} />
        <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </NotificationProvider>
    </BrowserRouter>
  );
};

export default App;
