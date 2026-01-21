import React, { useState, useEffect, useMemo } from 'react';
import { LayoutGrid, X, AlertCircle } from 'lucide-react';
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
import { toHeldOrder } from '@/core/stores/order/orderAdapter';

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
    const [enableIndividualMode, setEnableIndividualMode] = useState(false);
    
    const [showManagementModal, setShowManagementModal] = useState(false);
    const isManageOnly = !!manageTableId;
    
    // Track processed manageTableId to prevent re-opening or loops
    const processedManageIdRef = React.useRef<string | null>(null);

    // Get order for a table
    const getOrder = (tableId: string) => heldOrders.find((o) => o.key === tableId);

    // Auto-open management modal if manageTableId is provided
    useEffect(() => {
      if (!manageTableId) {
        processedManageIdRef.current = null;
        return;
      }

      // In管理模式下直接显示管理弹窗
      setShowManagementModal(true);

      if (manageTableId !== processedManageIdRef.current) {
        const order = getOrder(manageTableId);
        if (order && order.zoneName) {
          const targetZone = zones.find((z) => z.name === order.zoneName);
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
      return Date.now() - order.startTime > 2 * 60 * 60 * 1000;
    };

    // Load zones on mount
    useEffect(() => {
      const init = async () => {
        try {
          const zsResp = await api.listZones();
          const zs = zsResp.data?.zones || [];
          setZones(zs);
          setActiveZoneId((prev) => prev || 'ALL');
          // If initializing to ALL, fetch all tables
          if (!activeZoneId) {
            const tsResp = await api.listTables();
            const tables = tsResp.data?.tables || [];
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
          const tsResp = await api.listTables();
          const tables = tsResp.data?.tables || [];
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
        EMPTY: zoneTables.filter((t) => !getOrder(t.id)).length,
        OCCUPIED: zoneTables.filter((t) => {
          const order = getOrder(t.id);
          return !!order && !order.isPrePayment;
        }).length,
        OVERTIME: zoneTables.filter((t) => isOvertime(getOrder(t.id))).length,
        PRE_PAYMENT: zoneTables.filter((t) => {
          const order = getOrder(t.id);
          return order && order.isPrePayment;
        }).length,
      };
    }, [zoneTables, heldOrders]);

    // Filter tables
    const filteredTables = useMemo(() => {
      const filtered = zoneTables.filter((table) => {
        const order = getOrder(table.id);
        const isOccupied = !!order;

        switch (activeFilter) {
          case 'EMPTY':
            return !isOccupied;
          case 'OCCUPIED':
            return isOccupied && !order?.isPrePayment;
          case 'PRE_PAYMENT':
            return isOccupied && !!order?.isPrePayment;
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

    // Identify Ghost Orders (Active orders whose tables have been deleted)
    const ghostOrders = useMemo(() => {
      // Only check for ghosts when we have the full table list (ALL zones)
      // Otherwise we can't be sure if it's a ghost or just in another zone
      if (activeZoneId !== 'ALL' || loading) return [];
      
      const tableIds = new Set(zoneTables.map(t => t.id));
      
      return heldOrders.filter(o =>
        !tableIds.has(o.key) &&
        !o.isRetail
      );
    }, [activeZoneId, loading, zoneTables, heldOrders]);

    // Handle table click
    const handleTableClick = (table: Table, isOccupied: boolean, order?: HeldOrder) => {
      if (mode === 'RETRIEVE' && !isOccupied) return;

      const activeZone = zones.find(z => z.id === table.zone);

      if (isOccupied) {
        if (mode === 'HOLD') {
          setGuestInput(order?.guestCount.toString() || '0');
          setEnableIndividualMode(false);
          setSelectedTableForInput(table);
        } else {
          onSelectTable(table, order?.guestCount || 1, undefined, activeZone);
        }
      } else {
        setGuestInput('');
        setEnableIndividualMode(false);
        setSelectedTableForInput(table);
      }
    };

    // Handle confirm
    const handleConfirm = () => {
      if (selectedTableForInput) {
        const isOccupied = !!getOrder(selectedTableForInput.id);
        const count = parseInt(guestInput) || (isOccupied ? 0 : 2);
        const activeZone = zones.find(z => z.id === selectedTableForInput.zone);

        if (count > 0 || isOccupied) {
          onSelectTable(selectedTableForInput, count, enableIndividualMode, activeZone);
          setSelectedTableForInput(null);
        }
      }
    };

    // Toggle individual mode (deprecated but kept for backward compatibility)
    const toggleIndividualMode = () => {
      // Individual mode has been removed in refactoring
      // Keep this as no-op to prevent errors
      setEnableIndividualMode(false);
    };

    const isTableOccupied =
      selectedTableForInput && !!getOrder(selectedTableForInput.id);

    // Helper to get the table object for management
    const managementTable = selectedTableForInput || (manageTableId ? ((): Table | null => {
      const found = zoneTables.find((t) => t.id === manageTableId);
      if (found) return found;
      const order = heldOrders.find((o) => o.key === manageTableId);
      if (!order) return null;
      const zone = zones.find((z) => z.name === order.zoneName);
      return {
        id: manageTableId,
        name: order.tableName || '',
        zone: zone ? zone.id : '',
        capacity: 0,
        is_active: true,
      } as Table;
    })() : null);

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
                  const targetSnapshot = store.getOrder(navigateToTableId);
                  if (targetSnapshot) {
                    checkout.setCheckoutOrder(toHeldOrder(targetSnapshot));
                  } else {
                    // Wait for event to arrive
                    setTimeout(() => {
                      const snapshot = useActiveOrdersStore.getState().getOrder(navigateToTableId);
                      if (snapshot) useCheckoutStore.getState().setCheckoutOrder(toHeldOrder(snapshot));
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
                  {/* Ghost Orders Section */}
                  {ghostOrders.length > 0 && (
                    <div className="mb-6 mx-3">
                        <div className="flex items-center gap-2 mb-3 text-amber-600 bg-amber-50 p-2 rounded-lg border border-amber-100">
                            <AlertCircle size={16} />
                            <span className="text-sm font-medium">{t('table.ghostOrders')}</span>
                        </div>
                        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
                            {ghostOrders.map(order => (
                                <TableCard
                                    key={order.key}
                                    table={{
                                        id: order.key,
                                        name: order.tableName || order.key,
                                        zone: 'GHOST',
                                        capacity: 0,
                                        is_active: true,
                                    }}
                                    order={order}
                                    mode={mode}
                                    onClick={() => handleTableClick({
                                        id: order.key,
                                        name: order.tableName || order.key,
                                        zone: 'GHOST',
                                        capacity: 0,
                                        is_active: true,
                                    }, true, order)}
                                    className="border-amber-300 bg-amber-50"
                                />
                            ))}
                        </div>
                    </div>
                  )}

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
                        const order = getOrder(table.id);
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
              enableIndividualMode={enableIndividualMode}
              cart={cart}
              onGuestInputChange={setGuestInput}
              onIndividualModeToggle={toggleIndividualMode}
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
                  const targetSnapshot = store.getOrder(navigateToTableId);
                  if (targetSnapshot) {
                    checkout.setCheckoutOrder(toHeldOrder(targetSnapshot));
                  } else {
                    // Wait for event to arrive
                    setTimeout(() => {
                      const snapshot = useActiveOrdersStore.getState().getOrder(navigateToTableId);
                      if (snapshot) useCheckoutStore.getState().setCheckoutOrder(toHeldOrder(snapshot));
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
