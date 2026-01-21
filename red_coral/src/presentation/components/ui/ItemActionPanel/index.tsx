import React, { useState, useEffect } from 'react';
import { Minus, Plus, Percent, X, Trash2, Check } from 'lucide-react';
import { Numpad } from '../Numpad';
import { Currency } from '@/utils/currency';
import { formatCurrency } from '@/utils/currency';
import { EscalatableGate } from '../../auth/EscalatableGate';
import { Permission } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';

import { ItemActionPanelProps, EditMode } from './types';

// --- Logic Hook ---
const useItemActionLogic = (props: ItemActionPanelProps) => {
  const {
    basePrice,
    optionsModifier,
    discount,
    quantity,
    onQuantityChange,
    onDiscountChange,
  } = props;

  const [editMode, setEditMode] = useState<EditMode>('STANDARD');
  const [inputBuffer, setInputBuffer] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const [authorizedUser, setAuthorizedUser] = useState<{ id: string; username: string } | null>(null);

  // --- Calculations ---
  // 1. Base + Options
  const unitPriceBeforeDiscount = Currency.add(basePrice, optionsModifier);
  // 2. Discount Amount (on the enhanced unit price)
  const unitDiscountAmount = Currency.floor2(Currency.mul(unitPriceBeforeDiscount, discount / 100));
  // 3. Final Unit Price
  const unitPriceFinal = Currency.sub(unitPriceBeforeDiscount, unitDiscountAmount);
  // 4. Total Line Price
  const finalTotal = Currency.mul(unitPriceFinal, quantity);

  // --- Numpad Handlers ---

  const openNumpad = (mode: 'QTY' | 'DISC' | 'PRICE', authorizer?: { id: string; username: string }) => {
    setEditMode(mode);
    setIsTyping(false);
    if (authorizer) {
      setAuthorizedUser(authorizer);
    } else {
      setAuthorizedUser(null);
    }

    let initialValue = '';
    if (mode === 'QTY') initialValue = quantity.toString();
    else if (mode === 'DISC') initialValue = discount.toString();
    else if (mode === 'PRICE') initialValue = unitPriceFinal.toFixed(2);

    setInputBuffer(initialValue);
  };

  useEffect(() => {
    // Sync buffer changes to state (Live Preview)
    if (editMode === 'QTY') {
      const val = parseInt(inputBuffer);
      if (!isNaN(val) && val > 0) {
        onQuantityChange(val);
      }
    } else if (editMode === 'DISC') {
      let val = parseFloat(inputBuffer);
      if (!isNaN(val)) {
        if (val > 100) val = 100;
        // NOTE: For live preview, we don't pass authorizer yet because it's not final confirmation
        onDiscountChange(val); 
      }
    }
    // PRICE mode is handled on confirm to avoid circular dependency issues during typing
  }, [inputBuffer, editMode]);

  const handleConfirmInput = () => {
    if (editMode === 'PRICE') {
      const targetPrice = parseFloat(inputBuffer);
      if (!isNaN(targetPrice) && targetPrice >= 0) {
        // Reverse calculate discount
        if (unitPriceBeforeDiscount.gt(0)) {
          const baseVal = unitPriceBeforeDiscount.toNumber();
          let newDiscount = 100 * (1 - targetPrice / baseVal);
          // Clamp discount
          newDiscount = Math.max(0, Math.min(100, newDiscount));
          // For price override, if it implies a discount, we might want to pass authorizer if we had one for 'PRICE' mode?
          // Currently PRICE mode isn't protected explicitly here, but maybe it should be if it results in discount?
          // The request specifically asked for Discount protection.
          onDiscountChange(parseFloat(newDiscount.toFixed(2)), authorizedUser || undefined);
        }
      }
    } else if (editMode === 'DISC') {
        let val = parseFloat(inputBuffer);
        if (!isNaN(val)) {
            if (val > 100) val = 100;
            onDiscountChange(val, authorizedUser || undefined);
        }
    }
    setEditMode('STANDARD');
    setIsTyping(false);
    setAuthorizedUser(null);
  };

  const handleNumInput = (char: string) => {
    setInputBuffer(prev => {
      if (!isTyping) {
        setIsTyping(true);
        return char === '.' ? '0.' : char;
      }
      if (char === '.' && prev.includes('.')) return prev;
      if (prev.replace('.', '').length >= 4) return prev;
      return prev + char;
    });
  };

  const handleClearInput = () => {
    setInputBuffer('0');
    setIsTyping(true);
  };

  const incrementQty = (delta: number) => {
    onQuantityChange(Math.max(1, quantity + delta));
  };

  return {
    editMode,
    setEditMode,
    inputBuffer,
    setInputBuffer,
    isTyping,
    unitPriceFinal,
    finalTotal,
    openNumpad,
    handleConfirmInput,
    handleNumInput,
    handleClearInput,
    incrementQty,
  };
};

