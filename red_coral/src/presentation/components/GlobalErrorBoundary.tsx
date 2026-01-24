import { Component, ErrorInfo, ReactNode } from 'react';
import { RotateCcw, AlertTriangle, ChevronDown, ChevronRight, Copy, CheckCircle2 } from 'lucide-react';
import { reportError } from '@/utils/reportError';
import { useI18n } from '@/hooks/useI18n';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  showDetails: boolean;
  copied: boolean;
  autoReloading: boolean;
}

const ErrorBoundaryUI = ({ 
  state, 
  handleReload, 
  toggleDetails, 
  copyError 
}: { 
  state: State; 
  handleReload: () => void; 
  toggleDetails: () => void; 
  copyError: () => void;
}) => {
  const { t } = useI18n();
  const { error, errorInfo, showDetails, copied, autoReloading } = state;

  return (
    <div className="min-h-screen w-full flex flex-col items-center justify-center bg-gray-50 p-6 font-sans relative overflow-hidden">
      {/* Decorative background elements */}
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden pointer-events-none z-0">
        <div className="absolute top-[-10%] left-[-10%] w-96 h-96 bg-red-100/50 rounded-full blur-3xl" />
        <div className="absolute bottom-[-10%] right-[-10%] w-96 h-96 bg-blue-100/50 rounded-full blur-3xl" />
      </div>

      <div className="bg-white/80 backdrop-blur-sm p-8 md:p-10 rounded-2xl shadow-xl max-w-xl w-full text-center border border-white/50 relative z-10 transition-all duration-300">
        {/* Icon */}
        <div className="mb-8 flex justify-center relative">
          <div className="relative">
            <div className="w-24 h-24 bg-red-50 rounded-full flex items-center justify-center shadow-inner">
              <AlertTriangle className="text-red-500 w-12 h-12" strokeWidth={1.5} />
            </div>
            {autoReloading && (
              <svg className="absolute top-0 left-0 w-24 h-24 -rotate-90 pointer-events-none">
                <circle
                  cx="48"
                  cy="48"
                  r="46"
                  stroke="currentColor"
                  strokeWidth="2"
                  fill="transparent"
                  className="text-red-500 animate-[dash_3s_linear_forwards]"
                  strokeDasharray="289"
                  strokeDashoffset="289"
                  style={{ strokeDashoffset: 0 }}
                />
              </svg>
            )}
          </div>
        </div>

        {/* Content */}
        <h1 className="text-3xl font-bold text-gray-900 mb-3 tracking-tight">
          {t('error.message.title')}
        </h1>
        <p className="text-gray-500 mb-8 text-lg leading-relaxed">
          {t('error.message.description')}
        </p>
        
        {error && (
          <div className="mb-8 p-4 bg-red-50 text-red-700 rounded-lg border border-red-100 text-sm font-medium break-words">
            {error.toString()}
          </div>
        )}
        
        <div className="space-y-4">
          <button
            onClick={handleReload}
            className="w-full bg-red-600 hover:bg-red-700 text-white font-medium py-3.5 px-6 rounded-xl transition-all shadow-lg shadow-red-600/20 hover:shadow-red-600/30 flex items-center justify-center gap-2 active:scale-[0.98]"
          >
            <RotateCcw size={20} />
            {autoReloading ? t('error.status.reloading') : t('error.action.reload')}
          </button>

          {autoReloading && (
            <p className="text-sm text-gray-400 animate-pulse">
              {t('error.action.auto_reload')}
            </p>
          )}
        </div>

        {/* Technical Details Accordion */}
        <div className="mt-8 border-t border-gray-100 pt-6">
          <button 
            onClick={toggleDetails}
            className="flex items-center justify-center gap-2 text-sm text-gray-400 hover:text-gray-600 transition-colors w-full py-2"
          >
            {showDetails ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
            <span>{t('error.action.view_details')}</span>
          </button>

          <div className={`overflow-hidden transition-all duration-300 ease-in-out ${showDetails ? 'max-h-[31.25rem] opacity-100 mt-4' : 'max-h-0 opacity-0'}`}>
            <div className="bg-gray-50 rounded-lg border border-gray-200 text-left overflow-hidden relative group">
              <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
                <button 
                  onClick={copyError}
                  className="p-1.5 hover:bg-white rounded-md text-gray-400 hover:text-gray-600 transition-colors shadow-sm"
                  title={t('error.action.copy_details')}
                >
                  {copied ? <CheckCircle2 size={16} className="text-green-500" /> : <Copy size={16} />}
                </button>
              </div>
              <div className="p-4 overflow-auto max-h-64 scrollbar-thin scrollbar-thumb-gray-300 scrollbar-track-transparent">
                <p className="text-red-600 font-mono text-xs font-semibold mb-2">
                  {error?.toString()}
                </p>
                {errorInfo && (
                  <pre className="text-gray-500 font-mono text-[0.625rem] leading-relaxed whitespace-pre-wrap">
                    {errorInfo.componentStack}
                  </pre>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <style>{`
        @keyframes dash {
          from { stroke-dashoffset: 0; }
          to { stroke-dashoffset: 289; }
        }
      `}</style>
    </div>
  );
};

class GlobalErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { 
      hasError: false, 
      error: null, 
      errorInfo: null,
      showDetails: false,
      copied: false,
      autoReloading: false
    };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught error:", error, errorInfo);
    this.setState({ errorInfo });

    void reportError(
      error.message,
      error,
      'crash',
      {
        source: 'react_boundary',
        extras: {
          component_stack: errorInfo.componentStack,
        },
      }
    );

    // In dev mode, don't auto-reload so we can see console errors
    if (import.meta.env.DEV) {
      console.warn('[GlobalErrorBoundary] Dev mode: auto-reload disabled. Click reload button manually.');
      return;
    }

    const lastCrash = sessionStorage.getItem('last_crash_timestamp');
    const now = Date.now();
    const threshold = 10000;

    // If last crash was > threshold ago, it's safe-ish to reload automatically
    if (!lastCrash || (now - parseInt(lastCrash) > threshold)) {
       sessionStorage.setItem('last_crash_timestamp', now.toString());
       this.setState({ autoReloading: true });
       setTimeout(() => {
         window.location.reload();
       }, 3000);
    }
  }

  handleReload = () => {
    sessionStorage.setItem('last_crash_timestamp', Date.now().toString());
    window.location.reload();
  };

  toggleDetails = () => {
    this.setState(prevState => ({ showDetails: !prevState.showDetails }));
  };

  copyError = () => {
    const { error, errorInfo } = this.state;
    const text = `Error: ${error?.toString()}\n\nComponent Stack:${errorInfo?.componentStack}`;
    navigator.clipboard.writeText(text);
    this.setState({ copied: true });
    setTimeout(() => this.setState({ copied: false }), 2000);
  };

  render() {
    if (this.state.hasError) {
      return (
        <ErrorBoundaryUI 
          state={this.state}
          handleReload={this.handleReload}
          toggleDetails={this.toggleDetails}
          copyError={this.copyError}
        />
      );
    }

    return this.props.children;
  }
}

export default GlobalErrorBoundary;
