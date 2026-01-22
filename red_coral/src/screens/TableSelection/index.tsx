import React, { useState, useEffect, useMemo } from 'react';
import { LayoutGrid, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';

const api = createTauriClient();
import { HeldOrder, Table, Zone } from '@/core/domain/types';
import { TableFilter, TableSelectionScreenProps } from './types';
import { TableCard } from './TableCard';
import { ZoneSidebar } from './ZoneSidebar';
import { TableFilters } from './TableFilters';
import { GuestInputPanel } from './GuestInputPanel';
import { TableManagementModal } from './components';
import { useDataVersion } from '@/core/stores/settings/useSettingsStore';
import { useCheckoutStore } from '@/core/stores/order/useCheckoutStore';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';

export const TableSelectionScreen: React.FC<TableSelectionScreenProps> = React.memo(
  ({ heldOrders, onSelectTable, onClose, mode, cart = [], manageTableId }) => {
    const { t } = useI18n();
    const dataVersion = useDataVersion();
    const [zones, setZones] = useState<Zone[]>([]);
    const [activeZoneId, setActiveZoneId] = useState<string>('');
    const [activeFilter, setActiveFilter] = useState<TableFilter>('ALL');
    const [zoneTables, setZoneTables] = useState<Table[]>([]);
    const [loading, setLoading] = useState(false);

    const [selectedTableForInput, setSelectedTableForInput] = useState<Table | null>(null);
    const [guestInput, setGuestInput] = useState<string>('');

    const [showManagementModal, setShowManagementModal] = useState(false);
    const isManageOnly = !!manageTableId;
    
    // Track processed manageTableId to prevent re-opening or loops
    const processedManageIdRef = React.useRef<string | null>(null);

    // Get order for a table by table_id
    const getOrderByTable = (tableId: string) => heldOrders.find((o) => o.table_id === tableId);

    // Auto-open management modal if manageTableId is provided
    useEffect(() => {
      if (!manageTableId) {
        processedManageIdRef.current = null;
        return;
      }

      // In管理模式下直接显示管理弹窗
      setShowManagementModal(true);

      if (manageTableId !== processedManageIdRef.current) {
        const order = getOrderByTable(manageTableId);
        if (order && order.zone_name) {
          const targetZone = zones.find((z) => z.name === order.zone_name);
          if (targetZone && targetZone.id !== activeZoneId) {
            setActiveZoneId(targetZone.id);
          }
        }
        processedManageIdRef.current = manageTableId;
      }
    }, [manageTableId, zones, activeZoneId, heldOrders]);

    // Check if overtime (2 hours)
    const isOvertime = (order?: HeldOrder) => {
      if (!order) return false;
      return Date.now() - order.start_time > 2 * 60 * 60 * 1000;
    };

    // Load zones on mount
    useEffect(() => {
      const init = async () => {
        try {
          const zs = await api.listZones();
          setZones(zs);
          setActiveZoneId((prev) => prev || 'ALL');
          // If initializing to ALL, fetch all tables
          if (!activeZoneId) {
            const tables = await api.listTables();
            setZoneTables(tables);
          }
        } catch {}
      };
      init();
    }, [dataVersion]);

    // Load tables when zone changes
    useEffect(() => {
      const loadTables = async () => {
        if (!activeZoneId) return;
        setLoading(true);
        try {
          const tables = await api.listTables();
          setZoneTables(tables);
        } catch {
        } finally {
          setLoading(false);
        }
      };
      loadTables();
    }, [activeZoneId, dataVersion]);

    // Calculate stats
    const stats = useMemo(() => {
      return {
        ALL: zoneTables.length,
        EMPTY: zoneTables.filter((t) => !getOrderByTable(t.id)).length,
        OCCUPIED: zoneTables.filter((t) => {
          const order = getOrderByTable(t.id);
          return !!order && !order.is_pre_payment;
        }).length,
        OVERTIME: zoneTables.filter((t) => isOvertime(getOrderByTable(t.id))).length,
        PRE_PAYMENT: zoneTables.filter((t) => {
          const order = getOrderByTable(t.id);
          return order && order.is_pre_payment;
        }).length,
      };
    }, [zoneTables, heldOrders]);

    // Filter tables
    const filteredTables = useMemo(() => {
      const filtered = zoneTables.filter((table) => {
        const order = getOrderByTable(table.id);
        const isOccupied = !!order;

        switch (activeFilter) {
          case 'EMPTY':
            return !isOccupied;
          case 'OCCUPIED':
            return isOccupied && !order?.is_pre_payment;
          case 'PRE_PAYMENT':
            return isOccupied && !!order?.is_pre_payment;
          case 'OVERTIME':
            return isOvertime(order);
          default:
            return true;
        }
      });

      // Deduplicate tables by ID to prevent React key errors
      const seen = new Set();
      return filtered.filter(t => {
        if (seen.has(t.id)) return false;
        seen.add(t.id);
        return true;
      });
    }, [zoneTables, activeFilter, heldOrders]);

    // Handle table click
    const handleTableClick = (table: Table, isOccupied: boolean, order?: HeldOrder) => {
      if (mode === 'RETRIEVE' && !isOccupied) return;

      const activeZone = zones.find(z => z.id === table.zone);

      if (isOccupied) {
        if (mode === 'HOLD') {
          setGuestInput(order?.guest_count.toString() || '0');
          setSelectedTableForInput(table);
        } else {
          onSelectTable(table, order?.guest_count || 1, activeZone);
        }
      } else {
        setGuestInput('');
        setSelectedTableForInput(table);
      }
    };

    // Handle confirm
    const handleConfirm = () => {
      if (selectedTableForInput) {
        const isOccupied = !!getOrderByTable(selectedTableForInput.id);
        const count = parseInt(guestInput) || (isOccupied ? 0 : 2);
        const activeZone = zones.find(z => z.id === selectedTableForInput.zone);

        if (count > 0 || isOccupied) {
          onSelectTable(selectedTableForInput, count, activeZone);
          setSelectedTableForInput(null);
        }
      }
    };

    const isTableOccupied =
      selectedTableForInput && !!getOrderByTable(selectedTableForInput.id);

    // Helper to get the table object for management
    const managementTable = selectedTableForInput ||
      (manageTableId ? zoneTables.find((t) => t.id === manageTableId) : null) ||
      null;

    if (isManageOnly) {
      return (
        <>
          {managementTable && (
            <TableManagementModal
              sourceTable={managementTable}
              zones={zones}
              heldOrders={heldOrders}
              onClose={onClose}
              onSuccess={(navigateToTableId) => {
                if (navigateToTableId) {
                  const checkout = useCheckoutStore.getState();
                  const store = useActiveOrdersStore.getState();
                  checkout.setCurrentOrderKey(navigateToTableId);
                  const targetSnapshot = store.getOrderByTable(navigateToTableId);
                  if (targetSnapshot) {
                    checkout.setCheckoutOrder(targetSnapshot);
                  } else {
                    // Wait for event to arrive
                    setTimeout(() => {
                      const snapshot = useActiveOrdersStore.getState().getOrderByTable(navigateToTableId);
                      if (snapshot) useCheckoutStore.getState().setCheckoutOrder(snapshot);
                    }, 50);
                  }
                }
                onClose();
              }}
            />
          )}
        </>
      );
    }

    return (
      <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4 font-sans animate-in fade-in duration-200">
        <div className="bg-gray-100 w-full max-w-2xl h-[550px] rounded-2xl shadow-2xl overflow-hidden flex flex-col relative animate-in zoom-in-95 duration-200">
          <div className="bg-white h-12 border-b border-gray-200 flex items-center justify-between px-4 shrink-0 z-10 gap-4">
            <h1 className="text-base font-bold text-gray-800 flex items-center gap-2 shrink-0">
              <LayoutGrid className="text-[#FF5E5E]" size={18} />
              <span>
                {mode === 'HOLD' ? t('table.selection.title') : t('table.selection.retrieve')}
              </span>
            </h1>

            <button
              onClick={onClose}
              className="p-1.5 bg-gray-100 hover:bg-gray-200 rounded-full text-gray-600 transition-colors"
            >
              <X size={18} />
            </button>
          </div>

          {!selectedTableForInput ? (
            <div className="flex-1 flex overflow-hidden">
              <ZoneSidebar
                zones={zones}
                activeZoneId={activeZoneId}
                onZoneSelect={(zoneId) => {
                  setActiveZoneId(zoneId);
                  setActiveFilter('ALL');
                }}
              />

              <div className="flex-1 flex flex-col overflow-hidden bg-gray-50">
                <TableFilters
                  activeFilter={activeFilter}
                  onFilterChange={setActiveFilter}
                  stats={stats}
                />

                <div className="relative flex-1 overflow-y-auto p-3 custom-scrollbar">
                  {loading && zoneTables.length > 0 && (
                    <div className="absolute inset-0 bg-gray-50/60 z-10 flex items-center justify-center backdrop-blur-[1px]">
                      <div className="w-8 h-8 border-4 border-gray-200 border-t-red-500 rounded-full animate-spin" />
                    </div>
                  )}

                  {loading && zoneTables.length === 0 ? (
                    <div className="text-center text-gray-400 text-sm py-8">
                      {t('common.message.loading')}
                    </div>
                  ) : zoneTables.length === 0 ? (
                    <div className="text-center text-gray-400 text-sm py-8">
                      {t('table.noTables')}
                    </div>
                  ) : (
                    <div className="grid grid-cols-2 md:grid-cols-3 gap-3 pb-10">
                      {filteredTables.map((table) => {
                        const order = getOrderByTable(table.id);
                        return (
                          <TableCard
                            key={table.id}
                            table={table}
                            order={order}
                            mode={mode}
                            onClick={() => handleTableClick(table, !!order, order)}
                          />
                        );
                      })}
                    </div>
                  )}
                </div>
              </div>
            </div>
          ) : (
            <GuestInputPanel
              selectedTable={selectedTableForInput}
              isOccupied={!!isTableOccupied}
              guestInput={guestInput}
              cart={cart}
              onGuestInputChange={setGuestInput}
              onConfirm={handleConfirm}
              onBack={() => setSelectedTableForInput(null)}
              onManage={mode === 'RETRIEVE' ? () => setShowManagementModal(true) : undefined}
            />
          )}

          {showManagementModal && managementTable && (
            <TableManagementModal
              sourceTable={managementTable}
              zones={zones}
              heldOrders={heldOrders}
              onClose={() => setShowManagementModal(false)}
              onSuccess={(navigateToTableId) => {
                setShowManagementModal(false);
                setSelectedTableForInput(null);
                if (navigateToTableId) {
                  const checkout = useCheckoutStore.getState();
                  const store = useActiveOrdersStore.getState();
                  checkout.setCurrentOrderKey(navigateToTableId);
                  const targetSnapshot = store.getOrderByTable(navigateToTableId);
                  if (targetSnapshot) {
                    checkout.setCheckoutOrder(targetSnapshot);
                  } else {
                    // Wait for event to arrive
                    setTimeout(() => {
                      const snapshot = useActiveOrdersStore.getState().getOrderByTable(navigateToTableId);
                      if (snapshot) useCheckoutStore.getState().setCheckoutOrder(snapshot);
                    }, 50);
                  }
                }
              }}
            />
          )}
        </div>
      </div>
    );
  }
);
