import React, { useState, useMemo } from 'react';
import { FlaskConical, ChevronRight, Check, X, Globe, ShoppingCart, Armchair } from 'lucide-react';
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
  currentRuleId: string | null;
}

interface RuleMatchResult {
  rule: PriceRule;
  matched: boolean;
  reason?: string;
  adjustment: number;
}

export const RulePreviewTester: React.FC<RulePreviewTesterProps> = ({
  rules,
  currentRuleId,
}) => {
  const { t } = useI18n();
  const zones = useZoneStore(state => state.items);
  const products = useProductStore(state => state.items);
  const categories = useCategoryStore(state => state.items);
  const tags = useTagStore(state => state.items);

  const [selectedZone, setSelectedZone] = useState<string>('zone:all');
  const [selectedProduct, setSelectedProduct] = useState<Product | null>(null);
  const [showZonePicker, setShowZonePicker] = useState(false);
  const [showProductPicker, setShowProductPicker] = useState(false);

  // Get zone display name
  const getZoneName = (zoneScope: string): string => {
    if (zoneScope === 'zone:all') return t('settings.price_rule.zone.all');
    if (zoneScope === 'zone:retail') return t('settings.price_rule.zone.retail');
    const zoneId = zoneScope.replace('zone:', '');
    const zone = zones.find(z => z.id === zoneId || z.id === `zone:${zoneId}`);
    return zone?.name || zoneId;
  };

  // Get zone icon
  const getZoneIcon = (zoneScope: string): React.ElementType => {
    if (zoneScope === 'zone:all') return Globe;
    if (zoneScope === 'zone:retail') return ShoppingCart;
    return Armchair;
  };

  // Check if a rule matches the selected zone and product
  const evaluateRule = (rule: PriceRule): RuleMatchResult => {
    // Skip inactive rules
    if (!rule.is_active) {
      return { rule, matched: false, reason: t('settings.price_rule.reason.disabled'), adjustment: 0 };
    }

    // Check zone scope
    if (rule.zone_scope !== 'zone:all') {
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
          if (rule.target !== selectedProduct.category) {
            const cat = categories.find(c => c.id === rule.target);
            return {
              rule,
              matched: false,
              reason: `${t('settings.price_rule.reason.product_mismatch')} (${t('settings.price_rule.reason.only')} ${cat?.name || rule.target} ${t('settings.price_rule.reason.category')})`,
              adjustment: 0,
            };
          }
          break;
        case 'TAG':
          if (!selectedProduct.tags?.some(tag => tag.id === rule.target)) {
            const tagObj = tags.find(tg => tg.id === rule.target);
            return {
              rule,
              matched: false,
              reason: `${t('settings.price_rule.reason.product_mismatch')} (${t('settings.price_rule.reason.only')} ${tagObj?.name || rule.target} ${t('settings.price_rule.reason.tag')})`,
              adjustment: 0,
            };
          }
          break;
        case 'PRODUCT':
          if (rule.target !== selectedProduct.id) {
            const prod = products.find(p => p.id === rule.target);
            return {
              rule,
              matched: false,
              reason: `${t('settings.price_rule.reason.product_mismatch')} (${t('settings.price_rule.reason.only')} ${prod?.name || rule.target})`,
              adjustment: 0,
            };
          }
          break;
      }
    }

    // Check time constraints
    const now = new Date();
    const currentDay = now.getDay();
    const currentTime = `${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}`;

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
      if (!rule.active_days.includes(currentDay)) {
        const dayNames = ['日', '一', '二', '三', '四', '五', '六'];
        const days = rule.active_days.map(d => `周${dayNames[d]}`).join('、');
        return {
          rule,
          matched: false,
          reason: `${t('settings.price_rule.reason.time_mismatch')} (${t('settings.price_rule.reason.only')} ${days})`,
          adjustment: 0,
        };
      }
    }

    // Check time range
    if (rule.active_start_time && rule.active_end_time) {
      if (currentTime < rule.active_start_time || currentTime > rule.active_end_time) {
        return {
          rule,
          matched: false,
          reason: `${t('settings.price_rule.reason.time_mismatch')} (${t('settings.price_rule.reason.only')} ${rule.active_start_time}-${rule.active_end_time})`,
          adjustment: 0,
        };
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
  }, [rules, selectedZone, selectedProduct]);

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
      <div className="grid grid-cols-2 gap-3 mb-4">
        {/* Zone selector */}
        <button
          onClick={() => setShowZonePicker(true)}
          className="flex items-center justify-between p-4 bg-white rounded-xl border border-gray-200 hover:border-blue-300 transition-colors text-left"
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
          className="flex items-center justify-between p-4 bg-white rounded-xl border border-gray-200 hover:border-blue-300 transition-colors text-left"
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
                const isCurrentRule = rule.id === currentRuleId;
                const isDiscount = rule.rule_type === 'DISCOUNT';

                return (
                  <div
                    key={rule.id}
                    className={`flex items-center justify-between text-sm p-2 rounded-lg ${
                      isCurrentRule ? 'bg-blue-50 ring-1 ring-blue-200' : ''
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <span
                        className={`w-2 h-2 rounded-full ${
                          isDiscount ? 'bg-amber-500' : 'bg-purple-500'
                        }`}
                      />
                      <span className={isCurrentRule ? 'font-medium' : ''}>
                        {rule.display_name}
                        {isCurrentRule && ` ← ${t('settings.price_rule.preview.current')}`}
                      </span>
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
                const isCurrentRule = rule.id === currentRuleId;
                const isDiscount = rule.rule_type === 'DISCOUNT';

                return (
                  <div
                    key={rule.id}
                    className={`flex items-center justify-between text-sm text-gray-400 p-2 rounded-lg ${
                      isCurrentRule ? 'bg-gray-100' : ''
                    }`}
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
