import React, { useState, useMemo, useEffect } from 'react';
import { X, ArrowLeftRight, Percent, Users, Check, ArrowLeft, LayoutGrid, Split, Minus, Plus, CreditCard, Banknote } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';

const api = createTauriClient();
import { Table, Zone, HeldOrder, Permission } from '@/core/domain/types';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { toHeldOrder } from '@/core/stores/order/orderAdapter';
import * as orderOps from '@/core/stores/order/useOrderOperations';
import { useCheckoutStore } from '@/core/stores/order/useCheckoutStore';
import { ZoneSidebar } from '../ZoneSidebar';
import { TableCard } from '../TableCard';
import { toast } from '@/presentation/components/Toast';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';

interface TableManagementModalProps {
    sourceTable: Table;
    zones: Zone[];
    heldOrders: HeldOrder[];
    onClose: () => void;
    onSuccess: (navigateToTableId?: string) => void;
}

type ManagementMode = 'MENU' | 'MERGE' | 'MOVE' | 'SPLIT';

export const TableManagementModal: React.FC<TableManagementModalProps> = ({
    sourceTable,
    zones,
    heldOrders,
    onClose,
    onSuccess,
}) => {
    const { t } = useI18n();
    const [mode, setMode] = useState<ManagementMode>('MENU');

    const [activeZoneId, setActiveZoneId] = useState<string>(zones[0]?.id || '');
    const [zoneTables, setZoneTables] = useState<Table[]>([]);
    const [selectedTargetTable, setSelectedTargetTable] = useState<Table | null>(null);

    // Split Bill State
    const [splitItems, setSplitItems] = useState<Record<string, number>>({});
    const [isProcessingSplit, setIsProcessingSplit] = useState(false);

    useEffect(() => {
        const loadTables = async () => {
            if (!activeZoneId) return;
            try {
                const resp = await api.listTables();
                const tables = resp.data?.tables || [];
                setZoneTables(tables);
            } catch { }
        };
        loadTables();
    }, [activeZoneId]);

    // Ensure a valid initial zone when zones prop arrives or source order indicates a zone
    useEffect(() => {
        if (!zones || zones.length === 0) return;
        if (activeZoneId) return;
        const sourceOrder = heldOrders.find(o => o.key === sourceTable.id);
        const preferZone = sourceOrder?.zoneName ? zones.find(z => z.name === sourceOrder.zoneName) : undefined;
        const nextZoneId = preferZone?.id || zones[0].id;
        if (nextZoneId) setActiveZoneId(nextZoneId);
    }, [zones, heldOrders, sourceTable.id, activeZoneId]);

    // Get source order from active store or fallback to prop
    const sourceOrderSnapshot = useActiveOrdersStore(state => state.orders[sourceTable.id as string]);
    const sourceOrder = sourceOrderSnapshot ? toHeldOrder(sourceOrderSnapshot) : heldOrders.find(o => o.key === sourceTable.id);

    const isSurchargeExempt = sourceOrder?.surchargeExempt || false;

    const hasPayments = useMemo(() => {
        if (!sourceOrder) return false;
        const hasPaidAmount = (sourceOrder.paidAmount || 0) > 0;
        const hasPaidItems = sourceOrder.paidItemQuantities && Object.keys(sourceOrder.paidItemQuantities).length > 0;
        return hasPaidAmount || hasPaidItems;
    }, [sourceOrder]);

    const hasSurcharge = useMemo(() => {
        if (!sourceOrder) return false;
        const orderSurcharge = sourceOrder.surcharge?.amount || 0;
        const itemsSurcharge = sourceOrder.items?.reduce((sum, item) => sum + (item.surcharge || 0), 0) || 0;
        return orderSurcharge > 0 || itemsSurcharge > 0;
    }, [sourceOrder]);

    const handleWaiveSurcharge = async () => {
        if (sourceOrder && sourceTable.id) {
            try {
                await orderOps.setSurchargeExempt(sourceOrder, !isSurchargeExempt);

                // Sync with checkout store after command completes
                const checkoutStore = useCheckoutStore.getState();
                if (checkoutStore.currentOrderKey === sourceTable.id) {
                    const store = useActiveOrdersStore.getState();
                    const updatedSnapshot = store.getOrder(sourceTable.id as string);
                    if (updatedSnapshot) {
                        checkoutStore.setCheckoutOrder(toHeldOrder(updatedSnapshot));
                    }
                }
            } catch (err) {
                console.error('Failed to set surcharge exempt:', err);
            }
        }
    };

    const handleMerge = async () => {
        if (!selectedTargetTable || !sourceOrder) return;
        const store = useActiveOrdersStore.getState();
        const targetSnapshot = store.getOrder(selectedTargetTable.id as string);
        if (!targetSnapshot) return;
        const targetOrder = toHeldOrder(targetSnapshot);

        try {
            const mergedOrder = await orderOps.mergeOrders(sourceOrder, targetOrder);
            const checkout = useCheckoutStore.getState();
            checkout.setCurrentOrderKey(selectedTargetTable.id as string);
            checkout.setCheckoutOrder(mergedOrder);
            onSuccess(selectedTargetTable.id as string);
        } catch (err) {
            console.error('Merge failed:', err);
            toast.error(t('checkout.error.mergeFailed'));
        }
    };

    const handleMove = async () => {
        if (!selectedTargetTable || !sourceOrder) return;
        const targetZone = zones.find(z => z.id === selectedTargetTable.zone);

        try {
            const movedOrder = await orderOps.moveOrder(
                sourceOrder,
                selectedTargetTable.id as string,
                selectedTargetTable.name,
                targetZone?.name
            );
            const checkout = useCheckoutStore.getState();
            checkout.setCurrentOrderKey(selectedTargetTable.id as string);
            checkout.setCheckoutOrder(movedOrder);
            onSuccess(selectedTargetTable.id as string);
        } catch (err) {
            console.error('Move failed:', err);
            toast.error(t('checkout.error.moveFailed'));
        }
    };

    const handleSplitPayment = async (method: 'CASH' | 'CARD') => {
        if (!sourceOrder || isProcessingSplit) return;

        const itemsToSplit = (Object.entries(splitItems) as [string, number][])
            .filter(([_, qty]) => qty > 0)
            .map(([instanceId, qty]) => {
                const originalItem = sourceOrder.items.find(i => i.id === instanceId);
                return {
                    instanceId,
                    quantity: qty,
                    name: originalItem?.name || t('checkout.unknownItem'),
                    price: originalItem?.price || 0
                };
            });

        if (itemsToSplit.length === 0) return;

        setIsProcessingSplit(true);

        try {
            // Calculate total for split items
            let total = 0;
            itemsToSplit.forEach(splitItem => {
                total += splitItem.price * splitItem.quantity;
            });

            await orderOps.splitOrder(sourceOrder, {
                splitAmount: total,
                items: itemsToSplit,
                paymentMethod: method
            });

            // Server handles everything via event sourcing
            // State will be updated when server emits OrderUpdated event

            onClose();
            toast.success(t('checkout.split.success'));
        } catch (err) {
            console.error("Split failed:", err);
            toast.error(t('checkout.split.failed') + ": " + err);
        } finally {
            setIsProcessingSplit(false);
        }
    };

    const splitTotal = useMemo(() => {
        if (!sourceOrder) return 0;
        let total = 0;
        (Object.entries(splitItems) as [string, number][]).forEach(([id, qty]) => {
            const item = sourceOrder.items.find(i => i.id === id);
            if (item) {
                total += (item.price || 0) * qty;
            }
        });
        return total;
    }, [splitItems, sourceOrder]);

    // Filter tables based on mode
    const displayedTables = useMemo(() => {
        return zoneTables.filter(table => {
            if (table.id === sourceTable.id) return false; // Don't show self

            const isOccupied = heldOrders.some(o => o.key === table.id);

            if (mode === 'MERGE') {
                return isOccupied;
            }
            if (mode === 'MOVE') {
                return true; // Show all tables for move (so user sees context)
            }
            return false;
        });
    }, [zoneTables, mode, heldOrders, sourceTable.id]);

    const renderSplit = () => {
        if (!sourceOrder) return null;

        return (
            <div className="flex flex-col h-full bg-gray-50/50">
                <div className="p-3 bg-white border-b border-gray-100 flex justify-between items-center shadow-sm z-10">
                    <h3 className="font-bold text-gray-800 text-base flex items-center gap-2">
                        <Split size={18} className="text-purple-600" />
                        {t('checkout.split.title')}
                    </h3>
                    <button onClick={() => { setMode('MENU'); setSplitItems({}); }} className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 text-gray-600 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-all">
                        <ArrowLeft size={14} /> {t('common.back')}
                    </button>
                </div>

                <div className="flex-1 overflow-y-auto p-4 custom-scrollbar">
                    <div className="space-y-2">
                        {sourceOrder.items.map(item => {
                            const currentSplitQty = splitItems[item.id] || 0;
                            const paidQty = (sourceOrder.paidItemQuantities && sourceOrder.paidItemQuantities[item.instanceId]) || 0;
                            const maxQty = Math.max(0, item.quantity - paidQty);

                            return (
                                <div key={item.id} className={`bg-white p-3 rounded-lg border border-gray-200 shadow-sm flex items-center justify-between ${maxQty === 0 ? 'opacity-60 bg-gray-50' : ''}`}>
                                    <div className="flex-1">
                                        <div className="font-bold text-gray-800">
                                            {item.name}
                                            {paidQty > 0 && <span className="text-xs text-green-600 ml-2 font-medium">({t('checkout.paidQty', { qty: paidQty })})</span>}
                                        </div>
                                        <div className="text-sm text-gray-500">${(item.price || 0).toFixed(2)}</div>
                                    </div>

                                    <div className="flex items-center gap-3 bg-gray-50 rounded-lg p-1 border border-gray-100">
                                        <button
                                            onClick={() => setSplitItems(prev => ({ ...prev, [item.id]: Math.max(0, (prev[item.id] || 0) - 1) }))}
                                            disabled={currentSplitQty <= 0}
                                            className="w-8 h-8 flex items-center justify-center rounded-md bg-white border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-50 transition-colors"
                                        >
                                            <Minus size={14} />
                                        </button>
                                        <span className="w-8 text-center font-bold text-gray-800">{currentSplitQty}</span>
                                        <button
                                            onClick={() => setSplitItems(prev => ({ ...prev, [item.id]: Math.min(maxQty, (prev[item.id] || 0) + 1) }))}
                                            disabled={currentSplitQty >= maxQty || maxQty === 0}
                                            className="w-8 h-8 flex items-center justify-center rounded-md bg-white border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-50 transition-colors"
                                        >
                                            <Plus size={14} />
                                        </button>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                </div>

                <div className="p-4 bg-white border-t border-gray-100 shadow-[0_-4px_6px_-1px_rgba(0,0,0,0.05)] space-y-3">
                    <div className="flex justify-between items-center mb-2">
                        <span className="text-gray-500 font-medium">{t('checkout.split.total')}</span>
                        <span className="text-2xl font-bold text-gray-900">${splitTotal.toFixed(2)}</span>
                    </div>

                    <div className="grid grid-cols-2 gap-3">
                        <button
                            onClick={() => handleSplitPayment('CASH')}
                            disabled={splitTotal <= 0 || isProcessingSplit}
                            className="flex items-center justify-center gap-2 py-3 bg-green-600 text-white rounded-lg font-bold hover:bg-green-700 hover:shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.99]"
                        >
                            <Banknote size={18} />
                            {t('checkout.split.payCash')}
                        </button>
                        <button
                            onClick={() => handleSplitPayment('CARD')}
                            disabled={splitTotal <= 0 || isProcessingSplit}
                            className="flex items-center justify-center gap-2 py-3 bg-blue-600 text-white rounded-lg font-bold hover:bg-blue-700 hover:shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.99]"
                        >
                            <CreditCard size={18} />
                            {t('checkout.split.payCard')}
                        </button>
                    </div>
                </div>
            </div>
        );
    };

    const renderMenu = () => (
        <div className="grid grid-cols-2 gap-4 p-6">
            {hasPayments && (
                <div className="col-span-2 bg-red-50 border border-red-100 text-red-600 px-3 py-2 rounded-lg text-xs font-medium flex items-center gap-2">
                    <X size={14} />
                    {t('table.warning.cannotMoveWithPayments')}
                </div>
            )}

            <EscalatableGate
                permission={Permission.MERGE_BILL}
                mode="intercept"
                description={t('table.authRequired.merge')}
                onAuthorized={() => setMode('MERGE')}
            >
                <button
                    onClick={() => setMode('MERGE')}
                    disabled={hasPayments}
                    className={`flex flex-col items-center justify-center text-center p-4 h-32 rounded-lg border transition-all duration-200 gap-2 ${hasPayments
                            ? 'bg-gray-50 border-gray-200 opacity-50 cursor-not-allowed'
                            : 'bg-blue-50/50 hover:bg-blue-50 border-blue-100 hover:border-blue-200 hover:shadow-md active:scale-[0.99]'
                        }`}
                >
                    <div className={`w-12 h-12 rounded-full flex items-center justify-center shadow-sm ${hasPayments ? 'bg-gray-200 text-gray-500' : 'bg-blue-100 text-blue-600'}`}>
                        <Users size={24} />
                    </div>
                    <div>
                        <div className="font-bold text-gray-800 text-base mb-0.5">{t('table.action.merge')}</div>
                        <div className="text-xs text-gray-500 font-medium">{t('table.combineDescription')}</div>
                    </div>
                </button>
            </EscalatableGate>

            <EscalatableGate
                permission={Permission.TRANSFER_TABLE}
                mode="intercept"
                description={t('table.authRequired.move')}
                onAuthorized={() => setMode('MOVE')}
            >
                <button
                    onClick={() => setMode('MOVE')}
                    disabled={hasPayments}
                    className={`flex flex-col items-center justify-center text-center p-4 h-32 rounded-lg border transition-all duration-200 gap-2 ${hasPayments
                            ? 'bg-gray-50 border-gray-200 opacity-50 cursor-not-allowed'
                            : 'bg-orange-50/50 hover:bg-orange-50 border-orange-100 hover:border-orange-200 hover:shadow-md active:scale-[0.99]'
                        }`}
                >
                    <div className={`w-12 h-12 rounded-full flex items-center justify-center shadow-sm ${hasPayments ? 'bg-gray-200 text-gray-500' : 'bg-orange-100 text-orange-600'}`}>
                        <ArrowLeftRight size={24} />
                    </div>
                    <div>
                        <div className="font-bold text-gray-800 text-base mb-0.5">{t('table.action.move')}</div>
                        <div className="text-xs text-gray-500 font-medium">{t('table.description.move')}</div>
                    </div>
                </button>
            </EscalatableGate>
                        <EscalatableGate
                permission={Permission.APPLY_DISCOUNT}
                mode="intercept"
                description={t('table.authRequired.discount')}
                onAuthorized={handleWaiveSurcharge}
            >
            {(hasSurcharge || isSurchargeExempt) && (
                <button
                    onClick={handleWaiveSurcharge}
                    className={`col-span-2 flex items-center p-4 rounded-lg border transition-all duration-200 gap-4 ${isSurchargeExempt
                            ? 'bg-green-50/50 border-green-200 shadow-sm'
                            : 'bg-white hover:bg-gray-50 border-gray-200 hover:border-gray-300 hover:shadow-md active:scale-[0.99]'
                        }`}
                >
                    <div className={`w-10 h-10 rounded-full flex items-center justify-center shadow-sm shrink-0 ${isSurchargeExempt ? 'bg-green-100 text-green-600' : 'bg-gray-100 text-gray-500'
                        }`}>
                        {isSurchargeExempt ? <Check size={18} /> : <Percent size={18} />}
                    </div>
                    <div className="flex-1 text-left">
                        <div className="font-bold text-gray-800 text-base mb-0.5">{t('checkout.surcharge.exempt')}</div>
                        <div className="text-xs text-gray-500 font-medium">
                            {isSurchargeExempt ? t('checkout.surcharge.waived') : t('checkout.surcharge.description')}
                        </div>
                    </div>
                </button>
            )}
            </EscalatableGate>
        </div>
    );

    const renderTableSelector = () => (
        <div className="flex flex-1 overflow-hidden">
            <ZoneSidebar
                zones={zones}
                activeZoneId={activeZoneId}
                onZoneSelect={setActiveZoneId}
            />
            <div className="flex-1 flex flex-col bg-white">
                <div className="p-3 bg-white border-b border-gray-100 flex justify-between items-center shadow-sm z-10">
                    <h3 className="font-bold text-gray-800 text-base">
                        {mode === 'MERGE' ? t('table.action.merge') : t('table.action.move')} - {t('table.selectTarget')}
                    </h3>
                    <button onClick={() => { setMode('MENU'); setSelectedTargetTable(null); }} className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 text-gray-600 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-all">
                        <ArrowLeft size={14} /> {t('common.back')}
                    </button>
                </div>
                <div className="flex-1 overflow-y-auto p-4 custom-scrollbar bg-gray-50/50">
                    {displayedTables.length === 0 ? (
                        <div className="flex flex-col items-center justify-center h-full text-gray-400 gap-3">
                            <div className="w-12 h-12 bg-gray-100 rounded-full flex items-center justify-center">
                                <LayoutGrid size={24} className="opacity-50" />
                            </div>
                            <p className="text-sm font-medium">{t('table.noTables')}</p>
                        </div>
                    ) : (
                        <div className="grid grid-cols-3 md:grid-cols-4 xl:grid-cols-5 gap-3">
                            {displayedTables.map(table => {
                                const order = heldOrders.find(o => o.key === table.id);
                                const isOccupied = !!order;
                                const isSelected = selectedTargetTable?.id === table.id;

                                const isDisabled = mode === 'MOVE' ? isOccupied : false;
                                const cardMode = mode === 'MOVE' ? 'HOLD' : 'RETRIEVE';

                                return (
                                    <div key={table.id} className={`relative transition-all duration-200 ${isSelected ? 'ring-2 ring-blue-500 ring-offset-2 rounded-xl shadow-lg transform scale-[1.02] z-10' : isDisabled ? 'opacity-60 grayscale-[0.5]' : 'hover:scale-[1.02] hover:shadow-md'}`}>
                                        <TableCard
                                            table={table}
                                            order={order}
                                            mode={cardMode}
                                            disabled={isDisabled}
                                            className="h-24 w-full text-base"
                                            onClick={() => !isDisabled && setSelectedTargetTable(table)}
                                        />
                                        {isSelected && (
                                            <div className="absolute -top-1.5 -right-1.5 bg-blue-500 text-white rounded-full p-1 z-20 shadow-md ring-2 ring-white">
                                                <Check size={14} />
                                            </div>
                                        )}
                                    </div>
                                );
                            })}
                        </div>
                    )}
                </div>
                <div className="p-4 bg-white border-t border-gray-100 shadow-[0_-4px_6px_-1px_rgba(0,0,0,0.05)]">
                    <button
                        onClick={mode === 'MERGE' ? handleMerge : handleMove}
                        disabled={!selectedTargetTable}
                        className="w-full py-3 bg-blue-600 text-white rounded-lg font-bold text-base disabled:opacity-50 disabled:cursor-not-allowed hover:bg-blue-700 hover:shadow-lg transition-all active:scale-[0.99]"
                    >
                        {t('common.confirm')}
                    </button>
                </div>
            </div>
        </div>
    );

    return (
	    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-60 flex items-center justify-center p-6 font-sans">
            <div className="bg-white w-full max-w-3xl h-[520px] rounded-xl shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 duration-200 border border-gray-100">
                <div className="h-14 border-b border-gray-100 flex items-center justify-between px-6 bg-white shadow-sm z-20">
                    <div className="font-bold text-lg text-gray-800 flex items-center gap-3">
                        <span>{t('table.action.manage')}</span>
                        <span className="text-gray-200 text-xl font-light">|</span>
                        <span className="text-blue-600 bg-blue-50 px-3 py-1 rounded-lg border border-blue-100 text-base">{sourceTable.name}</span>
                    </div>
                    <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full text-gray-400 hover:text-gray-600 transition-colors">
                        <X size={20} />
                    </button>
                </div>

                <div className="flex-1 overflow-hidden flex flex-col bg-white">
                    {mode === 'MENU' ? renderMenu() : mode === 'SPLIT' ? renderSplit() : renderTableSelector()}
                </div>
            </div>
        </div>
    );
};
