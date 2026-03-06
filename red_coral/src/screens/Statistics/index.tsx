import React, { useState } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { Sidebar } from './components/Sidebar';
import { OverviewTab } from './components/OverviewTab';
import { InvoiceList } from './components/InvoiceList';
import { ReportsAndShifts } from './components/ReportsAndShifts';
import { RedFlagsTab } from './components/RedFlagsTab';
import { AuditLog } from './components/AuditLog';
import type { ActiveTab } from '@/core/domain/types';

interface StatisticsScreenProps {
  isVisible: boolean;
  onBack: () => void;
}

export const StatisticsScreen: React.FC<StatisticsScreenProps> = ({ isVisible, onBack }) => {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = useState<ActiveTab>('overview');

  if (!isVisible) return null;

  return (
    <div className="flex h-full w-full bg-gray-50 overflow-hidden font-sans">
      <Sidebar
        onBack={onBack}
        activeTab={activeTab}
        onTabChange={setActiveTab}
      />

      <div className="flex-1 overflow-y-auto p-8 min-w-0" style={{ scrollbarGutter: 'stable' }}>
        <div className="max-w-7xl mx-auto">
          <h1 className="text-2xl font-bold text-gray-800 mb-6">
            {activeTab === 'overview' && t('statistics.sidebar.overview')}
            {activeTab === 'invoices' && t('statistics.sidebar.invoices')}
            {activeTab === 'reports_shifts' && t('statistics.sidebar.reports_shifts')}
            {activeTab === 'red_flags' && t('statistics.sidebar.red_flags')}
            {activeTab === 'audit_log' && t('statistics.sidebar.audit_log')}
          </h1>

          {activeTab === 'overview' && <OverviewTab />}
          {activeTab === 'invoices' && <InvoiceList />}
          {activeTab === 'reports_shifts' && <ReportsAndShifts />}
          {activeTab === 'red_flags' && <RedFlagsTab />}
          {activeTab === 'audit_log' && <AuditLog />}
        </div>
      </div>
    </div>
  );
};
