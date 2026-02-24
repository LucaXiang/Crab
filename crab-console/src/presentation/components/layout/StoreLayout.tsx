import React, { useEffect, useState } from 'react';
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
  Settings, ChevronLeft, LogOut, Globe, ChevronDown,
  Radio, ShoppingBag, BarChart3, Package, FolderTree,
  Tag, SlidersHorizontal, Percent, Users, Map, Grid3x3, ShieldAlert, ScrollText, Tags,
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
        setStoreName(store.name ?? (store.store_info?.name as string) ?? `Store #${storeId}`);
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
        { key: 'nav.daily_report', href: `/stores/${storeId}/stats`, icon: ScrollText },
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

  return (
    <div className="flex h-dvh overflow-hidden bg-slate-50">
      {/* Store sidebar (desktop only) */}
      <aside className="hidden md:flex md:w-48 flex-col bg-white border-r border-slate-100 shrink-0">
        <div className="h-14 flex items-center gap-2 px-4 border-b border-slate-100">
          <Link to="/stores" className="text-slate-400 hover:text-slate-600 shrink-0" title={t('store.back')}>
            <ChevronLeft className="w-4 h-4" />
          </Link>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-semibold text-slate-900 truncate">{storeName || '...'}</p>
            {storeOnline && (
              <p className="text-[10px] text-green-500 flex items-center gap-1">
                <span className="w-1.5 h-1.5 bg-green-400 rounded-full inline-block" />
                Online
              </p>
            )}
          </div>
        </div>
        <nav className="flex-1 overflow-y-auto px-2 py-3 space-y-4">
          {storeNav.map(group => (
            <div key={group.label}>
              <p className="px-2.5 mb-1 text-[10px] font-semibold text-slate-400 uppercase tracking-wider">{t(group.label)}</p>
              <div className="space-y-0.5">
                {group.items.map(item => (
                  <Link key={item.href} to={item.href}
                    className={`flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[13px] font-medium transition-colors ${
                      isActive(item.href) ? 'bg-primary-50 text-primary-600' : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'
                    }`}
                  >
                    <item.icon className="w-3.5 h-3.5" />
                    <span>{t(item.key)}</span>
                  </Link>
                ))}
              </div>
            </div>
          ))}
          <div>
            <Link to={`/stores/${storeId}`}
              className={`flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[13px] font-medium transition-colors ${
                location.pathname === `/stores/${storeId}` ? 'bg-primary-50 text-primary-600' : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'
              }`}
            >
              <Settings className="w-3.5 h-3.5" />
              <span>{t('nav.store_settings')}</span>
            </Link>
          </div>
        </nav>
        <div className="px-2 pb-3 space-y-1 border-t border-slate-100 pt-2">
          <div className="relative">
            <button onClick={() => setLangOpen(!langOpen)} className="flex items-center gap-2 w-full px-2.5 py-1.5 rounded-lg text-xs text-slate-400 hover:bg-slate-50 cursor-pointer">
              <Globe className="w-3.5 h-3.5" />
              <span>{LANG_LABELS[locale]}</span>
              <ChevronDown className="w-3 h-3 ml-auto" />
            </button>
            {langOpen && (
              <div className="absolute bottom-full left-0 mb-1 w-full bg-white border border-slate-200 rounded-lg shadow-lg py-1 z-10">
                {SUPPORTED_LOCALES.map(lang => (
                  <button key={lang} onClick={() => { setLocale(lang as Locale); setLangOpen(false); }}
                    className={`block w-full text-left px-3 py-1.5 text-sm hover:bg-slate-50 cursor-pointer ${locale === lang ? 'text-primary-500 font-medium' : 'text-slate-600'}`}
                  >{LANG_LABELS[lang]}</button>
                ))}
              </div>
            )}
          </div>
          <button onClick={() => { clearAuth(); navigate('/login'); }} className="flex items-center gap-2 w-full px-2.5 py-1.5 rounded-lg text-xs text-slate-400 hover:bg-slate-50 hover:text-slate-600 cursor-pointer">
            <LogOut className="w-3.5 h-3.5" />
            <span>{t('nav.logout')}</span>
          </button>
          <p className="px-2.5 text-[10px] text-slate-300">v{__APP_VERSION__} ({__GIT_HASH__})</p>
        </div>
      </aside>

      {/* Mobile + Content */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Mobile store header */}
        <header className="md:hidden h-12 flex items-center gap-2 px-3 bg-white border-b border-slate-200 shrink-0">
          <Link to="/stores" className="text-slate-400 hover:text-slate-600 shrink-0">
            <ChevronLeft className="w-5 h-5" />
          </Link>
          <span className="text-sm font-semibold text-slate-900 truncate">{storeName || '...'}</span>
          {storeOnline && <span className="w-1.5 h-1.5 bg-green-400 rounded-full shrink-0" />}
        </header>

        {/* Mobile tabs */}
        <nav className="md:hidden flex overflow-x-auto bg-white border-b border-slate-100 px-2 gap-1 no-scrollbar shrink-0">
          {mobileTabItems.map(item => (
            <Link key={item.href} to={item.href}
              className={`flex items-center gap-1.5 px-3 py-2.5 text-xs font-medium whitespace-nowrap border-b-2 transition-colors ${
                isActive(item.href) ? 'border-primary-500 text-primary-600' : 'border-transparent text-slate-500 hover:text-slate-700'
              }`}
            >
              <item.icon className="w-3.5 h-3.5" />
              <span>{t(item.key)}</span>
            </Link>
          ))}
          <Link to={`/stores/${storeId}`}
            className={`flex items-center gap-1.5 px-3 py-2.5 text-xs font-medium whitespace-nowrap border-b-2 transition-colors ${
              location.pathname === `/stores/${storeId}` ? 'border-primary-500 text-primary-600' : 'border-transparent text-slate-500 hover:text-slate-700'
            }`}
          >
            <Settings className="w-3.5 h-3.5" />
            <span>{t('nav.store_settings')}</span>
          </Link>
        </nav>

        <main className="flex-1 overflow-y-auto">
          <Outlet />
        </main>
      </div>
    </div>
  );
};
