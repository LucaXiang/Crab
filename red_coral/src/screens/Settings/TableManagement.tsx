import React, { useEffect, useMemo, useState } from 'react';
import { LayoutGrid, Plus, Filter, Users, Search, Map as MapIcon, MapPin } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useSettingsTables,
  useSettingsZones,
  useSettingsModal,
  useDataVersion,
} from '@/core/stores/settings/useSettingsStore';
import { createClient } from '@/infrastructure/api';

const api = createClient();
import { DataTable, Column } from '@/presentation/components/ui/DataTable';
import { ConfirmDialog } from '@/presentation/components/ui/ConfirmDialog';
import { toast } from '@/presentation/components/Toast';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission } from '@/core/domain/types';
import { useCanManageTables, useCanManageZones } from '@/hooks/usePermission';

interface ZoneItem {
  id: string;
  name: string;
}

const ZoneList: React.FC = React.memo(() => {
  const { t } = useI18n();

  // Permission check
  const canManageZones = useCanManageZones();

  const { zones, loading, setZones, setLoading } = useSettingsZones();
  const { openModal } = useSettingsModal();
  const dataVersion = useDataVersion();
  const [searchQuery, setSearchQuery] = useState('');
  const [confirmDialog, setConfirmDialog] = useState({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => {},
  });

  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      try {
        const zsResp = await api.listZones();
        const zs = zsResp.data?.zones || [];
        setZones(zs);
      } finally {
        setLoading(false);
      }
    };
    loadData();
  }, [dataVersion]);

  const filteredZones = useMemo(() => {
    if (!searchQuery.trim()) return zones;
    const q = searchQuery.toLowerCase();
    return zones.filter(
      (z) => z.name.toLowerCase().includes(q) || z.id.toLowerCase().includes(q)
    );
  }, [zones, searchQuery]);

  const handleBatchDelete = (items: ZoneItem[]) => {
    setConfirmDialog({
      isOpen: true,
      title: t('settings.batchDelete.confirmTitle'),
      description: t('settings.batchDelete.confirmDeleteZones', { count: items.length }) || `确定要删除选中的 ${items.length} 个区域吗？此操作无法撤销。`,
      onConfirm: async () => {
        setConfirmDialog(prev => ({ ...prev, isOpen: false }));
        setLoading(true);
        try {
          const results = await Promise.all(
            items.map(async (item) => {
              try {
                await api.deleteZone(Number(item.id));
                return { success: true, id: item.id };
              } catch (e: any) {
                return { success: false, id: item.id, error: e };
              }
            })
          );

          const failures = results.filter((r) => !r.success);
          const successCount = results.length - failures.length;

          if (successCount > 0) {
            toast.success(t('settings.batchDelete.zonesSuccess', { count: successCount }) || `成功删除 ${successCount} 个区域`);
            const zsResp = await api.listZones();
        const zs = zsResp.data?.zones || [];
            setZones(zs);
          }

          if (failures.length > 0) {
            const hasTableError = failures.some((f) => String(f.error).includes('ZONE_HAS_TABLES'));
            if (hasTableError) {
              toast.error(t('settings.batchDelete.zonesBlocked'));
            } else {
              toast.error(t('settings.batchDelete.zonesFailed'));
            }
          }
        } catch (e) {
          console.error(e);
          toast.error(t('settings.batchDelete.zonesFailed'));
        } finally {
          setLoading(false);
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
              <div className="text-xs text-gray-400 mt-0.5">ID: {item.id.slice(0, 8)}</div>
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
            <span className="text-sm font-medium">{t('common.filter')}</span>
          </div>
          <div className="h-5 w-px bg-gray-200" />
          
          <div className="relative flex-1 max-w-xs">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('common.searchPlaceholder')}
              className="w-full pl-9 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500/20 focus:border-purple-500"
            />
          </div>

          <div className="ml-auto flex items-center gap-4">
             <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-purple-500" />
                <span className="text-sm text-gray-600">{t('common.total')}</span>
                <span className="text-sm font-bold text-gray-900">{filteredZones.length}</span>
                <span className="text-sm text-gray-600">{t('settings.zone.unit')}</span>
             </div>
          </div>
        </div>
      </div>

      {/* Data Table */}
      <DataTable
        data={filteredZones}
        columns={columns}
        loading={loading}
        getRowKey={(item) => item.id}
        onEdit={canManageZones ? (item) => openModal('ZONE', 'EDIT', item) : undefined}
        onDelete={canManageZones ? (item) => openModal('ZONE', 'DELETE', item) : undefined}
        onBatchDelete={canManageZones ? handleBatchDelete : undefined}
        emptyText={t('settings.zone.noData')}
        themeColor="purple"
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
  id: string;
  name: string;
  zoneId: string;
  capacity: number;
}

interface TableManagementProps {
  initialTab?: 'tables' | 'zones';
}

export const TableManagement: React.FC<TableManagementProps> = React.memo(({ initialTab = 'tables' }) => {
  const { t } = useI18n();

  // Permission check
  const canManageTables = useCanManageTables();

  const [activeTab, setActiveTab] = useState<'tables' | 'zones'>(initialTab);

  const { tables, loading, zoneFilter, page, total, setTables, setLoading, setZoneFilter, setPagination } =
    useSettingsTables();
  const { zones, setZones } = useSettingsZones();
  const { openModal } = useSettingsModal();
  const dataVersion = useDataVersion();
  const [searchQuery, setSearchQuery] = useState('');
  const [confirmDialog, setConfirmDialog] = useState({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => {},
  });

  useEffect(() => {
    if (activeTab === 'tables') {
      const loadData = async () => {
        setLoading(true);
        try {
          const zsResp = await api.listZones();
          const zs = zsResp.data?.zones || [];
          setZones(zs);
          const tsResp = await api.listTables();
          const ts = tsResp.data?.tables || [];
          setTables(ts);
          setPagination(1, ts.length);
        } finally {
          setLoading(false);
        }
      };
      // Debounce search could be added here
      const timer = setTimeout(loadData, 300);
      return () => clearTimeout(timer);
    }
    return () => {};
  }, [zoneFilter, page, searchQuery, dataVersion, activeTab]);

  const zonesMap = useMemo(() => {
    const m = new Map<string, string>();
    zones.forEach((z) => m.set(z.id, z.name));
    return m;
  }, [zones]);
  
  // Client-side filtering removed in favor of server-side search
  const filteredTables = tables;

  const handleBatchDelete = (items: TableItem[]) => {
    setConfirmDialog({
      isOpen: true,
      title: t('settings.batchDelete.confirmTitle'),
      description: t('settings.batchDelete.confirmDeleteTables', { count: items.length }) || `确定要删除选中的 ${items.length} 个桌台吗？此操作无法撤销。`,
      onConfirm: async () => {
        setConfirmDialog((prev) => ({ ...prev, isOpen: false }));
        setLoading(true);
        try {
          await Promise.all(items.map((item) => api.deleteTable(Number(item.id))));
          toast.success(t('settings.batchDelete.tablesSuccess', { count: items.length }) || '批量删除成功');
          const tsResp = await api.listTables();
          const ts = tsResp.data?.tables || [];
          setTables(ts);
          setPagination(1, ts.length);
        } catch (e) {
          console.error(e);
          toast.error(t('settings.batchDelete.failed'));
        } finally {
          setLoading(false);
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
        width: '120px',
        align: 'center',
        render: (item) => (
          <div className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-emerald-50 text-emerald-700 rounded-full text-xs font-medium">
            <Users size={12} />
            <span>{item.capacity} 人</span>
          </div>
        ),
      },
      {
        key: 'zone',
        header: t('table.common.zones'),
        width: '140px',
        render: (item) => (
          <span className="inline-flex items-center px-2.5 py-1 bg-purple-50 text-purple-700 rounded-full text-xs font-medium">
            {zonesMap.get(item.zoneId) || item.zoneId}
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
                {t('settings.table.zoneManagement')}
              </h2>
              <p className="text-sm text-gray-500 mt-1">
                {t('settings.table.zoneManagementDesc')}
              </p>
            </div>
          </div>
          {activeTab === 'tables' ? (
            <ProtectedGate permission={Permission.MANAGE_TABLES}>
              <button
                onClick={() => openModal('TABLE', 'CREATE')}
                className="inline-flex items-center gap-2 px-4 py-2.5 bg-blue-600 text-white rounded-xl text-sm font-semibold shadow-lg shadow-blue-600/20 hover:bg-blue-700 hover:shadow-blue-600/30 transition-all"
              >
                <Plus size={16} />
                <span>{t('settings.table.action.add')}</span>
              </button>
            </ProtectedGate>
          ) : (
            <ProtectedGate permission={Permission.MANAGE_ZONES}>
              <button
                onClick={() => openModal('ZONE', 'CREATE')}
                className="inline-flex items-center gap-2 px-4 py-2.5 bg-purple-600 text-white rounded-xl text-sm font-semibold shadow-lg shadow-purple-600/20 hover:bg-purple-700 hover:shadow-purple-600/30 transition-all"
              >
                <Plus size={16} />
                <span>{t('settings.zone.action.add')}</span>
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
                <span className="text-sm font-medium">{t('common.filter')}</span>
              </div>
              <div className="h-5 w-px bg-gray-200" />
              <div className="flex items-center gap-2">
                <label className="text-sm text-gray-600">{t('table.common.zones')}:</label>
                <select
                  value={zoneFilter}
                  onChange={(e) => setZoneFilter(e.target.value as any)}
                  className="border border-gray-200 rounded-lg px-3 py-1.5 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-colors min-w-[140px]"
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
                  placeholder={t('common.searchPlaceholder')}
                  className="w-full pl-9 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                />
              </div>

              <div className="ml-auto text-xs text-gray-400">
                {t('common.total')} {total} {t('common.items')}
              </div>
            </div>
          </div>

          {/* Data Table */}
          <DataTable
            data={filteredTables}
            columns={columns}
            loading={loading}
            getRowKey={(item) => item.id}
            onEdit={canManageTables ? (item) => openModal('TABLE', 'EDIT', item) : undefined}
            onDelete={canManageTables ? (item) => openModal('TABLE', 'DELETE', item) : undefined}
            onBatchDelete={canManageTables ? handleBatchDelete : undefined}
            emptyText={t('settings.table.noData')}
            pageSize={5}
            totalItems={total}
            currentPage={page}
            onPageChange={(newPage) => setPagination(newPage, total)}
            themeColor="blue"
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
