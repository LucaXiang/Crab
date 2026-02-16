import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { HeldOrder, Permission } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { X, Percent, Tag } from 'lucide-react';
import { formatCurrency } from '@/utils/currency';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { Numpad } from '@/presentation/components/ui/Numpad';

type AdjustmentInputType = 'percent' | 'fixed';

const QUICK_PERCENT_VALUES = [5, 10, 15, 20];

export interface AdjustmentConfig {
  type: 'discount' | 'surcharge';
  /** Header icon component */
  icon: React.ElementType;
  /** Tailwind color prefix (e.g. 'orange', 'purple') */
  color: string;
  /** Get existing values from order */
  getExisting: (order: HeldOrder) => { percent?: number | null; fixed?: number | null };
  /** Apply the adjustment */
  applyFn: (orderId: string, payload: {
    percent?: number;
    fixed?: number;
    authorizer?: { id: number; name: string };
  }) => Promise<void>;
  /** i18n key prefix (e.g. 'checkout.order_discount') */
  i18nPrefix: string;
}

interface PriceAdjustmentModalProps {
  isOpen: boolean;
  order: HeldOrder;
  onClose: () => void;
  config: AdjustmentConfig;
}

// Color scheme mapping
const COLOR_SCHEMES: Record<string, {
  headerBg: string; iconBg: string; iconText: string; hoverBg: string;
  borderActive: string; bgActive: string; textActive: string;
  inputBorder: string; inputBg: string; cursorBg: string; symbolText: string;
  quickBg: string; quickBorder: string; quickText: string; quickHover: string;
  clearText: string; clearBg: string; clearHover: string; clearBorder: string;
  confirmBg: string; confirmHover: string; confirmShadow: string;
}> = {
  orange: {
    headerBg: 'bg-orange-50', iconBg: 'bg-orange-100', iconText: 'text-orange-600', hoverBg: 'hover:bg-orange-100',
    borderActive: 'border-orange-500', bgActive: 'bg-orange-50', textActive: 'text-orange-700',
    inputBorder: 'border-orange-200', inputBg: 'bg-orange-500', cursorBg: 'bg-orange-400', symbolText: 'text-orange-500',
    quickBg: 'bg-orange-50', quickBorder: 'border-orange-200', quickText: 'text-orange-600', quickHover: 'hover:bg-orange-100',
    clearText: 'text-orange-600', clearBg: 'bg-orange-50', clearHover: 'hover:bg-orange-100', clearBorder: 'border-orange-200',
    confirmBg: 'bg-orange-500', confirmHover: 'hover:bg-orange-600', confirmShadow: 'hover:shadow-orange-500/30',
  },
  purple: {
    headerBg: 'bg-purple-50', iconBg: 'bg-purple-100', iconText: 'text-purple-600', hoverBg: 'hover:bg-purple-100',
    borderActive: 'border-purple-500', bgActive: 'bg-purple-50', textActive: 'text-purple-700',
    inputBorder: 'border-purple-200', inputBg: 'bg-purple-500', cursorBg: 'bg-purple-400', symbolText: 'text-purple-500',
    quickBg: 'bg-purple-50', quickBorder: 'border-purple-200', quickText: 'text-purple-600', quickHover: 'hover:bg-purple-100',
    clearText: 'text-purple-600', clearBg: 'bg-purple-50', clearHover: 'hover:bg-purple-100', clearBorder: 'border-purple-200',
    confirmBg: 'bg-purple-500', confirmHover: 'hover:bg-purple-600', confirmShadow: 'hover:shadow-purple-500/30',
  },
};

