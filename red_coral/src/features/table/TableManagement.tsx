import React, { useEffect, useMemo, useState } from 'react';
import { LayoutGrid, Plus, Filter, Users, Search, Map as MapIcon, MapPin, ListChecks } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useSettingsModal, useDataVersion, useSettingsFilters } from '@/core/stores/settings/useSettingsStore';
import { useZoneStore } from '@/features/zone/store';
import { useTableStore } from './store';
import { createTauriClient } from '@/infrastructure/api';
import { getErrorMessage } from '@/utils/error';
import { displayId } from '@/utils/formatting';

const getApi = () => createTauriClient();
import { DataTable, Column } from '@/shared/components/DataTable';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { toast } from '@/presentation/components/Toast';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission } from '@/core/domain/types';
import { useCanManageTables } from '@/hooks/usePermission';

interface ZoneItem {
  id: number;
  name: string;
}

const ZoneList: React.FC = React.memo(() => {
  const { t } = useI18n();
  const canManageZones = useCanManageTables();

  // Use resources store for zones
  const zoneStore = useZoneStore();
  const zones = zoneStore.items;
  const loading = zoneStore.isLoading;

  const { openModal } = useSettingsModal();
  const dataVersion = useDataVersion();
  const [searchQuery, setSearchQuery] = useState('');
  const [confirmDialog, setConfirmDialog] = useState({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => {},
  });

  const [isSelectionMode, setIsSelectionMode] = useState(false);

  useEffect(() => {
    zoneStore.fetchAll();
  }, [dataVersion]);

  const filteredZones = useMemo(() => {
    if (!searchQuery.trim()) return zones;
    const q = searchQuery.toLowerCase();
    return zones.filter(
      (z) => z.name.toLowerCase().includes(q) || String(z.id).includes(q)
    );
  }, [zones, searchQuery]);

  const handleBatchDelete = (items: ZoneItem[]) => {
    setConfirmDialog({
      isOpen: true,
      title: t('settings.batch_delete.confirm_title'),
      description: t('settings.batchDelete.confirmDeleteZones', { count: items.length }) || `确定要删除选中的 ${items.length} 个区域吗？此操作无法撤销。`,
      onConfirm: async () => {
        setConfirmDialog(prev => ({ ...prev, isOpen: false }));
        try {
          const results = await Promise.all(
            items.map(async (item) => {
              try {
                await getApi().deleteZone(item.id);
                return { success: true, id: item.id };
              } catch (e: unknown) {
                return { success: false, id: item.id, error: e };
              }
            })
          );

          const failures = results.filter((r) => !r.success);
          const successCount = results.length - failures.length;

          if (successCount > 0) {
            toast.success(t('settings.batchDelete.zonesSuccess', { count: successCount }) || `成功删除 ${successCount} 个区域`);
            zoneStore.fetchAll();
          }

          if (failures.length > 0) {
            toast.error(getErrorMessage(failures[0].error));
          }
        } catch (e) {
          toast.error(getErrorMessage(e));
        }
      },
    });
  };

  const columns: Column<ZoneItem>[] = useMemo(
    () => [
      {
        key: 'name',
        header: t('settings.zone.form.name'),
        render: (item) => (
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 bg-gradient-to-br from-purple-100 to-purple-200 rounded-lg flex items-center justify-center">
              <MapPin size={16} className="text-purple-600" />
            </div>
            <div>
              <span className="font-medium text-gray-900">{item.name}</span>
              <div className="text-xs text-gray-400 mt-0.5">ID: {displayId(item.id)}</div>
            </div>
          </div>
        ),
      },
    ],
    [t]
  );

  return (
    <div className="space-y-5">
      {/* Filter Bar */}
      <div className="bg-white rounded-xl border border-gray-200 p-4 shadow-sm">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 text-gray-500">
            <Filter size={16} />
            <span className="text-sm font-medium">{t('common.action.filter')}</span>
          </div>
          <div className="h-5 w-px bg-gray-200" />

          <div className="relative flex-1 max-w-xs">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('common.hint.search_placeholder')}
              className="w-full pl-9 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500/20 focus:border-purple-500"
            />
          </div>

          <div className="ml-auto flex items-center gap-3">
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-purple-500" />
              <span className="text-sm text-gray-600">{t('common.label.total')}</span>
              <span className="text-sm font-bold text-gray-900">{filteredZones.length}</span>
              <span className="text-sm text-gray-600">{t('settings.zone.unit')}</span>
            </div>
            {canManageZones && !isSelectionMode && (
              <button
                onClick={() => setIsSelectionMode(true)}
                className="flex items-center gap-1 px-2 py-1 rounded-md text-xs transition-colors border border-transparent text-purple-600 bg-purple-50 hover:bg-purple-100 border-purple-100"
              >
                <ListChecks size={14} />
                <span>{t('common.action.select')}</span>
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Data Table */}
      <DataTable
        data={filteredZones}
        columns={columns}
        loading={loading}
        getRowKey={(item) => String(item.id)}
        onEdit={canManageZones ? (item) => openModal('ZONE', 'EDIT', item) : undefined}
        onDelete={canManageZones ? (item) => openModal('ZONE', 'DELETE', item) : undefined}
        onBatchDelete={canManageZones ? handleBatchDelete : undefined}
        emptyText={t('common.empty.no_data')}
        themeColor="purple"
        isSelectionMode={isSelectionMode}
        onSelectionModeChange={setIsSelectionMode}
      />

      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={() => setConfirmDialog(prev => ({ ...prev, isOpen: false }))}
      />
    </div>
  );
});