// --- Main Component ---
export const ItemActionPanel: React.FC<ItemActionPanelProps> = (props) => {
  const {
    t,
    quantity,
    discount,
    basePrice,
    optionsModifier,
    onDiscountChange,
    onConfirm,
    onCancel,
    onDelete,
    confirmLabel,
    showDelete,
  } = props;

  const {
    editMode,
    setEditMode,
    inputBuffer,
    setInputBuffer,
    isTyping,
    unitPriceFinal,
    handleNumInput,
    handleClearInput,
    handleConfirmInput,
    incrementQty,
    openNumpad,
    finalTotal
  } = useItemActionLogic(props);

  const { user: currentUser } = useAuthStore();
  const { hasPermission } = usePermission();

  const DISCOUNTS = [10, 20, 50, 100];

  const handleProtectedDelete = (authorizer?: { id: string; username: string }) => {
     if (onDelete) {
        onDelete(authorizer || (currentUser ? { id: String(currentUser.id), username: currentUser.username } : undefined));
     }
  };

  const handleProtectedDiscount = (val: number, authorizer?: { id: string; username: string }) => {
      onDiscountChange(val, authorizer || (currentUser ? { id: String(currentUser.id), username: currentUser.username } : undefined));
  };

  return (
    <div className="flex flex-col h-full bg-white">
      {editMode === 'STANDARD' ? (
        <>
          <div className="flex-1 overflow-y-auto p-5 space-y-5 custom-scrollbar">
            
            {/* 1. Compact Price Breakdown */}
            <div className="bg-gray-50/80 border border-gray-100 rounded-2xl p-4 space-y-2">
              <div className="flex justify-between items-center text-sm">
                 <div className="flex items-center gap-2">
                    <span className="text-gray-500">{t('pos.cart.unitPrice')}</span>
                    {discount > 0 && (
                        <span className="text-xs font-bold text-red-600 bg-red-100 px-1.5 py-0.5 rounded">
                            -{discount}%
                        </span>
                    )}
                 </div>
                 <div className="flex flex-col items-end">
                    {/* Original Price (if discounted) */}
                    {discount > 0 && (
                        <span className="text-xs text-gray-400 line-through decoration-gray-300">
                            {formatCurrency(Currency.add(basePrice, optionsModifier).toNumber())}
                        </span>
                    )}
                    {/* Final Unit Price */}
                    <span className="text-xl font-bold text-gray-900 font-mono leading-none">
                        {formatCurrency(unitPriceFinal.toNumber())}
                    </span>
                 </div>
              </div>
              
              {/* Collapsed Details - Only show if relevant */}
              {(optionsModifier !== 0) && (
                  <div className="pt-2 border-t border-gray-200/50 flex justify-between text-xs text-gray-400">
                    <span>{t('pos.cart.basePrice')}: {formatCurrency(basePrice)}</span>
                    <span className={optionsModifier > 0 ? 'text-orange-500' : 'text-green-600'}>
                        {t('pos.product.options')}: {optionsModifier > 0 ? '+' : ''}{formatCurrency(optionsModifier)}
                    </span>
                  </div>
              )}
            </div>

            {/* 2. Controls Group */}
            <div className="space-y-5">
                {/* Quantity */}
                <div className="space-y-2">
                    <div className="flex justify-between items-end px-1">
                        <label className="text-xs font-bold text-gray-900 uppercase tracking-wider">
                            {t('common.label.quantity')}
                        </label>
                    </div>
                    <div className="flex items-center gap-3">
                        <button
                        onClick={() => incrementQty(-1)}
                        className="w-14 h-14 bg-white border border-gray-200 shadow-sm rounded-xl flex items-center justify-center text-gray-600 active:scale-95 hover:border-gray-300 transition-all"
                        >
                        <Minus size={24} />
                        </button>
                        <button
                        onClick={() => openNumpad('QTY')}
                        className="flex-1 h-14 bg-gray-50 rounded-xl border border-transparent hover:bg-white hover:border-blue-200 hover:shadow-sm transition-all flex flex-col items-center justify-center group cursor-pointer"
                        >
                        <span className="text-2xl font-bold text-gray-800 group-hover:text-blue-600 transition-colors">
                            {quantity}
                        </span>
                        </button>
                        <button
                        onClick={() => incrementQty(1)}
                        className="w-14 h-14 bg-white border border-gray-200 shadow-sm rounded-xl flex items-center justify-center text-gray-600 active:scale-95 hover:border-gray-300 transition-all"
                        >
                        <Plus size={24} />
                        </button>
                    </div>
                </div>

                {/* Discount */}
                <div className="space-y-2">
                    <div className="flex justify-between items-end px-1">
                        <label className="text-xs font-bold text-gray-900 uppercase tracking-wider">
                            {t('checkout.cart.discount')}
                        </label>
                    </div>
                    
                    {/* Discount Controls */}
                    <div className="bg-white rounded-2xl border border-gray-200 p-2 shadow-sm space-y-2">
                        <div className="flex items-center gap-2">
                            <EscalatableGate
                                permission={Permission.APPLY_DISCOUNT}
                                mode="intercept"
                                description={t('pos.cart.enterDiscount')}
                                onAuthorized={(user) => openNumpad('DISC', { id: String(user.id), username: user.username })}
                            >
                                <button
                                    onClick={() => {
                                        if (hasPermission(Permission.APPLY_DISCOUNT)) {
                                            openNumpad('DISC');
                                        }
                                    }}
                                    className={`
                                        flex-1 h-10 rounded-lg border flex items-center justify-center gap-2 font-bold transition-all text-sm
                                        ${discount > 0 
                                            ? 'bg-red-50 border-red-200 text-red-600' 
                                            : 'bg-gray-50 border-gray-100 text-gray-500 hover:bg-gray-100'}
                                    `}
                                >
                                    {discount > 0 ? (
                                        <>
                                            <span className="text-lg">{discount}%</span>
                                            <span className="text-[10px] uppercase opacity-75">{t('common.discount.off')}</span>
                                        </>
                                    ) : (
                                        <>
                                            <Percent size={16} />
                                            <span>{t('pos.cart.custom')}</span>
                                        </>
                                    )}
                                </button>
                            </EscalatableGate>
                            {discount > 0 && (
                                <button 
                                    onClick={() => onDiscountChange(0)}
                                    className="w-10 h-10 flex items-center justify-center rounded-lg bg-gray-100 text-gray-500 hover:bg-gray-200 hover:text-gray-700 transition-colors"
                                >
                                    <X size={18} />
                                </button>
                            )}
                        </div>
                        
                        {/* Quick Presets - Tighter Grid */}
                        <div className="grid grid-cols-4 gap-2">
                            {DISCOUNTS.map(d => (
                                <EscalatableGate
                                    key={d}
                                    permission={Permission.APPLY_DISCOUNT}
                                    mode="intercept"
                                    description={`${t('checkout.cart.discount')} ${d}%`}
                                    onAuthorized={(user) => handleProtectedDiscount(d === discount ? 0 : d, { id: String(user.id), username: user.username })}
                                >
                                    <button
                                        onClick={() => {
                                            if (hasPermission(Permission.APPLY_DISCOUNT)) {
                                                handleProtectedDiscount(d === discount ? 0 : d);
                                            }
                                        }}
                                        className={`
                                            h-9 rounded-lg text-xs font-bold border transition-all
                                            ${discount === d 
                                                ? 'bg-gray-900 text-white border-gray-900' 
                                                : 'bg-white text-gray-600 border-gray-100 hover:border-gray-300'}
                                        `}
                                    >
                                        {d === 100 ? t('common.discount.free') : `${d}%`}
                                    </button>
                                </EscalatableGate>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
          </div>

          {/* Bottom Actions - Compact Footer */}
          <div className="p-5 bg-white border-t border-gray-100 space-y-3 z-10 shadow-[0_-4px_6px_-1px_rgba(0,0,0,0.05)]">
             <div className="flex gap-3 h-14">
                {/* Cancel Button - Always visible for better UX */}
                {onCancel && (
                    <button
                        onClick={onCancel}
                        className="h-full px-4 rounded-xl font-bold text-gray-600 bg-gray-100 border border-gray-200 hover:bg-gray-200 hover:text-gray-800 active:scale-95 transition-all flex items-center gap-2"
                    >
                        <X size={20} />
                        <span>{t('common.action.cancel')}</span>
                    </button>
                )}

                {/* Delete Button (Only if visible) */}
                {showDelete && onDelete && (
                    <EscalatableGate
                        permission={Permission.VOID_ORDER}
                        mode="intercept"
                        description={t('common.action.delete')}
                        onAuthorized={(user) => handleProtectedDelete({ id: String(user.id), username: user.username })}
                    >
                        <button
                            onClick={() => {
                                if (hasPermission(Permission.VOID_ORDER)) {
                                    handleProtectedDelete();
                                }
                            }}
                            className="h-full w-14 flex items-center justify-center rounded-xl bg-red-50 text-red-500 border border-red-100 hover:bg-red-100 hover:border-red-200 active:scale-95 transition-all"
                        >
                            <Trash2 size={24} />
                        </button>
                    </EscalatableGate>
                )}

                {/* Confirm Button */}
                <button
                    onClick={onConfirm}
                    className="flex-1 h-full bg-gray-900 text-white rounded-xl font-bold text-lg shadow-lg hover:bg-black active:scale-[0.98] transition-all flex items-center justify-between px-4"
                >
                    <div className="flex flex-col items-start leading-none">
                        <span className="text-[10px] opacity-60 font-medium uppercase tracking-wider">{t('checkout.amount.total')}</span>
                        <span>{formatCurrency(finalTotal.toNumber())}</span>
                    </div>
                    <div className="flex items-center gap-2">
                        <span>{confirmLabel || t('common.action.confirm')}</span>
                        <Check size={20} />
                    </div>
                </button>
             </div>
          </div>
        </>
      ) : (
        <div className="flex flex-col h-full animate-in slide-in-from-right duration-200 bg-gray-50">
          {/* Numpad Header - Enhanced */}
          <div className="flex items-center justify-between p-6 bg-white shadow-sm z-10">
            <div>
                <span className="block font-bold text-gray-900 text-lg">
                    {editMode === 'QTY' ? (t('common.label.quantity')) : editMode === 'PRICE' ? (t('pos.cart.unitPrice')) : (t('checkout.cart.discount'))}
                </span>
                <span className="text-xs text-gray-500">
                     {editMode === 'QTY' ? (t('pos.cart.input.quantity')) : editMode === 'PRICE' ? (t('pos.cart.input.price')) : (t('pos.cart.input.discount'))}
                </span>
            </div>
            <button 
                onClick={() => setEditMode('STANDARD')} 
                className="w-10 h-10 flex items-center justify-center bg-gray-100 hover:bg-gray-200 rounded-full transition-colors"
                title={t('common.action.close')}
            >
              <X size={20} className="text-gray-600" />
            </button>
          </div>

          {/* Display Area */}
          <div className="flex-1 flex flex-col p-4">
            <div className="bg-white rounded-2xl shadow-sm border border-gray-200 p-4 mb-4 flex items-center justify-end h-24 shrink-0">
                <span className="text-5xl font-bold text-gray-900 tracking-tight font-mono">
                {editMode === 'PRICE' && !isTyping ? '€' : ''}
                {inputBuffer}
                {editMode === 'DISC' && <span className="text-gray-400 text-3xl ml-2">%</span>}
                </span>
            </div>

            {/* Numpad Container */}
            <div className="flex-1 bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden min-h-0">
                <Numpad
                onNumber={handleNumInput}
                onDelete={() => setInputBuffer(prev => prev.slice(0, -1))}
                onClear={handleClearInput}
                onEnter={handleConfirmInput}
                showDecimal={editMode === 'DISC' || editMode === 'PRICE'}
                className="h-full"
                />
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
