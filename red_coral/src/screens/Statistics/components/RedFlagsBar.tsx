import React, { useState } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { AlertTriangle, ChevronDown, ChevronUp } from 'lucide-react';

interface RedFlagsSummary {
  item_removals: number;
  item_comps: number;
  order_voids: number;
  order_discounts: number;
  price_modifications: number;
}

interface OperatorRedFlags {
  operator_id: number;
  operator_name: string;
  item_removals: number;
  item_comps: number;
  order_voids: number;
  order_discounts: number;
  price_modifications: number;
}

export interface RedFlagsData {
  summary: RedFlagsSummary;
  operator_breakdown: OperatorRedFlags[];
}

interface RedFlagsBarProps {
  data: RedFlagsData;
}

const FLAG_KEYS = ['item_removals', 'item_comps', 'order_voids', 'order_discounts', 'price_modifications'] as const;

export const RedFlagsBar: React.FC<RedFlagsBarProps> = ({ data }) => {
  const { t } = useI18n();
  const [expanded, setExpanded] = useState(false);

  const total = FLAG_KEYS.reduce((sum, key) => sum + data.summary[key], 0);
  if (total === 0) return null;

  const activeFlags = FLAG_KEYS
    .map(key => ({ key, count: data.summary[key], label: t(`statistics.red_flags.${key}`) }))
    .filter(f => f.count > 0);

  return (
    <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center justify-between"
      >
        <div className="flex items-center gap-3">
          <AlertTriangle className="w-5 h-5 text-red-500 shrink-0" />
          <div className="flex flex-wrap gap-x-4 gap-y-1">
            {activeFlags.map(f => (
              <span key={f.key} className="text-sm font-medium text-red-700">
                {f.label}: {f.count}
              </span>
            ))}
          </div>
        </div>
        {expanded
          ? <ChevronUp className="w-4 h-4 text-red-400 shrink-0" />
          : <ChevronDown className="w-4 h-4 text-red-400 shrink-0" />}
      </button>

      {expanded && data.operator_breakdown.length > 0 && (
        <div className="mt-3 pt-3 border-t border-red-200 overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-red-600 text-left">
                <th className="py-1 pr-4 font-medium">{t('statistics.red_flags.operator')}</th>
                {activeFlags.map(f => (
                  <th key={f.key} className="py-1 px-2 text-center font-medium">{f.label}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {data.operator_breakdown.map(op => (
                <tr key={op.operator_id} className="text-red-700">
                  <td className="py-1 pr-4">{op.operator_name || `#${op.operator_id}`}</td>
                  {activeFlags.map(f => (
                    <td key={f.key} className="py-1 px-2 text-center">
                      {op[f.key] || 0}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};
