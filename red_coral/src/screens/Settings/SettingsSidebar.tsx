import React, { useState } from 'react';
import {
  ArrowLeft,
  Settings as SettingsIcon,
  Languages,
  Printer,
  LayoutGrid,
  Utensils,
  Tag,
  Tags,
  Sliders,
  Database,
  Store,
  Monitor,
  Users,
  ChevronDown,
  ChevronRight,
  Menu,
  Percent,
  Clock,
  FileText,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useSettingsCategory, useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';

type SettingsCategory = 'LANG' | 'PRINTER' | 'TABLES' | 'PRODUCTS' | 'CATEGORIES' | 'TAGS' | 'ATTRIBUTES' | 'PRICE_RULES' | 'DATA_TRANSFER' | 'STORE' | 'SYSTEM' | 'USERS' | 'SHIFTS' | 'DAILY_REPORTS';

interface SettingsSidebarProps {
  onBack: () => void;
}

interface CategoryItemProps {
  category: SettingsCategory;
  icon: React.FC<{ size?: number; className?: string }>;
  label: string;
}

const CategoryItem: React.FC<CategoryItemProps> = ({ category, icon: Icon, label }) => {
  const activeCategory = useSettingsCategory();
  const setActiveCategory = useSettingsStore((s) => s.setActiveCategory);
  const isActive = activeCategory === category;

  return (
    <button
      onClick={() => setActiveCategory(category)}
      className={`w-full px-3 py-2 text-left transition-all rounded-lg flex items-center gap-3 mb-1 ${isActive
          ? 'bg-red-50 text-red-600 shadow-sm'
          : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900'
        }`}
    >
      <Icon size={18} className={isActive ? 'text-red-500' : 'text-gray-500 group-hover:text-gray-700'} />
      <span className={`text-sm font-medium ${isActive ? 'font-semibold' : ''}`}>
        {label}
      </span>
      {isActive && <div className="ml-auto w-1.5 h-1.5 rounded-full bg-red-500" />}
    </button>
  );
};

interface CollapsibleGroupProps {
  title: string;
  icon?: React.FC<{ size?: number; className?: string }>;
  children: React.ReactNode;
  defaultOpen?: boolean;
}

const CollapsibleGroup: React.FC<CollapsibleGroupProps> = ({ title, icon: Icon, children, defaultOpen = false }) => {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="mb-1">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="w-full px-3 py-2 flex items-center justify-between text-gray-700 hover:bg-gray-50 rounded-lg transition-colors group"
      >
        <div className="flex items-center gap-3">
          {Icon && <Icon size={18} className="text-gray-500 group-hover:text-gray-700" />}
          <span className="text-sm font-semibold">{title}</span>
        </div>
        {isOpen ? (
          <ChevronDown size={16} className="text-gray-400" />
        ) : (
          <ChevronRight size={16} className="text-gray-400" />
        )}
      </button>
      
      <div 
        className={`overflow-hidden transition-all duration-300 ease-in-out ${
          isOpen ? 'max-h-[31.25rem] opacity-100' : 'max-h-0 opacity-0'
        }`}
      >
        <div className="pl-4 mt-1 border-l-2 border-gray-100 ml-4 space-y-1">
          {children}
        </div>
      </div>
    </div>
  );
};

