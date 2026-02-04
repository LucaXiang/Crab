import React from 'react';
import { useSettingsCategory } from '@/core/stores/settings/useSettingsStore';
import { SettingsSidebar } from './SettingsSidebar';
import { LanguageSettings } from './LanguageSettings';
import { PrinterSettings } from './PrinterSettings';
import { TableManagement } from '@/features/table';
import { ProductManagement } from '@/features/product';
import { CategoryManagement } from '@/features/category';
import { TagManagement } from '@/features/tag';
import { AttributeManagement } from '@/features/attribute';
import { PriceRuleManagement } from '@/features/price-rule';
import { ShiftManagement } from '@/features/shift';
import { DataTransfer } from './DataTransfer';
import { StoreSettings } from './StoreSettings';
import { SystemSettings } from './SystemSettings';
import { UserManagement } from '@/features/user';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission } from '@/core/domain/types';
import { TableModal } from '@/features/table';
import { ProductModal } from '@/features/product';
import { CategoryModal } from '@/features/category';
import { TagModal } from '@/features/tag';
import { ZoneModal } from '@/features/zone';

interface SettingsScreenProps {
  onBack: () => void;
}

const SettingsContent: React.FC = React.memo(() => {
  const activeCategory = useSettingsCategory();

  return (
    <div className="flex-1 overflow-y-auto bg-gray-50/50">
      <div className="p-6 md:p-8 max-w-7xl mx-auto w-full space-y-6">
        {activeCategory === 'TABLES' && (
          <ProtectedGate permission={Permission.TABLES_MANAGE}>
            <TableManagement initialTab="tables" />
          </ProtectedGate>
        )}
        {activeCategory === 'PRODUCTS' && (
          <ProtectedGate permission={Permission.MENU_MANAGE}>
            <ProductManagement />
          </ProtectedGate>
        )}
        {activeCategory === 'CATEGORIES' && (
          <ProtectedGate permission={Permission.MENU_MANAGE}>
            <CategoryManagement />
          </ProtectedGate>
        )}
        {activeCategory === 'TAGS' && (
          <ProtectedGate permission={Permission.MENU_MANAGE}>
            <TagManagement />
          </ProtectedGate>
        )}
        {activeCategory === 'ATTRIBUTES' && (
          <ProtectedGate permission={Permission.MENU_MANAGE}>
            <AttributeManagement />
          </ProtectedGate>
        )}
        {activeCategory === 'PRICE_RULES' && (
          <ProtectedGate permission={Permission.PRICE_RULES_MANAGE}>
            <PriceRuleManagement />
          </ProtectedGate>
        )}
        {activeCategory === 'SHIFTS' && (
          <ProtectedGate permission={Permission.SHIFTS_MANAGE}>
            <ShiftManagement />
          </ProtectedGate>
        )}
        {activeCategory === 'USERS' && (
          <ProtectedGate permission={Permission.USERS_MANAGE}>
            <UserManagement />
          </ProtectedGate>
        )}
        {activeCategory === 'LANG' && <LanguageSettings />}
        {activeCategory === 'PRINTER' && (
          <ProtectedGate permission={Permission.SETTINGS_MANAGE}>
            <PrinterSettings />
          </ProtectedGate>
        )}
        {activeCategory === 'DATA_TRANSFER' && (
          <ProtectedGate permission={Permission.SETTINGS_MANAGE}>
            <DataTransfer />
          </ProtectedGate>
        )}
        {activeCategory === 'STORE' && (
          <ProtectedGate permission={Permission.SETTINGS_MANAGE}>
            <StoreSettings />
          </ProtectedGate>
        )}
        {activeCategory === 'SYSTEM' && (
          <ProtectedGate permission={Permission.SETTINGS_MANAGE}>
            <SystemSettings />
          </ProtectedGate>
        )}
      </div>
    </div>
  );
});

export const SettingsScreen: React.FC<SettingsScreenProps> = React.memo(({ onBack }) => {
  return (
    <div className="flex h-full w-full bg-gray-100 overflow-hidden font-sans">
      <SettingsSidebar onBack={onBack} />
      <SettingsContent />
      
      {/* Global Modals */}
      <TableModal />
      <ZoneModal />
      <CategoryModal />
      <TagModal />
      <ProductModal />
    </div>
  );
});