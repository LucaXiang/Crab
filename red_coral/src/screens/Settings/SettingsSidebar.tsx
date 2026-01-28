import React from 'react';
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
  Percent,
  Clock,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useSettingsCategory, useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission } from '@/core/domain/types';

type SettingsCategory = 'LANG' | 'PRINTER' | 'TABLES' | 'PRODUCTS' | 'CATEGORIES' | 'TAGS' | 'ATTRIBUTES' | 'PRICE_RULES' | 'DATA_TRANSFER' | 'STORE' | 'SYSTEM' | 'USERS' | 'SHIFTS';

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
          ? 'bg-primary-50 text-primary-600 shadow-sm'
          : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900'
        }`}
    >
      <Icon size={18} className={isActive ? 'text-primary-500' : 'text-gray-500 group-hover:text-gray-700'} />
      <span className={`text-sm font-medium ${isActive ? 'font-semibold' : ''}`}>
        {label}
      </span>
      {isActive && <div className="ml-auto w-1.5 h-1.5 rounded-full bg-primary-500" />}
    </button>
  );
};

const Divider: React.FC = () => (
  <div className="my-3 mx-3 border-t border-gray-200" />
);

export const SettingsSidebar: React.FC<SettingsSidebarProps> = ({ onBack }) => {
  const { t } = useI18n();

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
            <div className="w-8 h-8 bg-primary-50 rounded-lg flex items-center justify-center">
              <SettingsIcon className="text-primary-500" size={18} />
            </div>
            <span className="font-bold text-gray-800 text-lg">{t('settings.sidebar.title')}</span>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto py-4 px-2 custom-scrollbar">
        {/* 店铺信息 & 桌台管理 */}
        <ProtectedGate permission={Permission.SYSTEM_WRITE}>
          <CategoryItem
            category="STORE"
            icon={Store}
            label={t('settings.store.title')}
          />
        </ProtectedGate>
        <ProtectedGate permission={Permission.TABLES_MANAGE}>
          <CategoryItem
            category="TABLES"
            icon={LayoutGrid}
            label={t('settings.table.title')}
          />
        </ProtectedGate>

        <Divider />

        {/* 商品管理相关 */}
        <ProtectedGate permission={Permission.PRODUCTS_WRITE}>
          <CategoryItem
            category="PRODUCTS"
            icon={Utensils}
            label={t('settings.product.title')}
          />
        </ProtectedGate>
        <ProtectedGate permission={Permission.CATEGORIES_MANAGE}>
          <CategoryItem
            category="CATEGORIES"
            icon={Tag}
            label={t('settings.category.title')}
          />
        </ProtectedGate>
        <ProtectedGate permission={Permission.ATTRIBUTES_MANAGE}>
          <CategoryItem
            category="ATTRIBUTES"
            icon={Sliders}
            label={t('settings.attribute.title')}
          />
        </ProtectedGate>
        <ProtectedGate permission={Permission.CATEGORIES_MANAGE}>
          <CategoryItem
            category="TAGS"
            icon={Tags}
            label={t('settings.tag.title')}
          />
        </ProtectedGate>
        <ProtectedGate permission={Permission.SYSTEM_WRITE}>
          <CategoryItem
            category="PRICE_RULES"
            icon={Percent}
            label={t('settings.price_rule.title')}
          />
        </ProtectedGate>

        <Divider />

        {/* 员工管理 & 数据转移 */}
        <ProtectedGate permission={Permission.USERS_MANAGE}>
          <CategoryItem
            category="USERS"
            icon={Users}
            label={t('settings.user.title')}
          />
        </ProtectedGate>
        <ProtectedGate permission={Permission.SYSTEM_WRITE}>
          <CategoryItem
            category="DATA_TRANSFER"
            icon={Database}
            label={t('settings.data_transfer.title')}
          />
        </ProtectedGate>

        <Divider />

        {/* 班次管理 */}
        <ProtectedGate permission={Permission.SYSTEM_WRITE}>
          <CategoryItem
            category="SHIFTS"
            icon={Clock}
            label={t('settings.shift.title')}
          />
        </ProtectedGate>

        <Divider />

        {/* 系统设置 */}
        <CategoryItem category="LANG" icon={Languages} label={t('settings.language.title')} />
        <ProtectedGate permission={Permission.PRINTERS_MANAGE}>
          <CategoryItem category="PRINTER" icon={Printer} label={t('settings.printer.title')} />
        </ProtectedGate>
        <ProtectedGate permission={Permission.SYSTEM_WRITE}>
          <CategoryItem
            category="SYSTEM"
            icon={Monitor}
            label={t('settings.system.title')}
          />
        </ProtectedGate>
      </div>
    </div>
  );
};