export const SettingsSidebar: React.FC<SettingsSidebarProps> = ({ onBack }) => {
  const { t } = useI18n();
  const { hasPermission } = usePermission();

  // Visibility Logic
  const showGeneralSection = hasPermission(Permission.SYSTEM_SETTINGS);

  const showMenuManageGroup = hasPermission(Permission.CREATE_PRODUCT) ||
    hasPermission(Permission.MANAGE_CATEGORIES) ||
    hasPermission(Permission.MANAGE_ATTRIBUTES);

  const showStoreManageGroup = hasPermission(Permission.MANAGE_ZONES) ||
    hasPermission(Permission.MANAGE_TABLES);

  const showUsersItem = hasPermission(Permission.MANAGE_USERS);
  const showDataTransferItem = hasPermission(Permission.SYSTEM_SETTINGS);

  const showDataSection = showMenuManageGroup || showStoreManageGroup || showUsersItem || showDataTransferItem;

  return (
    <div className="w-64 bg-white border-r border-gray-200 flex flex-col shrink-0 h-full">
      <div className="p-4 border-b border-gray-100 shrink-0">
        <div className="flex items-center gap-3">
          <button
            onClick={onBack}
            className="p-2 -ml-2 hover:bg-gray-100 rounded-full text-gray-500 hover:text-gray-700 transition-colors"
          >
            <ArrowLeft size={20} />
          </button>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 bg-red-50 rounded-lg flex items-center justify-center">
              <SettingsIcon className="text-red-500" size={18} />
            </div>
            <span className="font-bold text-gray-800 text-lg">{t('settings.sidebar.title')}</span>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto py-4 px-2 custom-scrollbar">
        <div className="space-y-6">

          {/* General Group */}
          {showGeneralSection && (
            <div>
              <div className="px-4 mb-2 text-xs font-semibold text-gray-400 uppercase tracking-wider">
                {t('settings.group.general')}
              </div>
              <ProtectedGate permission={Permission.SYSTEM_SETTINGS}>
                <CategoryItem
                  category="STORE"
                  icon={Store}
                  label={t('settings.store.title')}
                />
              </ProtectedGate>
            </div>
          )}

          {/* Data Management Group */}
          {showDataSection && (
            <div>
              <div className="px-4 mb-2 text-xs font-semibold text-gray-400 uppercase tracking-wider">
                {t('settings.group.data')}
              </div>

              {/* Menu Management Sub-group */}
              {showMenuManageGroup && (
                <CollapsibleGroup title={t("settings.permissions.group.menu")} icon={Menu} defaultOpen={true}>
                  <ProtectedGate permission={Permission.CREATE_PRODUCT}>
                    <CategoryItem
                      category="PRODUCTS"
                      icon={Utensils}
                      label={t('settings.product.title')}
                    />
                  </ProtectedGate>
                  <ProtectedGate permission={Permission.MANAGE_CATEGORIES}>
                    <CategoryItem
                      category="CATEGORIES"
                      icon={Tag}
                      label={t('settings.category.title')}
                    />
                  </ProtectedGate>
                  <ProtectedGate permission={Permission.MANAGE_CATEGORIES}>
                    <CategoryItem
                      category="TAGS"
                      icon={Tags}
                      label={t('settings.tag.title')}
                    />
                  </ProtectedGate>
                  <ProtectedGate permission={Permission.MANAGE_ATTRIBUTES}>
                    <CategoryItem
                      category="ATTRIBUTES"
                      icon={Sliders}
                      label={t('settings.attribute.title')}
                    />
                  </ProtectedGate>
                  <ProtectedGate permission={Permission.SYSTEM_SETTINGS}>
                    <CategoryItem
                      category="PRICE_RULES"
                      icon={Percent}
                      label={t('settings.price_rule.title')}
                    />
                  </ProtectedGate>
                </CollapsibleGroup>
              )}

              {/* Space Management Sub-group */}
              {showStoreManageGroup && (
                <CollapsibleGroup title={t("settings.permissions.group.store")} icon={LayoutGrid} defaultOpen={false}>
                  <CategoryItem
                    category="TABLES"
                    icon={LayoutGrid}
                    label={t('settings.table.title')}
                  />
                </CollapsibleGroup>
              )}

              <div className="mt-2">
                <ProtectedGate permission={Permission.MANAGE_USERS}>
                  <CategoryItem
                    category="USERS"
                    icon={Users}
                    label={t('settings.user.title')}
                  />
                </ProtectedGate>
                <ProtectedGate permission={Permission.SYSTEM_SETTINGS}>
                  <CategoryItem
                    category="DATA_TRANSFER"
                    icon={Database}
                    label={t('settings.data_transfer.title')}
                  />
                </ProtectedGate>
              </div>
            </div>
          )}

          {/* Operations Group (班次与日结) */}
          <div>
            <div className="px-4 mb-2 text-xs font-semibold text-gray-400 uppercase tracking-wider">
              {t('settings.group.operations')}
            </div>
            <CategoryItem
              category="SHIFTS"
              icon={Clock}
              label={t('settings.shift.title')}
            />
            <ProtectedGate permission={Permission.SYSTEM_SETTINGS}>
              <CategoryItem
                category="DAILY_REPORTS"
                icon={FileText}
                label={t('settings.daily_report.title')}
              />
            </ProtectedGate>
          </div>

          {/* System Settings Group */}
          <div>
            <div className="px-4 mb-2 text-xs font-semibold text-gray-400 uppercase tracking-wider">
              {t('settings.group.system')}
            </div>
            <CategoryItem category="LANG" icon={Languages} label={t('settings.language.title')} />
            <ProtectedGate permission={Permission.MANAGE_PRINTERS}>
              <CategoryItem category="PRINTER" icon={Printer} label={t('settings.printer.title')} />
            </ProtectedGate>
            <ProtectedGate permission={Permission.SYSTEM_SETTINGS}>
              <CategoryItem
                category="SYSTEM"
                icon={Monitor}
                label={t('settings.system.title')}
              />
            </ProtectedGate>
          </div>
        </div>
      </div>
    </div>
  );
};
