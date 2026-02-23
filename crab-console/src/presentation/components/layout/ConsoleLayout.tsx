import React, { useState } from 'react';
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
  LayoutDashboard, Store, ScrollText, Settings, LogOut,
  Globe, ChevronDown, Menu, X,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { SUPPORTED_LOCALES, LANG_LABELS, type Locale } from '@/infrastructure/i18n';
import { LogoBadge, LogoText } from './Logo';

const navItems = [
  { key: 'nav.dashboard', href: '/', icon: LayoutDashboard },
  { key: 'nav.stores', href: '/stores', icon: Store, match: '/stores' },
  { key: 'nav.audit', href: '/audit', icon: ScrollText },
  { key: 'nav.settings', href: '/settings', icon: Settings },
] as const;

export const ConsoleLayout: React.FC = () => {
  const { t, locale, setLocale } = useI18n();
  const location = useLocation();
  const navigate = useNavigate();
  const clearAuth = useAuthStore(s => s.clearAuth);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [langOpen, setLangOpen] = useState(false);

  const isActive = (href: string, match?: string) => {
    if (match) return location.pathname.startsWith(match);
    return location.pathname === href;
  };

  const handleLogout = () => {
    clearAuth();
    navigate('/login');
  };

  return (
    <div className="flex h-dvh overflow-hidden">
      {/* Desktop sidebar */}
      <aside className="hidden md:flex md:w-60 flex-col bg-white border-r border-slate-200">
        <div className="h-16 flex items-center px-5 border-b border-slate-100">
          <Link to="/" className="flex items-center gap-2">
            <LogoBadge />
            <LogoText />
          </Link>
        </div>

        <nav className="flex-1 px-3 py-4 space-y-1">
          {navItems.map(item => (
            <Link
              key={item.href}
              to={item.href}
              className={`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors duration-150 ${
                isActive(item.href, 'match' in item ? item.match : undefined)
                  ? 'bg-primary-50 text-primary-600'
                  : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'
              }`}
            >
              <item.icon className="w-4.5 h-4.5" />
              <span>{t(item.key)}</span>
            </Link>
          ))}
        </nav>

        <div className="px-3 pb-4 space-y-2">
          {/* Language */}
          <div className="relative">
            <button
              onClick={() => setLangOpen(!langOpen)}
              className="flex items-center gap-2 w-full px-3 py-2 rounded-lg text-sm text-slate-500 hover:bg-slate-50 cursor-pointer"
            >
              <Globe className="w-4 h-4" />
              <span>{LANG_LABELS[locale]}</span>
              <ChevronDown className="w-3 h-3 ml-auto" />
            </button>
            {langOpen && (
              <div className="absolute bottom-full left-0 mb-1 w-full bg-white border border-slate-200 rounded-lg shadow-lg py-1 z-10">
                {SUPPORTED_LOCALES.map(lang => (
                  <button
                    key={lang}
                    onClick={() => { setLocale(lang as Locale); setLangOpen(false); }}
                    className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-slate-50 cursor-pointer ${locale === lang ? 'text-primary-500 font-medium' : 'text-slate-600'}`}
                  >
                    {LANG_LABELS[lang]}
                  </button>
                ))}
              </div>
            )}
          </div>

          <button
            onClick={handleLogout}
            className="flex items-center gap-2 w-full px-3 py-2 rounded-lg text-sm text-slate-400 hover:bg-slate-50 hover:text-slate-600 cursor-pointer"
          >
            <LogOut className="w-4 h-4" />
            <span>{t('nav.logout')}</span>
          </button>
          <p className="px-3 pb-1 text-[10px] text-slate-300">v{__APP_VERSION__} ({__GIT_HASH__})</p>
        </div>
      </aside>

      {/* Mobile + Content */}
      <div className="flex-1 flex flex-col min-w-0 relative">
        {/* Mobile header */}
        <header className="md:hidden h-14 flex items-center justify-between px-4 bg-white border-b border-slate-200 shrink-0 relative z-50">
          <Link to="/" className="flex items-center gap-2">
            <LogoBadge size="sm" />
            <LogoText className="text-base" />
          </Link>
          <button onClick={() => setMobileOpen(!mobileOpen)} className="text-slate-600 cursor-pointer">
            {mobileOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
          </button>
        </header>

        {/* Mobile nav overlay */}
        {mobileOpen && (
          <>
            <div className="md:hidden fixed inset-0 bg-slate-900/20 backdrop-blur-sm z-40" onClick={() => setMobileOpen(false)} />
            <div className="md:hidden absolute inset-x-0 top-14 bg-white border-b border-slate-200 shadow-xl z-50 px-4 py-3 space-y-1 max-h-[calc(100vh-3.5rem)] overflow-y-auto">
              {navItems.map(item => (
                <Link
                  key={item.href}
                  to={item.href}
                  onClick={() => setMobileOpen(false)}
                  className={`flex items-center gap-3 px-3 py-3 rounded-xl text-sm font-medium ${
                    isActive(item.href, 'match' in item ? item.match : undefined)
                      ? 'bg-primary-50 text-primary-600'
                      : 'text-slate-600'
                  }`}
                >
                  <item.icon className="w-5 h-5" />
                  <span>{t(item.key)}</span>
                </Link>
              ))}
              <div className="border-t border-slate-100 my-2 pt-2 space-y-1">
                <div className="px-3 py-2">
                  <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{t('settings.language')}</p>
                  <div className="flex flex-wrap gap-2">
                    {SUPPORTED_LOCALES.map(lang => (
                      <button
                        key={lang}
                        onClick={() => setLocale(lang as Locale)}
                        className={`px-3 py-1.5 text-xs rounded-lg border transition-colors ${locale === lang ? 'bg-primary-50 border-primary-200 text-primary-600 font-medium' : 'bg-white border-slate-200 text-slate-600'}`}
                      >
                        {LANG_LABELS[lang]}
                      </button>
                    ))}
                  </div>
                </div>
                <button
                  onClick={handleLogout}
                  className="flex items-center gap-3 px-3 py-3 rounded-xl text-sm font-medium text-slate-600 w-full cursor-pointer hover:bg-slate-50"
                >
                  <LogOut className="w-5 h-5" />
                  <span>{t('nav.logout')}</span>
                </button>
              </div>
            </div>
          </>
        )}

        <main className="flex-1 overflow-y-auto">
          <Outlet />
        </main>
      </div>
    </div>
  );
};
