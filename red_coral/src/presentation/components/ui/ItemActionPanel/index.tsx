import React, { useState, useEffect } from 'react';
import { Minus, Plus, X, Trash2, Check, Edit2 } from 'lucide-react';
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
    onBasePriceChange,
  } = props;

  const [editMode, setEditMode] = useState<EditMode>('STANDARD');
  const [inputBuffer, setInputBuffer] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const [authorizedUser, setAuthorizedUser] = useState<{ id: string; name: string } | null>(null);

  // --- Calculations ---
  // 1. Base + Options
  const unitPriceBeforeDiscount = Currency.add(basePrice, optionsModifier);
  // 2. Discount Amount (on the enhanced unit price)
  const unitDiscountAmount = Currency.round2(Currency.mul(unitPriceBeforeDiscount, discount / 100));
  // 3. Final Unit Price
  const unitPriceFinal = Currency.sub(unitPriceBeforeDiscount, unitDiscountAmount);
  // 4. Total Line Price
  const finalTotal = Currency.mul(unitPriceFinal, quantity);

  // --- Numpad Handlers ---

  const openNumpad = (mode: EditMode, authorizer?: { id: string; name: string }) => {
    if (mode === 'STANDARD') return;
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
    else if (mode === 'BASE_PRICE') initialValue = basePrice.toFixed(2);

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
        onDiscountChange(val);
      }
    } else if (editMode === 'BASE_PRICE' && onBasePriceChange) {
      const val = parseFloat(inputBuffer);
      if (!isNaN(val) && val >= 0) {
        onBasePriceChange(val);
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
      // Allow more digits for price
      const maxDigits = (editMode === 'PRICE' || editMode === 'BASE_PRICE') ? 6 : 4;
      if (prev.replace('.', '').length >= maxDigits) return prev;
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
    unitPriceBeforeDiscount,
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
    onBasePriceChange,
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
    unitPriceBeforeDiscount,
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

  const QUICK_DISCOUNTS = [5, 10, 20, 50];

  const handleProtectedDelete = (authorizer?: { id: string; name: string }) => {
     if (onDelete) {
        onDelete(authorizer || (currentUser ? { id: currentUser.id, name: currentUser.display_name ?? currentUser.username } : undefined));
     }
  };

  const handleProtectedDiscount = (val: number, authorizer?: { id: string; name: string }) => {
      onDiscountChange(val, authorizer || (currentUser ? { id: currentUser.id, name: currentUser.display_name ?? currentUser.username } : undefined));
  };

  const getEditModeTitle = () => {
    switch (editMode) {
      case 'QTY': return t('common.label.quantity');
      case 'DISC': return t('checkout.cart.discount');
      case 'PRICE': return t('pos.cart.final_price');
      case 'BASE_PRICE': return t('pos.cart.base_price');
      default: return '';
    }
  };

  const getEditModeHint = () => {
    switch (editMode) {
      case 'QTY': return t('pos.cart.input.quantity');
      case 'DISC': return t('pos.cart.input.discount');
      case 'PRICE': return t('pos.cart.input.final_price');
      case 'BASE_PRICE': return t('pos.cart.input.base_price');
      default: return '';
    }
  };

  return (
    <div className="flex flex-col h-full bg-white">
      {editMode === 'STANDARD' ? (
        <>
          <div className="flex-1 overflow-y-auto p-5 space-y-4 custom-scrollbar">

            {/* Price Section */}
            <div className="bg-gray-50 rounded-2xl border border-gray-100 overflow-hidden">
              {/* 原价 (可编辑) */}
              <div
                className="flex items-center justify-between p-4 border-b border-gray-100 cursor-pointer hover:bg-gray-100/50 transition-colors group"
                onClick={() => onBasePriceChange && openNumpad('BASE_PRICE')}
              >
                <span className="text-sm text-gray-600">{t('pos.cart.base_price')}</span>
                <div className="flex items-center gap-2">
                  <span className="text-lg font-semibold text-gray-900 font-mono">
                    {formatCurrency(basePrice)}
                  </span>
                  {onBasePriceChange && (
                    <Edit2 size={14} className="text-gray-400 opacity-0 group-hover:opacity-100 transition-opacity" />
                  )}
                </div>
              </div>

              {/* 属性加价 (如果有) */}
              {optionsModifier !== 0 && (
                <div className="flex items-center justify-between p-4 border-b border-gray-100 bg-orange-50/50">
                  <span className="text-sm text-gray-600">{t('pos.product.options')}</span>
                  <span className={`text-lg font-semibold font-mono ${optionsModifier > 0 ? 'text-orange-600' : 'text-green-600'}`}>
                    {optionsModifier > 0 ? '+' : ''}{formatCurrency(optionsModifier)}
                  </span>
                </div>
              )}

              {/* 折扣 (可编辑) */}
              <div className="p-4 border-b border-gray-100 space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-gray-600">{t('checkout.cart.discount')}</span>
                  <EscalatableGate
                    permission={Permission.ORDERS_DISCOUNT}
                    mode="intercept"
                    description={t('pos.cart.enter_discount')}
                    onAuthorized={(user) => openNumpad('DISC', { id: user.id, name: user.display_name })}
                  >
                    <button
                      onClick={() => {
                        if (hasPermission(Permission.ORDERS_DISCOUNT)) {
                          openNumpad('DISC');
                        }
                      }}
                      className={`flex items-center gap-2 px-3 py-1.5 rounded-lg font-semibold transition-all ${
                        discount > 0
                          ? 'bg-orange-100 text-orange-600 hover:bg-orange-200'
                          : 'bg-gray-100 text-gray-500 hover:bg-gray-200'
                      }`}
                    >
                      <span className="text-lg font-mono">{discount}%</span>
                      <Edit2 size={14} />
                    </button>
                  </EscalatableGate>
                </div>

                {/* 快速折扣按钮 */}
                <div className="flex gap-2">
                  {QUICK_DISCOUNTS.map(d => (
                    <EscalatableGate
                      key={d}
                      permission={Permission.ORDERS_DISCOUNT}
                      mode="intercept"
                      description={`${t('checkout.cart.discount')} ${d}%`}
                      onAuthorized={(user) => handleProtectedDiscount(d === discount ? 0 : d, { id: user.id, name: user.display_name })}
                    >
                      <button
                        onClick={() => {
                          if (hasPermission(Permission.ORDERS_DISCOUNT)) {
                            handleProtectedDiscount(d === discount ? 0 : d);
                          }
                        }}
                        className={`flex-1 h-10 rounded-lg text-sm font-bold transition-all ${
                          discount === d
                            ? 'bg-gray-900 text-white'
                            : 'bg-white text-gray-600 border border-gray-200 hover:border-gray-400'
                        }`}
                      >
                        {d}%
                      </button>
                    </EscalatableGate>
                  ))}
                  {discount > 0 && (
                    <button
                      onClick={() => onDiscountChange(0)}
                      className="w-10 h-10 flex items-center justify-center rounded-lg bg-gray-100 text-gray-500 hover:bg-gray-200 transition-colors"
                    >
                      <X size={18} />
                    </button>
                  )}
                </div>
              </div>

              {/* 最终价格 (可编辑) */}
              <EscalatableGate
                permission={Permission.ORDERS_DISCOUNT}
                mode="intercept"
                description={t('pos.cart.edit_final_price')}
                onAuthorized={(user) => openNumpad('PRICE', { id: user.id, name: user.display_name })}
              >
                <div
                  className="flex items-center justify-between p-4 cursor-pointer hover:bg-blue-50/50 transition-colors group"
                  onClick={() => {
                    if (hasPermission(Permission.ORDERS_DISCOUNT)) {
                      openNumpad('PRICE');
                    }
                  }}
                >
                  <span className="text-sm font-semibold text-gray-900">{t('pos.cart.final_price')}</span>
                  <div className="flex items-center gap-2">
                    <span className="text-2xl font-bold text-blue-600 font-mono">
                      {formatCurrency(unitPriceFinal.toNumber())}
                    </span>
                    <Edit2 size={14} className="text-gray-400 opacity-0 group-hover:opacity-100 transition-opacity" />
                  </div>
                </div>
              </EscalatableGate>
            </div>

            {/* Quantity Section */}
            <div className="space-y-2">
              <label className="text-xs font-bold text-gray-500 uppercase tracking-wider px-1">
                {t('common.label.quantity')}
              </label>
              <div className="flex items-center gap-3">
                <button
                  onClick={() => incrementQty(-1)}
                  className="w-14 h-14 bg-white border border-gray-200 shadow-sm rounded-xl flex items-center justify-center text-gray-600 active:scale-95 hover:border-gray-300 transition-all"
                >
                  <Minus size={24} />
                </button>
                <button
                  onClick={() => openNumpad('QTY')}
                  className="flex-1 h-14 bg-gray-50 rounded-xl border border-transparent hover:bg-white hover:border-blue-200 hover:shadow-sm transition-all flex items-center justify-center group cursor-pointer"
                >
                  <span className="text-3xl font-bold text-gray-800 group-hover:text-blue-600 transition-colors">
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

          </div>

          {/* Bottom Actions */}
          <div className="p-5 bg-white border-t border-gray-100 space-y-3 z-10 shadow-up">
             <div className="flex gap-3 h-14">
                {/* Cancel Button */}
                {onCancel && (
                    <button
                        onClick={onCancel}
                        className="h-full px-4 rounded-xl font-bold text-gray-600 bg-gray-100 border border-gray-200 hover:bg-gray-200 hover:text-gray-800 active:scale-95 transition-all flex items-center gap-2"
                    >
                        <X size={20} />
                        <span>{t('common.action.cancel')}</span>
                    </button>
                )}

                {/* Delete Button */}
                {showDelete && onDelete && (
                    <EscalatableGate
                        permission={Permission.ORDERS_CANCEL_ITEM}
                        mode="intercept"
                        description={t('common.action.delete')}
                        onAuthorized={(user) => handleProtectedDelete({ id: user.id, name: user.display_name })}
                    >
                        <button
                            onClick={() => {
                                if (hasPermission(Permission.ORDERS_CANCEL_ITEM)) {
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
                        <span className="text-[0.625rem] opacity-60 font-medium uppercase tracking-wider">{t('checkout.amount.total')}</span>
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
          {/* Numpad Header */}
          <div className="flex items-center justify-between p-6 bg-white shadow-sm z-10">
            <div>
                <span className="block font-bold text-gray-900 text-lg">
                    {getEditModeTitle()}
                </span>
                <span className="text-xs text-gray-500">
                     {getEditModeHint()}
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
                {(editMode === 'PRICE' || editMode === 'BASE_PRICE') && '€'}
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
                  showDecimal={editMode === 'DISC' || editMode === 'PRICE' || editMode === 'BASE_PRICE'}
                  className="h-full"
                />
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
