import { Component, ReactNode } from 'react';
import { reportError } from '@/utils/reportError';

interface SectionErrorBoundaryProps {
  children: ReactNode;
  region: string;
  title?: string;
  description?: string;
  allowRetry?: boolean;
  autoReload?: boolean;
}

interface SectionErrorBoundaryState {
  hasError: boolean;
  autoReloading: boolean;
}

export class SectionErrorBoundary extends Component<
  SectionErrorBoundaryProps,
  SectionErrorBoundaryState
> {
  constructor(props: SectionErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, autoReloading: false };
    this.handleRetry = this.handleRetry.bind(this);
  }

  static getDerivedStateFromError(): Partial<SectionErrorBoundaryState> {
    return { hasError: true };
  }

  componentDidCatch(error: Error) {
    void reportError(
      `Section crashed: ${this.props.region}`,
      error,
      this.props.region,
      { source: 'react_section_boundary' }
    );

    if (this.props.autoReload) {
      const lastCrash = sessionStorage.getItem('last_crash_timestamp');
      const now = Date.now();
      const threshold = import.meta.env.DEV ? 2000 : 10000;

      // If last crash was > threshold ago, it's safe-ish to reload automatically
      if (!lastCrash || (now - parseInt(lastCrash) > threshold)) {
        sessionStorage.setItem('last_crash_timestamp', now.toString());
        this.setState({ autoReloading: true });
        setTimeout(() => {
          window.location.reload();
        }, 3000);
      }
    }
  }

  handleRetry() {
    this.setState({ hasError: false });
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-full w-full items-center justify-center bg-gray-50/80">
          <div className="max-w-sm w-full px-6 py-5 rounded-2xl border border-red-100 bg-white/80 shadow-sm text-center space-y-3">
            <div className="mx-auto flex h-10 w-10 items-center justify-center rounded-full bg-red-50 text-red-500 text-lg">
              !
            </div>
            <div className="space-y-1">
              <div className="text-sm font-semibold text-gray-900">
                {this.props.title || 'Section error'}
              </div>
              <div className="text-xs text-gray-500">
                {this.state.autoReloading 
                  ? 'Reloading application...' 
                  : (this.props.description || 'This area failed to load. You can try again.')}
              </div>
            </div>
            {!this.state.autoReloading && this.props.allowRetry !== false && (
              <button
                type="button"
                onClick={this.handleRetry}
                className="mt-2 inline-flex items-center justify-center rounded-full bg-red-500 px-4 py-1.5 text-xs font-semibold text-white shadow-sm hover:bg-red-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-400 focus-visible:ring-offset-1"
              >
                Retry
              </button>
            )}
            {this.state.autoReloading && (
               <div className="mt-2 text-xs text-gray-400 animate-pulse">
                 Please wait...
               </div>
            )}
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