export const PriceAdjustmentModal: React.FC<PriceAdjustmentModalProps> = ({
  isOpen,
  order,
  onClose,
  config,
}) => {
  const { t } = useI18n();
  const cs = COLOR_SCHEMES[config.color] ?? COLOR_SCHEMES.orange;

  const existing = config.getExisting(order);
  const hasExisting = !!(existing.percent || existing.fixed);

  const [inputType, setInputType] = useState<AdjustmentInputType>(
    existing.fixed ? 'fixed' : 'percent'
  );
  const [value, setValue] = useState('');
  const [isSelected, setIsSelected] = useState(true);
  const [isProcessing, setIsProcessing] = useState(false);

  useEffect(() => {
    if (isOpen) {
      const ex = config.getExisting(order);
      if (ex.fixed) {
        setInputType('fixed');
        setValue(String(ex.fixed));
      } else if (ex.percent) {
        setInputType('percent');
        setValue(String(ex.percent));
      } else {
        setInputType('percent');
        setValue('');
      }
      setIsSelected(true);
      setIsProcessing(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  const parsedValue = parseFloat(value) || 0;

  const canConfirm = useMemo(() => {
    if (isProcessing) return false;
    if (parsedValue <= 0) return false;
    if (inputType === 'percent' && parsedValue > 100) return false;
    return true;
  }, [isProcessing, parsedValue, inputType]);

  const handleNumPress = useCallback((num: string) => {
    if (isSelected) {
      setIsSelected(false);
      setValue(num === '.' ? '0.' : num);
      return;
    }
    setValue((prev) => {
      if (num === '.' && prev.includes('.')) return prev;
      if (prev.includes('.') && prev.split('.')[1].length >= 2) return prev;
      if (prev === '0' && num !== '.') return num;
      return prev + num;
    });
  }, [isSelected]);

  const handleDelete = useCallback(() => {
    setIsSelected(false);
    setValue((prev) => {
      const newVal = prev.slice(0, -1);
      return newVal || '0';
    });
  }, []);

  const handleNumpadClear = useCallback(() => {
    setValue('0');
    setIsSelected(true);
  }, []);

  const handleQuickValue = useCallback((val: number) => {
    setValue(String(val));
    setIsSelected(false);
  }, []);

  const handleTypeChange = useCallback((type: AdjustmentInputType) => {
    setInputType(type);
    setValue('');
    setIsSelected(true);
  }, []);

  const handleApply = async (authorizer?: { id: number; name: string }) => {
    if (!canConfirm) return;
    setIsProcessing(true);
    try {
      await config.applyFn(order.order_id, {
        percent: inputType === 'percent' ? parsedValue : undefined,
        fixed: inputType === 'fixed' ? parsedValue : undefined,
        authorizer,
      });
      toast.success(t(`${config.i18nPrefix}.title`));
      onClose();
    } catch (err) {
      logger.error(`Apply ${config.type} failed`, err);
      toast.error(String(err));
    } finally {
      setIsProcessing(false);
    }
  };

  const handleClear = async (authorizer?: { id: number; name: string }) => {
    setIsProcessing(true);
    try {
      await config.applyFn(order.order_id, { authorizer });
      toast.success(t(`${config.i18nPrefix}.clear`));
      onClose();
    } catch (err) {
      logger.error(`Clear ${config.type} failed`, err);
      toast.error(String(err));
    } finally {
      setIsProcessing(false);
    }
  };

  if (!isOpen) return null;

  const Icon = config.icon;

  return (
    <div className="fixed inset-0 z-80 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-lg w-full overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className={`shrink-0 px-6 py-4 border-b border-gray-100 flex justify-between items-center ${cs.headerBg}`}>
          <div className="flex items-center gap-3">
            <div className={`p-2 ${cs.iconBg} rounded-full ${cs.iconText}`}>
              <Icon size={24} />
            </div>
            <h2 className="text-xl font-bold text-gray-800">{t(`${config.i18nPrefix}.title`)}</h2>
          </div>
          <button onClick={onClose} className={`p-2 ${cs.hoverBg} rounded-full transition-colors text-gray-500`}>
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-5 overflow-y-auto space-y-4">
          <div className="bg-gray-50 rounded-xl p-3">
            <div className="flex justify-between text-sm">
              <span className="text-gray-600">{t('checkout.amount.total')}</span>
              <span className="font-medium text-gray-900">{formatCurrency(order.original_total)}</span>
            </div>
          </div>

          {/* Type Selection */}
          <div className="grid grid-cols-2 gap-3">
            <button
              onClick={() => handleTypeChange('percent')}
              className={`p-3 rounded-xl border-2 text-left transition-all flex items-center gap-3 ${
                inputType === 'percent'
                  ? `${cs.borderActive} ${cs.bgActive}`
                  : `border-gray-100 hover:border-${config.color}-200 hover:bg-gray-50`
              }`}
            >
              <div className={`p-2 rounded-lg ${inputType === 'percent' ? `${cs.iconBg} ${cs.iconText}` : 'bg-gray-100 text-gray-500'}`}>
                <Percent size={20} />
              </div>
              <span className={`font-medium ${inputType === 'percent' ? cs.textActive : 'text-gray-700'}`}>
                {t(`${config.i18nPrefix}.type_percent`)}
              </span>
            </button>
            <button
              onClick={() => handleTypeChange('fixed')}
              className={`p-3 rounded-xl border-2 text-left transition-all flex items-center gap-3 ${
                inputType === 'fixed'
                  ? `${cs.borderActive} ${cs.bgActive}`
                  : `border-gray-100 hover:border-${config.color}-200 hover:bg-gray-50`
              }`}
            >
              <div className={`p-2 rounded-lg ${inputType === 'fixed' ? `${cs.iconBg} ${cs.iconText}` : 'bg-gray-100 text-gray-500'}`}>
                <Tag size={20} />
              </div>
              <span className={`font-medium ${inputType === 'fixed' ? cs.textActive : 'text-gray-700'}`}>
                {t(`${config.i18nPrefix}.type_fixed`)}
              </span>
            </button>
          </div>

          {/* Value Display */}
          <div className="space-y-2">
            <div className={`h-14 bg-white rounded-xl flex items-center justify-center px-4 border-2 ${cs.inputBorder} shadow-sm`}>
              <div className="flex items-center">
                {inputType === 'fixed' && (
                  <span className={`${cs.symbolText} mr-2 text-xl font-bold`}>€</span>
                )}
                <span
                  className={`text-3xl font-mono font-bold px-2 rounded transition-colors ${
                    isSelected ? `${cs.inputBg} text-white` : 'text-gray-800'
                  }`}
                >
                  {value || '0'}
                </span>
                {inputType === 'percent' && (
                  <span className={`${cs.symbolText} ml-1 text-xl font-bold`}>%</span>
                )}
                {!isSelected && (
                  <span className={`animate-pulse ml-0.5 w-0.5 h-7 ${cs.cursorBg} rounded`} />
                )}
              </div>
            </div>
            {inputType === 'percent' && parsedValue > 100 && (
              <p className="text-sm text-red-500 text-center">0-100%</p>
            )}
          </div>

          {/* Quick Value Buttons */}
          {inputType === 'percent' && (
            <div className="grid grid-cols-4 gap-2">
              {QUICK_PERCENT_VALUES.map((val) => (
                <button
                  key={val}
                  onClick={() => handleQuickValue(val)}
                  disabled={isProcessing}
                  className={`h-10 ${cs.quickBg} border ${cs.quickBorder} ${cs.quickText} font-semibold rounded-lg ${cs.quickHover} active:scale-95 transition-all disabled:opacity-50`}
                >
                  {val}%
                </button>
              ))}
            </div>
          )}

          <Numpad
            onNumber={handleNumPress}
            onDelete={handleDelete}
            onClear={handleNumpadClear}
            showEnter={false}
            showDecimal={inputType === 'fixed'}
          />
        </div>

        {/* Footer */}
        <div className="shrink-0 p-5 border-t border-gray-100 flex flex-col gap-3 bg-gray-50">
          {hasExisting && (
            <EscalatableGate
              permission={Permission.ORDERS_DISCOUNT}
              mode="intercept"
              description={t(`${config.i18nPrefix}.auth_required`)}
              onAuthorized={(user) => handleClear({ id: user.id, name: user.display_name })}
            >
              <button
                disabled={isProcessing}
                className={`w-full py-3 px-4 rounded-xl font-bold ${cs.clearText} ${cs.clearBg} ${cs.clearHover} border ${cs.clearBorder} transition-colors disabled:opacity-50`}
              >
                {t(`${config.i18nPrefix}.clear`)}
              </button>
            </EscalatableGate>
          )}
          <div className="flex gap-3">
            <button
              onClick={onClose}
              className="flex-1 py-3 px-4 rounded-xl font-bold text-gray-600 hover:bg-gray-200 transition-colors"
            >
              {t('common.action.cancel')}
            </button>
            <EscalatableGate
              permission={Permission.ORDERS_DISCOUNT}
              mode="intercept"
              description={t(`${config.i18nPrefix}.auth_required`)}
              onAuthorized={(user) => handleApply({ id: user.id, name: user.display_name })}
            >
              <button
                disabled={!canConfirm}
                className={`flex-1 py-3 px-4 rounded-xl font-bold text-white transition-all shadow-lg ${
                  canConfirm
                    ? `${cs.confirmBg} ${cs.confirmHover} ${cs.confirmShadow} hover:-translate-y-0.5`
                    : 'bg-gray-300 cursor-not-allowed'
                }`}
              >
                {t(`${config.i18nPrefix}.confirm`)}
              </button>
            </EscalatableGate>
          </div>
        </div>
      </div>
    </div>
  );
};
