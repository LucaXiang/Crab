import React, { useState, useMemo, useEffect } from 'react';
import { X, ArrowLeftRight, Users, Check, ArrowLeft, LayoutGrid, Split, Minus, Plus, CreditCard, Banknote, Percent } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';

const getApi = () => createTauriClient();
import { Table, Zone, HeldOrder, Permission, AppliedRule } from '@/core/domain/types';
import { Currency } from '@/utils/currency';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import * as orderOps from '@/core/stores/order/useOrderOperations';
import { toggleRuleSkip } from '@/core/stores/order/useOrderOperations';
import { ZoneSidebar } from '../ZoneSidebar';
import { formatCurrency } from '@/utils/currency';
import { TableCard } from '../TableCard';
import { toast } from '@/presentation/components/Toast';
import { getErrorMessage } from '@/utils/error';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';

interface TableManagementModalProps {
    sourceTable: Table;
    zones: Zone[];
    heldOrders: HeldOrder[];
    onClose: () => void;
    onSuccess: (navigateToTableId?: string) => void;
}

type ManagementMode = 'MENU' | 'MERGE' | 'MOVE' | 'SPLIT' | 'PRICE_RULES';

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
                const tables = await getApi().listTables();
                setZoneTables(tables);
            } catch (e) {
                toast.error(getErrorMessage(e));
            }
        };
        loadTables();
    }, [activeZoneId]);

    // Ensure a valid initial zone when zones prop arrives or source order indicates a zone
    useEffect(() => {
        if (!zones || zones.length === 0) return;
        if (activeZoneId) return;
        const sourceOrder = heldOrders.find(o => o.table_id === sourceTable.id);
        const preferZone = sourceOrder?.zone_name ? zones.find(z => z.name === sourceOrder.zone_name) : undefined;
        const nextZoneId = preferZone?.id || zones[0].id;
        if (nextZoneId) setActiveZoneId(nextZoneId);
    }, [zones, heldOrders, sourceTable.id, activeZoneId]);

    // Get source order from active store or fallback to prop
    const sourceOrderSnapshot = useActiveOrdersStore(state => state.getOrderByTable(sourceTable.id ));
    const sourceOrder = sourceOrderSnapshot ? sourceOrderSnapshot : heldOrders.find(o => o.table_id === sourceTable.id);

    const hasPayments = useMemo(() => {
        if (!sourceOrder) return false;
        const hasPaidAmount = sourceOrder.paid_amount > 0;
        const hasPaidItems = sourceOrder.paid_item_quantities && Object.keys(sourceOrder.paid_item_quantities).length > 0;
        return hasPaidAmount || hasPaidItems;
    }, [sourceOrder]);

    const handleMerge = async () => {
        if (!selectedTargetTable || !sourceOrder) return;
        const store = useActiveOrdersStore.getState();
        const targetSnapshot = store.getOrder(selectedTargetTable.id );
        if (!targetSnapshot) return;

        try {
            // Fire & forget - UI updates via WebSocket
            await orderOps.mergeOrders(sourceOrder.order_id, targetSnapshot.order_id);
            onSuccess(selectedTargetTable.id );
        } catch (err) {
            console.error('Merge failed:', err);
            toast.error(t('checkout.error.merge_failed'));
        }
    };

    const handleMove = async () => {
        if (!selectedTargetTable || !sourceOrder) return;
        const targetZone = zones.find(z => z.id === selectedTargetTable.zone);

        try {
            // Fire & forget - UI updates via WebSocket
            await orderOps.moveOrder(
                sourceOrder.order_id,
                selectedTargetTable.id ,
                selectedTargetTable.name,
                targetZone?.name
            );
            onSuccess(selectedTargetTable.id );
        } catch (err) {
            console.error('Move failed:', err);
            toast.error(t('checkout.error.move_failed'));
        }
    };

    const handleSplitPayment = async (method: 'CASH' | 'CARD') => {
        if (!sourceOrder || isProcessingSplit) return;

        const itemsToSplit = (Object.entries(splitItems) as [string, number][])
            .filter(([_, qty]) => qty > 0)
            .map(([instanceId, qty]) => {
                const originalItem = sourceOrder.items.find(i => i.id === instanceId);
                return {
                    instance_id: instanceId,
                    quantity: qty,
                    name: originalItem?.name || t('common.label.unknown_item'),
                    price: originalItem?.price || 0,
                    unit_price: originalItem?.unit_price ?? originalItem?.price ?? 0
                };
            });

        if (itemsToSplit.length === 0) return;

        setIsProcessingSplit(true);

        try {
            // Fire & forget - UI updates via WebSocket
            await orderOps.splitOrder(sourceOrder.order_id, {
                items: itemsToSplit,
                paymentMethod: method
            });

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
        let total = Currency.toDecimal(0);
        (Object.entries(splitItems) as [string, number][]).forEach(([id, qty]) => {
            const item = sourceOrder.items.find(i => i.id === id);
            if (item) {
                total = Currency.add(total, Currency.mul(item.price, qty));
            }
        });
        return Currency.round2(total).toNumber();
    }, [splitItems, sourceOrder]);

    // Filter tables based on mode
    const displayedTables = useMemo(() => {
        return zoneTables.filter(table => {
            if (table.id === sourceTable.id) return false; // Don't show self

            const isOccupied = heldOrders.some(o => o.table_id === table.id);

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
                        <ArrowLeft size={14} /> {t('common.action.back')}
                    </button>
                </div>

                <div className="flex-1 overflow-y-auto p-4 custom-scrollbar">
                    <div className="space-y-2">
                        {sourceOrder.items.map(item => {
                            const currentSplitQty = splitItems[item.id] || 0;
                            const paidQty = (sourceOrder.paid_item_quantities && sourceOrder.paid_item_quantities[item.instance_id]) || 0;
                            const maxQty = Math.max(0, item.quantity - paidQty);

                            return (
                                <div key={item.id} className={`bg-white p-3 rounded-lg border border-gray-200 shadow-sm flex items-center justify-between ${maxQty === 0 ? 'opacity-60 bg-gray-50' : ''}`}>
                                    <div className="flex-1">
                                        <div className="font-bold text-gray-800">
                                            {item.name}
                                            {paidQty > 0 && <span className="text-xs text-green-600 ml-2 font-medium">({t('checkout.paidQty', { qty: paidQty })})</span>}
                                        </div>
                                        <div className="text-sm text-gray-500">${item.price.toFixed(2)}</div>
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

                <div className="p-4 bg-white border-t border-gray-100 shadow-up space-y-3">
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
                            {t('checkout.split.pay_cash')}
                        </button>
                        <button
                            onClick={() => handleSplitPayment('CARD')}
                            disabled={splitTotal <= 0 || isProcessingSplit}
                            className="flex items-center justify-center gap-2 py-3 bg-blue-600 text-white rounded-lg font-bold hover:bg-blue-700 hover:shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.99]"
                        >
                            <CreditCard size={18} />
                            {t('checkout.split.pay_card')}
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
                    {t('table.warning.cannot_move_with_payments')}
                </div>
            )}

            <EscalatableGate
                permission={Permission.TABLES_MERGE_BILL}
                mode="intercept"
                description={t('table.auth_required.merge')}
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
                        <div className="text-xs text-gray-500 font-medium">{t('table.combine_description')}</div>
                    </div>
                </button>
            </EscalatableGate>

            <EscalatableGate
                permission={Permission.TABLES_TRANSFER}
                mode="intercept"
                description={t('table.auth_required.move')}
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

            {/* Price Rules Button */}
            <button
                onClick={() => setMode('PRICE_RULES')}
                className="flex flex-col items-center justify-center text-center p-4 h-32 rounded-lg border transition-all duration-200 gap-2 bg-teal-50/50 hover:bg-teal-50 border-teal-100 hover:border-teal-200 hover:shadow-md active:scale-[0.99]"
            >
                <div className="w-12 h-12 rounded-full flex items-center justify-center shadow-sm bg-teal-100 text-teal-600">
                    <Percent size={24} />
                </div>
                <div>
                    <div className="font-bold text-gray-800 text-base mb-0.5">{t('table.action.price_rules')}</div>
                    <div className="text-xs text-gray-500 font-medium">{t('table.description.price_rules')}</div>
                </div>
            </button>
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
                        {mode === 'MERGE' ? t('table.action.merge') : t('table.action.move')} - {t('table.select_target')}
                    </h3>
                    <button onClick={() => { setMode('MENU'); setSelectedTargetTable(null); }} className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 text-gray-600 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-all">
                        <ArrowLeft size={14} /> {t('common.action.back')}
                    </button>
                </div>
                <div className="flex-1 overflow-y-auto p-4 custom-scrollbar bg-gray-50/50">
                    {displayedTables.length === 0 ? (
                        <div className="flex flex-col items-center justify-center h-full text-gray-400 gap-3">
                            <div className="w-12 h-12 bg-gray-100 rounded-full flex items-center justify-center">
                                <LayoutGrid size={24} className="opacity-50" />
                            </div>
                            <p className="text-sm font-medium">{t('table.no_tables')}</p>
                        </div>
                    ) : (
                        <div className="grid grid-cols-3 md:grid-cols-4 xl:grid-cols-5 gap-3">
                            {displayedTables.map(table => {
                                const order = heldOrders.find(o => o.table_id === table.id);
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
                <div className="p-4 bg-white border-t border-gray-100 shadow-up">
                    <button
                        onClick={mode === 'MERGE' ? handleMerge : handleMove}
                        disabled={!selectedTargetTable}
                        className="w-full py-3 bg-blue-600 text-white rounded-lg font-bold text-base disabled:opacity-50 disabled:cursor-not-allowed hover:bg-blue-700 hover:shadow-lg transition-all active:scale-[0.99]"
                    >
                        {t('common.action.confirm')}
                    </button>
                </div>
            </div>
        </div>
    );

    // Collect all applied rules from order (items + order-level)
    const allAppliedRules = useMemo(() => {
        if (!sourceOrder) return [];
        
        const rulesMap = new Map<string, { rule: AppliedRule; sources: string[] }>();
        
        // Add item-level rules
        sourceOrder.items.forEach(item => {
            (item.applied_rules ?? []).forEach(rule => {
                const existing = rulesMap.get(rule.rule_id);
                if (existing) {
                    if (!existing.sources.includes(item.name)) {
                        existing.sources.push(item.name);
                    }
                } else {
                    rulesMap.set(rule.rule_id, { rule, sources: [item.name] });
                }
            });
        });
        
        // Add order-level rules
        (sourceOrder.order_applied_rules ?? []).forEach(rule => {
            const existing = rulesMap.get(rule.rule_id);
            if (existing) {
                if (!existing.sources.includes(t('table.price_rules.order_level'))) {
                    existing.sources.push(t('table.price_rules.order_level'));
                }
            } else {
                rulesMap.set(rule.rule_id, { rule, sources: [t('table.price_rules.order_level')] });
            }
        });
        
        return Array.from(rulesMap.values());
    }, [sourceOrder, t]);

    const handleToggleRule = async (ruleId: string, currentSkipped: boolean) => {
        if (!sourceOrder) return;
        try {
            await toggleRuleSkip(sourceOrder.order_id, ruleId, !currentSkipped);
            toast.success(currentSkipped ? t('table.price_rules.rule_enabled') : t('table.price_rules.rule_disabled'));
        } catch (err) {
            console.error('Toggle rule failed:', err);
            toast.error(t('table.price_rules.toggle_failed'));
        }
    };

    const renderPriceRules = () => {
        if (!sourceOrder) return null;

        return (
            <div className="flex flex-col h-full bg-gray-50/50">
                <div className="p-3 bg-white border-b border-gray-100 flex justify-between items-center shadow-sm z-10">
                    <h3 className="font-bold text-gray-800 text-base flex items-center gap-2">
                        <Percent size={18} className="text-teal-600" />
                        {t('table.action.price_rules')}
                    </h3>
                    <button onClick={() => setMode('MENU')} className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 text-gray-600 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-all">
                        <ArrowLeft size={14} /> {t('common.action.back')}
                    </button>
                </div>

                <div className="flex-1 overflow-y-auto p-4 custom-scrollbar">
                    {allAppliedRules.length === 0 ? (
                        <div className="flex flex-col items-center justify-center h-full text-gray-400 gap-3">
                            <div className="w-12 h-12 bg-gray-100 rounded-full flex items-center justify-center">
                                <Percent size={24} className="opacity-50" />
                            </div>
                            <p className="text-sm font-medium">{t('table.price_rules.no_rules')}</p>
                        </div>
                    ) : (
                        <div className="space-y-2">
                            {allAppliedRules.map(({ rule, sources }) => (
                                <div
                                    key={rule.rule_id}
                                    className={`bg-white p-4 rounded-lg border shadow-sm transition-all ${
                                        rule.skipped ? 'border-gray-200 opacity-60' : 'border-gray-200'
                                    }`}
                                >
                                    <div className="flex items-start justify-between gap-4">
                                        <div className="flex-1 min-w-0">
                                            <div className="flex items-center gap-2 mb-1">
                                                <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                                                    rule.rule_type === 'DISCOUNT'
                                                        ? 'bg-green-100 text-green-700'
                                                        : 'bg-amber-100 text-amber-700'
                                                }`}>
                                                    {rule.rule_type === 'DISCOUNT' ? t('settings.price_rule.type.discount') : t('settings.price_rule.type.surcharge')}
                                                </span>
                                                {rule.skipped && (
                                                    <span className="text-xs px-2 py-0.5 rounded-full bg-gray-100 text-gray-500 font-medium">
                                                        {t('table.price_rules.skipped')}
                                                    </span>
                                                )}
                                            </div>
                                            <div className="font-bold text-gray-800">{rule.display_name}</div>
                                            <div className="text-xs text-gray-500 mt-1">
                                                {rule.adjustment_type === 'PERCENTAGE'
                                                    ? `${rule.adjustment_value}%`
                                                    : formatCurrency(rule.adjustment_value)}
                                                {' · '}
                                                {t('table.price_rules.applied_to')}: {sources.join(', ')}
                                            </div>
                                            <div className={`text-sm font-bold mt-2 ${
                                                rule.rule_type === 'DISCOUNT' ? 'text-green-600' : 'text-amber-600'
                                            }`}>
                                                {rule.rule_type === 'DISCOUNT' ? '-' : '+'}
                                                {formatCurrency(Math.abs(rule.calculated_amount))}
                                            </div>
                                        </div>
                                        <label className="relative inline-flex items-center cursor-pointer shrink-0">
                                            <input
                                                type="checkbox"
                                                className="sr-only peer"
                                                checked={!rule.skipped}
                                                onChange={() => handleToggleRule(rule.rule_id, rule.skipped)}
                                            />
                                            <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-teal-100 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-teal-500 shadow-sm"></div>
                                        </label>
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>
        );
    };

    return (
	    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-60 flex items-center justify-center p-6 font-sans">
            <div className="bg-white w-full max-w-3xl h-[32.5rem] rounded-xl shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 duration-200 border border-gray-100">
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
                    {mode === 'MENU' ? renderMenu() : mode === 'SPLIT' ? renderSplit() : mode === 'PRICE_RULES' ? renderPriceRules() : renderTableSelector()}
                </div>
            </div>
        </div>
    );
};
