import React from 'react';
import { Globe, Package, Tag, Layers, ShoppingCart, Armchair } from 'lucide-react';
import type { PriceRule } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';
import { useZoneStore } from '@/features/zone/store';
import { calculatePriority, getStackingMode } from '../utils';

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

// Day keys for i18n (Sunday = 0)
const DAY_KEYS = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'] as const;

// Week order starting from Monday: [1,2,3,4,5,6,0]
const WEEK_ORDER = [1, 2, 3, 4, 5, 6, 0];

export const RuleListPanel: React.FC<RuleListPanelProps> = ({
  rules,
  selectedRuleId,
  onSelectRule,
  searchQuery,
}) => {
  const { t, locale } = useI18n();
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

  // Format active days - compress consecutive ranges (same logic as TimeVisualization)
  const formatActiveDays = (activeDays: number[]): string => {
    // Sort by week order (Mon-Sun)
    const sorted = [...activeDays].sort((a, b) =>
      WEEK_ORDER.indexOf(a) - WEEK_ORDER.indexOf(b)
    );

    // Find consecutive ranges
    const ranges: number[][] = [];
    let currentRange: number[] = [sorted[0]];

    for (let i = 1; i < sorted.length; i++) {
      const prevIdx = WEEK_ORDER.indexOf(sorted[i - 1]);
      const currIdx = WEEK_ORDER.indexOf(sorted[i]);
      if (currIdx === prevIdx + 1) {
        currentRange.push(sorted[i]);
      } else {
        ranges.push(currentRange);
        currentRange = [sorted[i]];
      }
    }
    ranges.push(currentRange);

    // Format each range
    const listFormatter = new Intl.ListFormat(locale, { style: 'narrow', type: 'conjunction' });

    const parts = ranges.map(range => {
      if (range.length === 1) {
        return t(`calendar.days.${DAY_KEYS[range[0]]}`);
      } else if (range.length === 2) {
        const twoDays = range.map(d => t(`calendar.days.${DAY_KEYS[d]}`));
        return listFormatter.format(twoDays);
      } else {
        // Range of 3+: "一至五" / "L-V"
        return `${t(`calendar.days.${DAY_KEYS[range[0]]}`)}${t('settings.price_rule.time_viz.to')}${t(`calendar.days.${DAY_KEYS[range[range.length - 1]]}`)}`;
      }
    });

    return listFormatter.format(parts);
  };

  // Format time summary
  const formatTimeSummary = (rule: PriceRule): string => {
    const parts: string[] = [];

    // Days of week
    if (rule.active_days && rule.active_days.length > 0 && rule.active_days.length < 7) {
      parts.push(formatActiveDays(rule.active_days));
    }

    // Time range
    if (rule.active_start_time && rule.active_end_time) {
      parts.push(`${rule.active_start_time}-${rule.active_end_time}`);
    }

    // Date range
    if (rule.valid_from || rule.valid_until) {
      const formatDate = (ts: number) => new Date(ts).toLocaleDateString(locale, { month: 'numeric', day: 'numeric' });
      const from = rule.valid_from ? formatDate(rule.valid_from) : '';
      const until = rule.valid_until ? formatDate(rule.valid_until) : '';
      if (from && until) {
        parts.push(`${from}~${until}`);
      } else if (from) {
        parts.push(`${from} ${t('settings.price_rule.time_viz.onwards')}`);
      } else if (until) {
        parts.push(`${t('settings.price_rule.time_viz.until')} ${until}`);
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

  return (
    <div className="w-80 shrink-0 flex flex-col h-full overflow-hidden bg-gray-50">
      {/* Header */}
      <div className="px-4 py-3 border-b border-gray-200">
        <span className="text-sm font-medium text-gray-700">
          {t('settings.price_rule.list_title')} ({filteredRules.length})
        </span>
      </div>

      {/* Rule list */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
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
                    P{calculatePriority(rule)}
                  </span>
                  <span className="text-gray-300">·</span>
                  <span className={`${rule.is_exclusive ? 'text-red-500' : rule.is_stackable ? 'text-green-500' : 'text-gray-500'}`}>
                    {t(`settings.price_rule.stacking.${getStackingMode(rule)}`)}
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