interface TableItem {
  id: number;
  name: string;
  zone_id?: number;
  capacity?: number;
}

interface TableManagementProps {
  initialTab?: 'tables' | 'zones';
}

export const TableManagement: React.FC<TableManagementProps> = React.memo(({ initialTab = 'tables' }) => {
  const { t } = useI18n();
  const canManageTables = useCanManageTables();
  const [activeTab, setActiveTab] = useState<'tables' | 'zones'>(initialTab);

  // Use resources stores
  const zoneStore = useZoneStore();
  const tableStore = useTableStore();
  const zones = zoneStore.items;
  const tables = tableStore.items;
  const loading = tableStore.isLoading;

  // UI state from settings store
  const {
    selectedZoneFilter: zoneFilter,
    tablesPage: page,
    tablesTotal: total,
    setSelectedZoneFilter: setZoneFilter,
    setTablesPagination: setPagination,
  } = useSettingsFilters();

  const { openModal } = useSettingsModal();
  const dataVersion = useDataVersion();
  const [searchQuery, setSearchQuery] = useState('');
  const [confirmDialog, setConfirmDialog] = useState({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => {},
  });

  const [isTableSelectionMode, setIsTableSelectionMode] = useState(false);

  useEffect(() => {
    if (activeTab === 'tables') {
      zoneStore.fetchAll();
      tableStore.fetchAll().then(() => {
        setPagination(1, tableStore.items.length);
      });
    }
  }, [zoneFilter, page, searchQuery, dataVersion, activeTab]);

  const zonesMap = useMemo(() => {
    const m = new Map<number, string>();
    zones.forEach((z) => m.set(z.id, z.name));
    return m;
  }, [zones]);

  // Filter tables by zone
  const filteredTables = useMemo(() => {
    if (zoneFilter === 'all') return tables;
    return tables.filter((t) => String(t.zone_id) === zoneFilter);
  }, [tables, zoneFilter]);

  const handleBatchDelete = (items: TableItem[]) => {
    setConfirmDialog({
      isOpen: true,
      title: t('settings.batch_delete.confirm_title'),
      description: t('settings.batchDelete.confirmDeleteTables', { count: items.length }) || `确定要删除选中的 ${items.length} 个桌台吗？此操作无法撤销。`,
      onConfirm: async () => {
        setConfirmDialog((prev) => ({ ...prev, isOpen: false }));
        try {
          await Promise.all(items.map((item) => getApi().deleteTable(item.id)));
          toast.success(t('settings.batchDelete.tablesSuccess', { count: items.length }) || '批量删除成功');
          await tableStore.fetchAll();
          setPagination(1, tableStore.items.length);
        } catch (e) {
          toast.error(getErrorMessage(e));
        }
      },
    });
  };

  const columns: Column<TableItem>[] = useMemo(
    () => [
      {
        key: 'name',
        header: t('settings.table.form.name'),
        render: (item) => (
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 bg-blue-100 rounded-lg flex items-center justify-center">
              <LayoutGrid size={14} className="text-blue-600" />
            </div>
            <span className="font-medium text-gray-900">{item.name}</span>
          </div>
        ),
      },
      {
        key: 'capacity',
        header: t('settings.table.form.capacity'),
        width: '150px',
        align: 'center',
        render: (item) => (
          <div className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-emerald-50 text-emerald-700 rounded-full text-xs font-medium whitespace-nowrap">
            <Users size={12} />
            <span>{item.capacity} {t('common.unit.person')}</span>
          </div>
        ),
      },
      {
        key: 'zone',
        header: t('table.zones'),
        render: (item) => (
          <span className="inline-flex items-center px-2.5 py-1 bg-purple-50 text-purple-700 rounded-full text-xs font-medium">
            {zonesMap.get(item.zone_id ?? 0) || item.zone_id}
          </span>
        ),
      },
    ],
    [t, zonesMap]
  );

  return (
    <div className="space-y-5">
      {/* Header Card */}
      <div className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${activeTab === 'tables' ? 'bg-blue-100' : 'bg-purple-100'}`}>
              {activeTab === 'tables' ? (
                <LayoutGrid size={20} className="text-blue-600" />
              ) : (
                <MapIcon size={20} className="text-purple-600" />
              )}
            </div>
            <div>
              <h2 className="text-lg font-bold text-gray-900">
                {t('settings.table.zone_management')}
              </h2>
              <p className="text-sm text-gray-500 mt-1">
                {t('settings.table.zone_management_desc')}
              </p>
            </div>
          </div>
          {activeTab === 'tables' ? (
            <ProtectedGate permission={Permission.TABLES_MANAGE}>
              <button
                onClick={() => openModal('TABLE', 'CREATE', { defaultZoneId: zones[0]?.id })}
                className="inline-flex items-center gap-2 px-4 py-2.5 bg-blue-600 text-white rounded-xl text-sm font-semibold shadow-lg shadow-blue-600/20 hover:bg-blue-700 hover:shadow-blue-600/30 transition-all"
              >
                <Plus size={16} />
                <span>{t('settings.table.add_table')}</span>
              </button>
            </ProtectedGate>
          ) : (
            <ProtectedGate permission={Permission.TABLES_MANAGE}>
              <button
                onClick={() => openModal('ZONE', 'CREATE')}
                className="inline-flex items-center gap-2 px-4 py-2.5 bg-purple-600 text-white rounded-xl text-sm font-semibold shadow-lg shadow-purple-600/20 hover:bg-purple-700 hover:shadow-purple-600/30 transition-all"
              >
                <Plus size={16} />
                <span>{t('settings.zone.add_zone')}</span>
              </button>
            </ProtectedGate>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex space-x-1 bg-gray-100 p-1 rounded-xl w-fit">
        <button
          onClick={() => setActiveTab('tables')}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === 'tables'
              ? 'bg-white text-blue-600 shadow-sm'
              : 'text-gray-600 hover:text-gray-900 hover:bg-gray-200/50'
          }`}
        >
          <LayoutGrid size={16} />
          {t('settings.table.title')}
        </button>
        <button
          onClick={() => setActiveTab('zones')}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === 'zones'
              ? 'bg-white text-purple-600 shadow-sm'
              : 'text-gray-600 hover:text-gray-900 hover:bg-gray-200/50'
          }`}
        >
          <MapIcon size={16} />
          {t('settings.zone.title')}
        </button>
      </div>

      {activeTab === 'tables' ? (
        <>
          {/* Filter Bar */}
          <div className="bg-white rounded-xl border border-gray-200 p-4 shadow-sm">
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2 text-gray-500">
                <Filter size={16} />
                <span className="text-sm font-medium">{t('common.action.filter')}</span>
              </div>
              <div className="h-5 w-px bg-gray-200" />
              <div className="flex items-center gap-2">
                <label className="text-sm text-gray-600">{t('table.zones')}:</label>
                <select
                  value={zoneFilter}
                  onChange={(e) => setZoneFilter(e.target.value)}
                  className="border border-gray-200 rounded-lg px-3 py-1.5 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-colors min-w-[8.75rem]"
                >
                  <option value="all">{t('table.filter.all')}</option>
                  {zones.map((z) => (
                    <option key={z.id} value={z.id}>
                      {z.name}
                    </option>
                  ))}
                </select>
              </div>

              <div className="h-5 w-px bg-gray-200 ml-2" />
              <div className="relative flex-1 max-w-xs">
                <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => {
                    setSearchQuery(e.target.value);
                    setPagination(1, total);
                  }}
                  placeholder={t('common.hint.search_placeholder')}
                  className="w-full pl-9 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                />
              </div>

              <div className="ml-auto flex items-center gap-3">
                <span className="text-xs text-gray-400">
                  {t('common.label.total')} {filteredTables.length} {t('common.label.items')}
                </span>
                {canManageTables && !isTableSelectionMode && (
                  <button
                    onClick={() => setIsTableSelectionMode(true)}
                    className="flex items-center gap-1 px-2 py-1 rounded-md text-xs transition-colors border border-transparent text-blue-600 bg-blue-50 hover:bg-blue-100 border-blue-100"
                  >
                    <ListChecks size={14} />
                    <span>{t('common.action.select')}</span>
                  </button>
                )}
              </div>
            </div>
          </div>

          {/* Data Table */}
          <DataTable
            data={filteredTables}
            columns={columns}
            loading={loading}
            getRowKey={(item) => String(item.id)}
            onEdit={canManageTables ? (item) => openModal('TABLE', 'EDIT', item) : undefined}
            onDelete={canManageTables ? (item) => openModal('TABLE', 'DELETE', item) : undefined}
            onBatchDelete={canManageTables ? handleBatchDelete : undefined}
            emptyText={t('common.empty.no_data')}
            pageSize={5}
            totalItems={filteredTables.length}
            currentPage={page}
            onPageChange={(newPage) => setPagination(newPage, filteredTables.length)}
            themeColor="blue"
            isSelectionMode={isTableSelectionMode}
            onSelectionModeChange={setIsTableSelectionMode}
          />
        </>
      ) : (
        <ZoneList />
      )}

      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={() => setConfirmDialog(prev => ({ ...prev, isOpen: false }))}
      />
    </div>
  );
});
