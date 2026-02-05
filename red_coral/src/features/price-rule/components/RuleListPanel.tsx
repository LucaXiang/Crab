import React from 'react';
import { Globe, Package, Tag, Layers, ShoppingCart, Armchair } from 'lucide-react';
import type { PriceRule } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';
import { useZoneStore } from '@/features/zone/store';

interface RuleListPanelProps {
  rules: PriceRule[];
  selectedRuleId: string | null;
  onSelectRule: (id: string) => void;
  searchQuery: string;
}

// Scope icons
const PRODUCT_SCOPE_ICONS: Record<string, React.ElementType> = {
  GLOBAL: Globe,
  CATEGORY: Layers,
  TAG: Tag,
  PRODUCT: Package,
};

const ZONE_ICONS: Record<string, React.ElementType> = {
  'zone:all': Globe,
  'zone:retail': ShoppingCart,
};

export const RuleListPanel: React.FC<RuleListPanelProps> = ({
  rules,
  selectedRuleId,
  onSelectRule,
  searchQuery,
}) => {
  const { t } = useI18n();
  const zones = useZoneStore(state => state.items);

  // Filter rules by search query
  const filteredRules = React.useMemo(() => {
    if (!searchQuery.trim()) return rules;
    const q = searchQuery.toLowerCase();
    return rules.filter(
      rule =>
        rule.name.toLowerCase().includes(q) ||
        rule.display_name.toLowerCase().includes(q)
    );
  }, [rules, searchQuery]);

  // Get zone display name
  const getZoneName = (zoneScope: string): string => {
    if (zoneScope === 'zone:all') return t('settings.price_rule.zone.all');
    if (zoneScope === 'zone:retail') return t('settings.price_rule.zone.retail');
    // Extract zone ID from "zone:xxx" format
    const zoneId = zoneScope.replace('zone:', '');
    const zone = zones.find(z => z.id === zoneId || z.id === `zone:${zoneId}`);
    return zone?.name || zoneId;
  };

  // Get zone icon
  const getZoneIcon = (zoneScope: string): React.ElementType => {
    if (ZONE_ICONS[zoneScope]) return ZONE_ICONS[zoneScope];
    return Armchair; // Default for specific zones
  };

  // Format time summary
  const formatTimeSummary = (rule: PriceRule): string => {
    const parts: string[] = [];

    // Days of week
    if (rule.active_days && rule.active_days.length > 0 && rule.active_days.length < 7) {
      const dayNames = ['日', '一', '二', '三', '四', '五', '六'];
      const days = rule.active_days.map(d => dayNames[d]).join('');
      parts.push(`周${days}`);
    }

    // Time range
    if (rule.active_start_time && rule.active_end_time) {
      parts.push(`${rule.active_start_time}-${rule.active_end_time}`);
    }

    // Date range
    if (rule.valid_from || rule.valid_until) {
      const from = rule.valid_from ? new Date(rule.valid_from).toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' }) : '';
      const until = rule.valid_until ? new Date(rule.valid_until).toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' }) : '';
      if (from && until) {
        parts.push(`${from}~${until}`);
      } else if (from) {
        parts.push(`${from}起`);
      } else if (until) {
        parts.push(`至${until}`);
      }
    }

    return parts.length > 0 ? parts.join(' ') : t('settings.price_rule.time.always');
  };

  // Format adjustment value
  const formatAdjustment = (rule: PriceRule): string => {
    const sign = rule.rule_type === 'DISCOUNT' ? '-' : '+';
    if (rule.adjustment_type === 'PERCENTAGE') {
      return `${sign}${rule.adjustment_value}%`;
    }
    return `${sign}€${rule.adjustment_value.toFixed(2)}`;
  };

  // Get stacking mode label
  const getStackingLabel = (rule: PriceRule): string => {
    if (rule.is_exclusive) return t('settings.price_rule.stacking.exclusive');
    if (rule.is_stackable) return t('settings.price_rule.stacking.stackable');
    return t('settings.price_rule.stacking.non_stackable');
  };

  return (
    <div className="w-80 shrink-0 bg-gray-50 border-r border-gray-200 flex flex-col h-full overflow-hidden">
      {/* Header */}
      <div className="px-4 py-3 border-b border-gray-200 bg-white">
        <span className="text-sm font-medium text-gray-700">
          {t('settings.price_rule.list_title')} ({filteredRules.length})
        </span>
      </div>

      {/* Rule list */}
      <div className="flex-1 overflow-y-auto p-2 space-y-2">
        {filteredRules.length === 0 ? (
          <div className="text-center py-8 text-gray-400 text-sm">
            {searchQuery ? t('common.empty.no_results') : t('settings.price_rule.empty')}
          </div>
        ) : (
          filteredRules.map(rule => {
            const isSelected = rule.id === selectedRuleId;
            const isDiscount = rule.rule_type === 'DISCOUNT';
            const ProductScopeIcon = PRODUCT_SCOPE_ICONS[rule.product_scope] || Globe;
            const ZoneIcon = getZoneIcon(rule.zone_scope);

            return (
              <button
                key={rule.id}
                onClick={() => onSelectRule(rule.id)}
                className={`
                  w-full text-left p-3 rounded-xl transition-all duration-150
                  ${isSelected
                    ? `ring-2 ${isDiscount ? 'ring-amber-400 bg-amber-50' : 'ring-purple-400 bg-purple-50'}`
                    : `bg-white hover:bg-gray-50 ${!rule.is_active ? 'opacity-50' : ''}`
                  }
                  ${isSelected ? 'shadow-md' : 'shadow-sm hover:shadow'}
                `}
              >
                {/* Row 1: Status dot + Name + Adjustment */}
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2 min-w-0">
                    <span
                      className={`w-2 h-2 rounded-full shrink-0 ${
                        !rule.is_active
                          ? 'bg-gray-400'
                          : isDiscount
                            ? 'bg-amber-500'
                            : 'bg-purple-500'
                      }`}
                    />
                    <span className="font-medium text-gray-900 truncate">
                      {rule.display_name}
                    </span>
                  </div>
                  <span
                    className={`text-sm font-bold shrink-0 ${
                      isDiscount ? 'text-amber-600' : 'text-purple-600'
                    }`}
                  >
                    {formatAdjustment(rule)}
                  </span>
                </div>

                {/* Row 2: Product scope + Zone scope */}
                <div className="flex items-center gap-2 mt-1.5 text-xs text-gray-500">
                  <span className="inline-flex items-center gap-1">
                    <ProductScopeIcon size={12} />
                    {t(`settings.price_rule.scope.${rule.product_scope.toLowerCase()}`)}
                  </span>
                  <span className="text-gray-300">·</span>
                  <span className="inline-flex items-center gap-1">
                    <ZoneIcon size={12} />
                    {getZoneName(rule.zone_scope)}
                  </span>
                </div>

                {/* Row 3: Time summary */}
                <div className="text-xs text-gray-400 mt-1">
                  {formatTimeSummary(rule)}
                </div>

                {/* Row 4: Priority + Stacking */}
                <div className="flex items-center gap-2 mt-1.5 text-xs">
                  <span className="text-gray-400">
                    {t('settings.price_rule.priority')}: {rule.is_exclusive ? 99 : (rule.is_stackable ? 1 : 50)}
                  </span>
                  <span className="text-gray-300">·</span>
                  <span className={`${rule.is_exclusive ? 'text-red-500' : rule.is_stackable ? 'text-green-500' : 'text-gray-500'}`}>
                    {getStackingLabel(rule)}
                  </span>
                </div>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
};
