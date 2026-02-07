import React, { useState, useMemo } from 'react';
import { FlaskConical, ChevronRight, Check, X, Globe, ShoppingCart, Armchair, Clock } from 'lucide-react';
import type { PriceRule, Product } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';
import { useZoneStore } from '@/features/zone/store';
import { useProductStore } from '@/core/stores/resources';
import { useCategoryStore } from '@/features/category/store';
import { useTagStore } from '@/features/tag/store';
import { formatCurrency } from '@/utils/currency';
import { ZonePicker } from './ZonePicker';
import { ProductPicker } from './ProductPicker';

interface RulePreviewTesterProps {
  rules: PriceRule[];
}

interface RuleMatchResult {
  rule: PriceRule;
  matched: boolean;
  reason?: string;
  adjustment: number;
}

// Day keys for i18n (Sunday = 0)
const DAY_KEYS = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'] as const;

export const RulePreviewTester: React.FC<RulePreviewTesterProps> = ({
  rules,
}) => {
  const { t } = useI18n();
  const zones = useZoneStore(state => state.items);
  const products = useProductStore(state => state.items);
  const categories = useCategoryStore(state => state.items);
  const tags = useTagStore(state => state.items);

  const [selectedZone, setSelectedZone] = useState<string>('all');
  const [selectedProduct, setSelectedProduct] = useState<Product | null>(null);
  const [showZonePicker, setShowZonePicker] = useState(false);
  const [showProductPicker, setShowProductPicker] = useState(false);

  // Time mode: 'ignore' | 'current' | 'custom'
  const [timeMode, setTimeMode] = useState<'ignore' | 'current' | 'custom'>('ignore');
  const [selectedDay, setSelectedDay] = useState<number>(new Date().getDay());
  const [selectedTime, setSelectedTime] = useState<string>(
    `${new Date().getHours().toString().padStart(2, '0')}:00`
  );

  // Get zone display name
  const getZoneName = (zoneScope: string): string => {
    if (zoneScope === 'all') return t('settings.price_rule.zone.all');
    if (zoneScope === 'retail') return t('settings.price_rule.zone.retail');
    const zone = zones.find(z => String(z.id) === zoneScope);
    return zone?.name || zoneScope;
  };

  // Get zone icon
  const getZoneIcon = (zoneScope: string): React.ElementType => {
    if (zoneScope === 'all') return Globe;
    if (zoneScope === 'retail') return ShoppingCart;
    return Armchair;
  };

  // Check if a rule matches the selected zone and product
  const evaluateRule = (rule: PriceRule): RuleMatchResult => {
    // Skip inactive rules
    if (!rule.is_active) {
      return { rule, matched: false, reason: t('settings.price_rule.reason.disabled'), adjustment: 0 };
    }

    // Check zone scope
    if (rule.zone_scope !== 'all') {
      if (rule.zone_scope !== selectedZone) {
        return {
          rule,
          matched: false,
          reason: `${t('settings.price_rule.reason.zone_mismatch')} (${t('settings.price_rule.reason.only')} ${getZoneName(rule.zone_scope)})`,
          adjustment: 0,
        };
      }
    }

    // Check product scope (only if product is selected)
    if (selectedProduct) {
      switch (rule.product_scope) {
        case 'GLOBAL':
          // Matches all products
          break;
        case 'CATEGORY':
          if (rule.target_id !== selectedProduct.category_id) {
            const cat = categories.find(c => c.id === rule.target_id);
            return {
              rule,
              matched: false,
              reason: `${t('settings.price_rule.reason.product_mismatch')} (${t('settings.price_rule.reason.only')} ${cat?.name || rule.target_id} ${t('settings.price_rule.reason.category')})`,
              adjustment: 0,
            };
          }
          break;
        case 'TAG':
          if (!selectedProduct.tags?.some(tag => tag.id === rule.target_id)) {
            const tagObj = tags.find(tg => tg.id === rule.target_id);
            return {
              rule,
              matched: false,
              reason: `${t('settings.price_rule.reason.product_mismatch')} (${t('settings.price_rule.reason.only')} ${tagObj?.name || rule.target_id} ${t('settings.price_rule.reason.tag')})`,
              adjustment: 0,
            };
          }
          break;
        case 'PRODUCT':
          if (rule.target_id !== selectedProduct.id) {
            const prod = products.find(p => p.id === rule.target_id);
            return {
              rule,
              matched: false,
              reason: `${t('settings.price_rule.reason.product_mismatch')} (${t('settings.price_rule.reason.only')} ${prod?.name || rule.target_id})`,
              adjustment: 0,
            };
          }
          break;
      }
    }

    // Check time constraints (skip if timeMode === 'ignore')
    if (timeMode !== 'ignore') {
      const now = new Date();
      const testDay = timeMode === 'custom' ? selectedDay : now.getDay();
      const testTime = timeMode === 'custom'
        ? selectedTime
        : `${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}`;

      // Check date range
      if (rule.valid_from && Date.now() < rule.valid_from) {
        return {
          rule,
          matched: false,
          reason: t('settings.price_rule.reason.not_started'),
          adjustment: 0,
        };
      }
      if (rule.valid_until && Date.now() > rule.valid_until) {
        return {
          rule,
          matched: false,
          reason: t('settings.price_rule.reason.expired'),
          adjustment: 0,
        };
      }

      // Check active days
      if (rule.active_days && rule.active_days.length > 0 && rule.active_days.length < 7) {
        if (!rule.active_days.includes(testDay)) {
          const days = rule.active_days.map(d => t(`calendar.days.${DAY_KEYS[d]}`)).join('、');
          return {
            rule,
            matched: false,
            reason: `${t('settings.price_rule.reason.time_mismatch')} (${t('settings.price_rule.reason.only')} ${days})`,
            adjustment: 0,
          };
        }
      }

      // Check time range (handle cross-midnight)
      if (rule.active_start_time && rule.active_end_time) {
        const start = rule.active_start_time;
        const end = rule.active_end_time;

        let inRange: boolean;
        if (end > start) {
          // Normal range (e.g., 09:00 - 18:00)
          inRange = testTime >= start && testTime < end;
        } else {
          // Cross-midnight range (e.g., 21:00 - 04:00)
          inRange = testTime >= start || testTime < end;
        }

        if (!inRange) {
          return {
            rule,
            matched: false,
            reason: `${t('settings.price_rule.reason.time_mismatch')} (${t('settings.price_rule.reason.only')} ${start}-${end})`,
            adjustment: 0,
          };
        }
      }
    }

    // Rule matched! Calculate adjustment
    const basePrice = selectedProduct
      ? (selectedProduct.specs?.[0]?.price ?? 0)
      : 10; // Default price for calculation

    let adjustment = 0;
    if (rule.adjustment_type === 'PERCENTAGE') {
      adjustment = basePrice * (rule.adjustment_value / 100);
    } else {
      adjustment = rule.adjustment_value;
    }

    if (rule.rule_type === 'DISCOUNT') {
      adjustment = -adjustment;
    }

    return { rule, matched: true, adjustment };
  };

  // Evaluate all rules
  const matchResults = useMemo(() => {
    return rules.map(evaluateRule);
  }, [rules, selectedZone, selectedProduct, timeMode, selectedDay, selectedTime]);

  const matchedRules = matchResults.filter(r => r.matched);
  const unmatchedRules = matchResults.filter(r => !r.matched);

  // Calculate final price
  const basePrice = selectedProduct
    ? (selectedProduct.specs?.[0]?.price ?? 0)
    : 0;

  // Simple calculation (ignoring stacking rules for preview)
  const totalAdjustment = matchedRules.reduce((sum, r) => sum + r.adjustment, 0);
  const finalPrice = basePrice + totalAdjustment;

  const ZoneIcon = getZoneIcon(selectedZone);

  return (
    <div className="bg-gray-50 rounded-xl p-4">
      <div className="flex items-center gap-2 mb-4">
        <FlaskConical size={16} className="text-gray-500" />
        <span className="text-sm font-medium text-gray-700">
          {t('settings.price_rule.preview.title')}
        </span>
      </div>

      {/* Selection buttons */}
      <div className="grid grid-cols-2 gap-3 mb-3">
        {/* Zone selector */}
        <button
          onClick={() => setShowZonePicker(true)}
          className="flex items-center justify-between p-4 bg-white rounded-xl border border-gray-200 hover:border-teal-300 transition-colors text-left"
        >
          <div>
            <div className="text-xs text-gray-500 mb-1">
              {t('settings.price_rule.preview.zone')}
            </div>
            <div className="flex items-center gap-2 font-medium text-gray-900">
              <ZoneIcon size={16} />
              {getZoneName(selectedZone)}
            </div>
          </div>
          <ChevronRight size={16} className="text-gray-400" />
        </button>

        {/* Product selector */}
        <button
          onClick={() => setShowProductPicker(true)}
          className="flex items-center justify-between p-4 bg-white rounded-xl border border-gray-200 hover:border-teal-300 transition-colors text-left"
        >
          <div>
            <div className="text-xs text-gray-500 mb-1">
              {t('settings.price_rule.preview.product')}
            </div>
            <div className="font-medium text-gray-900 truncate">
              {selectedProduct ? (
                <span>
                  {selectedProduct.name} · {formatCurrency(basePrice)}
                </span>
              ) : (
                <span className="text-gray-400">
                  {t('settings.price_rule.preview.select_product')}
                </span>
              )}
            </div>
          </div>
          <ChevronRight size={16} className="text-gray-400" />
        </button>
      </div>

      {/* Time mode selector */}
      <div className="mb-4 p-3 bg-white rounded-xl border border-gray-200">
        <div className="flex items-center gap-2 text-sm text-gray-600 mb-2">
          <Clock size={14} />
          <span>{t('settings.price_rule.preview.test_time')}</span>
        </div>

        {/* Three-option selector */}
        <div className="flex gap-1 p-1 bg-gray-100 rounded-lg mb-2">
          {(['ignore', 'current', 'custom'] as const).map(mode => (
            <button
              key={mode}
              onClick={() => setTimeMode(mode)}
              className={`flex-1 py-1.5 px-2 text-xs font-medium rounded-md transition-colors ${
                timeMode === mode
                  ? 'bg-white text-gray-900 shadow-sm'
                  : 'text-gray-500 hover:text-gray-700'
              }`}
            >
              {t(`settings.price_rule.preview.${mode}_time`)}
            </button>
          ))}
        </div>

        {timeMode === 'custom' && (
          <div className="flex items-center gap-3">
            {/* Day selector */}
            <select
              value={selectedDay}
              onChange={(e) => setSelectedDay(Number(e.target.value))}
              className="flex-1 px-3 py-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-teal-500"
            >
              {[1, 2, 3, 4, 5, 6, 0].map(day => (
                <option key={day} value={day}>
                  {t(`calendar.days.${DAY_KEYS[day]}`)}
                </option>
              ))}
            </select>

            {/* Time selector */}
            <input
              type="time"
              value={selectedTime}
              onChange={(e) => setSelectedTime(e.target.value)}
              className="px-3 py-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-teal-500"
            />
          </div>
        )}
      </div>

      {/* Results */}
      {selectedProduct && (
        <div className="bg-white rounded-xl p-4 space-y-3">
          {/* Base price */}
          <div className="flex items-center justify-between text-sm">
            <span className="text-gray-600">{t('settings.price_rule.preview.base_price')}</span>
            <span className="font-medium text-gray-900">{formatCurrency(basePrice)}</span>
          </div>

          <div className="border-t border-gray-100" />

          {/* Matched rules */}
          {matchedRules.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center gap-1 text-xs text-green-600">
                <Check size={12} />
                {t('settings.price_rule.preview.matched')}:
              </div>
              {matchedRules.map(({ rule, adjustment }) => {
                const isDiscount = rule.rule_type === 'DISCOUNT';

                return (
                  <div
                    key={rule.id}
                    className="flex items-center justify-between text-sm p-2 rounded-lg"
                  >
                    <div className="flex items-center gap-2">
                      <span
                        className={`w-2 h-2 rounded-full ${
                          isDiscount ? 'bg-amber-500' : 'bg-purple-500'
                        }`}
                      />
                      <span>{rule.display_name}</span>
                    </div>
                    <span
                      className={`font-medium ${
                        isDiscount ? 'text-amber-600' : 'text-purple-600'
                      }`}
                    >
                      {adjustment >= 0 ? '+' : ''}{formatCurrency(adjustment)}
                    </span>
                  </div>
                );
              })}
            </div>
          )}

          {/* Unmatched rules */}
          {unmatchedRules.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center gap-1 text-xs text-gray-400">
                <X size={12} />
                {t('settings.price_rule.preview.unmatched')}:
              </div>
              {unmatchedRules.slice(0, 5).map(({ rule, reason }) => {
                const isDiscount = rule.rule_type === 'DISCOUNT';

                return (
                  <div
                    key={rule.id}
                    className="flex items-center justify-between text-sm text-gray-400 p-2 rounded-lg"
                  >
                    <div className="flex items-center gap-2">
                      <span
                        className={`w-2 h-2 rounded-full ${
                          isDiscount ? 'bg-amber-200' : 'bg-purple-200'
                        }`}
                      />
                      <span>{rule.display_name}</span>
                    </div>
                    <span className="text-xs">{reason}</span>
                  </div>
                );
              })}
              {unmatchedRules.length > 5 && (
                <div className="text-xs text-gray-400 text-center">
                  +{unmatchedRules.length - 5} {t('settings.price_rule.preview.more_rules')}
                </div>
              )}
            </div>
          )}

          <div className="border-t border-gray-100" />

          {/* Final price */}
          <div className="flex items-center justify-between">
            <span className="font-medium text-gray-900">
              {t('settings.price_rule.preview.final_price')}
            </span>
            <span className="text-xl font-bold text-gray-900">{formatCurrency(finalPrice)}</span>
          </div>
        </div>
      )}

      {/* Pickers */}
      <ZonePicker
        isOpen={showZonePicker}
        selectedZone={selectedZone}
        onSelect={setSelectedZone}
        onClose={() => setShowZonePicker(false)}
      />

      <ProductPicker
        isOpen={showProductPicker}
        selectedProductId={selectedProduct?.id || null}
        onSelect={setSelectedProduct}
        onClose={() => setShowProductPicker(false)}
      />
    </div>
  );
};
