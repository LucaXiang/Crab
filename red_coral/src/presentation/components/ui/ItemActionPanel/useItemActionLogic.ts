import { useState, useEffect } from 'react';
import { Currency } from '@/utils/currency';
import { ItemActionPanelProps, EditMode } from './types';

export const useItemActionLogic = (props: ItemActionPanelProps) => {
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
  const [authorizedUser, setAuthorizedUser] = useState<{ id: number; name: string } | null>(null);

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

  const openNumpad = (mode: EditMode, authorizer?: { id: number; name: string }) => {
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
