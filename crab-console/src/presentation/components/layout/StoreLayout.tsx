import React, { useEffect, useState } from 'react';
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
  ArrowLeftRight, BarChart3, ChevronDown, ChevronLeft, FolderTree, Globe, Grid3x3, LogOut, Map, Menu,
  Package, Percent, Radio, ScrollText, Settings, ShieldAlert, ShoppingBag, SlidersHorizontal, Tag, Tags,
  Users, X,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getStores } from '@/infrastructure/api/stores';
import { ApiError } from '@/infrastructure/api/client';
import { SUPPORTED_LOCALES, LANG_LABELS, type Locale } from '@/infrastructure/i18n';

interface NavItem { key: string; href: string; icon: React.FC<{ className?: string }> }
interface NavGroup { label: string; items: NavItem[] }

export const StoreLayout: React.FC = () => {
  const { t, locale, setLocale } = useI18n();
  const location = useLocation();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);
  const [storeName, setStoreName] = useState('');
  const [storeOnline, setStoreOnline] = useState(false);
  const [langOpen, setLangOpen] = useState(false);

  useEffect(() => {
    if (!token) return;
    getStores(token).then(stores => {
      const store = stores.find(s => s.id === storeId);
      if (store) {
        setStoreName(store.alias);
        setStoreOnline(store.is_online);
      }
    }).catch(err => {
      if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); }
    });
  }, [token, storeId, clearAuth, navigate]);

  const storeNav: NavGroup[] = [
    {
      label: 'nav.group_operations',
      items: [
        { key: 'nav.overview', href: `/stores/${storeId}/overview`, icon: BarChart3 },
        { key: 'nav.live_orders', href: `/stores/${storeId}/live`, icon: Radio },
        { key: 'nav.orders', href: `/stores/${storeId}/orders`, icon: ShoppingBag },
        { key: 'nav.daily_report', href: `/stores/${storeId}/reports`, icon: ScrollText },
      ],
    },
    {
      label: 'nav.group_catalog',
      items: [
        { key: 'nav.products', href: `/stores/${storeId}/products`, icon: Package },
        { key: 'nav.categories', href: `/stores/${storeId}/categories`, icon: FolderTree },
        { key: 'nav.tags', href: `/stores/${storeId}/tags`, icon: Tag },
        { key: 'nav.attributes', href: `/stores/${storeId}/attributes`, icon: SlidersHorizontal },
        { key: 'nav.price_rules', href: `/stores/${storeId}/price-rules`, icon: Percent },
      ],
    },
    {
      label: 'nav.group_manage',
      items: [
        { key: 'nav.employees', href: `/stores/${storeId}/employees`, icon: Users },
        { key: 'nav.zones', href: `/stores/${storeId}/zones`, icon: Map },
        { key: 'nav.tables', href: `/stores/${storeId}/tables`, icon: Grid3x3 },
        { key: 'nav.label_templates', href: `/stores/${storeId}/label-templates`, icon: Tags },
        { key: 'nav.data_transfer', href: `/stores/${storeId}/data-transfer`, icon: ArrowLeftRight },
      ],
    },
    {
      label: 'nav.group_monitor',
      items: [
        { key: 'nav.red_flags', href: `/stores/${storeId}/red-flags`, icon: ShieldAlert },
      ],
    },
  ];

  const mobileTabItems = storeNav.flatMap(g => g.items);
  const isActive = (href: string) => location.pathname === href || location.pathname.startsWith(href + '/');

  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <div className="flex h-dvh overflow-hidden bg-slate-50">
      {/* Store sidebar (desktop only) */}
      <aside className="hidden md:flex md:w-60 flex-col bg-white border-r border-slate-100 shrink-0 h-full">
        <div className="h-16 flex items-center gap-3 px-5 border-b border-slate-100">
          <Link to="/stores" className="p-1.5 -ml-1.5 text-slate-400 hover:text-slate-600 hover:bg-slate-50 rounded-lg transition-colors shrink-0" title={t('store.back')}>
            <ChevronLeft className="w-5 h-5" />
          </Link>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-semibold text-slate-900 truncate">{storeName || '...'}</p>
            {storeOnline && (
              <p className="text-[10px] text-green-500 flex items-center gap-1.5 mt-0.5">
                <span className="w-1.5 h-1.5 bg-green-500 rounded-full inline-block animate-pulse" />
                Online
              </p>
            )}
          </div>
        </div>

        <nav className="flex-1 overflow-y-auto px-3 py-4 space-y-6">
          {storeNav.map(group => (
            <div key={group.label}>
              <p className="px-3 mb-2 text-xs font-semibold text-slate-400 uppercase tracking-wider">{t(group.label)}</p>
              <div className="space-y-1">
                {group.items.map(item => (
                  <Link key={item.href} to={item.href}
                    className={`flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      isActive(item.href) ? 'bg-primary-50 text-primary-600' : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'
                    }`}
                  >
                    <item.icon className="w-4.5 h-4.5" />
                    <span>{t(item.key)}</span>
                  </Link>
                ))}
              </div>
            </div>
          ))}
          <div className="pt-2 border-t border-slate-100 mt-2">
            <Link to={`/stores/${storeId}`}
              className={`flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                location.pathname === `/stores/${storeId}` ? 'bg-primary-50 text-primary-600' : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'
              }`}
            >
              <Settings className="w-4.5 h-4.5" />
              <span>{t('nav.store_settings')}</span>
            </Link>
          </div>
        </nav>
        
        <div className="px-3 pb-4 space-y-2 border-t border-slate-100 pt-3 bg-white">
          <div className="relative">
            <button onClick={() => setLangOpen(!langOpen)} className="flex items-center gap-2 w-full px-3 py-2 rounded-lg text-sm text-slate-500 hover:bg-slate-50 cursor-pointer">
              <Globe className="w-4 h-4" />
              <span>{LANG_LABELS[locale]}</span>
              <ChevronDown className="w-3 h-3 ml-auto" />
            </button>
            {langOpen && (
              <div className="absolute bottom-full left-0 mb-1 w-full bg-white border border-slate-200 rounded-lg shadow-lg py-1 z-10">
                {SUPPORTED_LOCALES.map(lang => (
                  <button key={lang} onClick={() => { setLocale(lang as Locale); setLangOpen(false); }}
                    className={`block w-full text-left px-3 py-2 text-sm hover:bg-slate-50 cursor-pointer ${locale === lang ? 'text-primary-500 font-medium' : 'text-slate-600'}`}
                  >{LANG_LABELS[lang]}</button>
                ))}
              </div>
            )}
          </div>
          <button onClick={() => { clearAuth(); navigate('/login'); }} className="flex items-center gap-2 w-full px-3 py-2 rounded-lg text-sm text-slate-400 hover:bg-slate-50 hover:text-slate-600 cursor-pointer">
            <LogOut className="w-4 h-4" />
            <span>{t('nav.logout')}</span>
          </button>
          <p className="px-3 text-[10px] text-slate-300">v{__APP_VERSION__} ({__GIT_HASH__})</p>
        </div>
      </aside>

      {/* Mobile + Content */}
      <div className="flex-1 flex flex-col min-w-0 h-full relative">
        {/* Mobile store header */}
        <header className="md:hidden h-14 flex items-center justify-between px-4 bg-white border-b border-slate-200 shrink-0 z-30 relative">
          <div className="flex items-center gap-3 min-w-0">
            <Link to="/stores" className="p-1 -ml-1 text-slate-400 hover:text-slate-600 shrink-0">
              <ChevronLeft className="w-6 h-6" />
            </Link>
            <div className="flex flex-col min-w-0">
              <span className="text-sm font-semibold text-slate-900 truncate">{storeName || '...'}</span>
              {storeOnline && <span className="text-[10px] text-green-500 leading-none">Online</span>}
            </div>
          </div>
          <button 
            onClick={() => setMobileOpen(true)}
            className="p-2 -mr-2 text-slate-600 hover:bg-slate-50 rounded-full"
          >
            <Menu className="w-6 h-6" />
          </button>
        </header>

        {/* Mobile Drawer */}
        {mobileOpen && (
          <div className="md:hidden fixed inset-0 z-50 flex">
            <div 
              className="fixed inset-0 bg-slate-900/20 backdrop-blur-sm transition-opacity" 
              onClick={() => setMobileOpen(false)} 
            />
            
            <div className="relative flex-1 flex flex-col max-w-xs w-full bg-white shadow-xl transform transition-transform duration-300 ease-in-out h-full">
              <div className="flex items-center justify-between h-16 px-5 border-b border-slate-100">
                <span className="font-bold text-lg text-slate-900">{t('nav.menu')}</span>
                <button 
                  onClick={() => setMobileOpen(false)}
                  className="p-2 -mr-2 text-slate-400 hover:text-slate-600 rounded-full hover:bg-slate-100"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              <div className="flex-1 overflow-y-auto py-4 px-4 space-y-6">
                {storeNav.map(group => (
                  <div key={group.label}>
                    <p className="px-2 mb-2 text-xs font-semibold text-slate-400 uppercase tracking-wider">{t(group.label)}</p>
                    <div className="space-y-1">
                      {group.items.map(item => (
                        <Link key={item.href} to={item.href}
                          onClick={() => setMobileOpen(false)}
                          className={`flex items-center gap-3 px-3 py-3 rounded-xl text-sm font-medium transition-colors ${
                            isActive(item.href) ? 'bg-primary-50 text-primary-600' : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'
                          }`}
                        >
                          <item.icon className="w-5 h-5" />
                          <span>{t(item.key)}</span>
                        </Link>
                      ))}
                    </div>
                  </div>
                ))}
                
                <div className="pt-2 border-t border-slate-100">
                  <Link to={`/stores/${storeId}`}
                    onClick={() => setMobileOpen(false)}
                    className={`flex items-center gap-3 px-3 py-3 rounded-xl text-sm font-medium transition-colors ${
                      location.pathname === `/stores/${storeId}` ? 'bg-primary-50 text-primary-600' : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'
                    }`}
                  >
                    <Settings className="w-5 h-5" />
                    <span>{t('nav.store_settings')}</span>
                  </Link>
                </div>
              </div>
            </div>
          </div>
        )}

        <main className="flex-1 overflow-y-auto bg-slate-50/50">
          <Outlet />
        </main>
      </div>
    </div>
  );
};
