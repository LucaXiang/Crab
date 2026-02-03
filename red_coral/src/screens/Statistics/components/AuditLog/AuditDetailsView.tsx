/**
 * Audit Details View Component (审计详情视图组件)
 *
 * 根据 renderers.ts 返回的 AuditDisplayData 渲染审计详情
 */

import React from 'react';
import type { AuditDisplayData } from './renderers';
import { ArrowRight } from 'lucide-react';

interface AuditDetailsViewProps {
  data: AuditDisplayData;
  emptyLabel: string;
}

export const AuditDetailsView: React.FC<AuditDetailsViewProps> = ({ data, emptyLabel }) => {
  if (data.isEmpty) {
    return <span className="text-gray-400 italic">{emptyLabel}</span>;
  }

  return (
    <div className="space-y-2">
      {/* 变更列表（UPDATE 操作） */}
      {data.changes && data.changes.length > 0 && (
        <div className="space-y-1.5">
          {data.changes.map((change, i) => (
            <div key={`change-${i}`} className="flex items-start gap-2 text-xs">
              <span className="font-medium text-gray-500 min-w-[5rem] shrink-0">
                {change.fieldLabel}:
              </span>
              <div className="flex items-center gap-1.5 flex-wrap">
                <span className="px-1.5 py-0.5 bg-red-50 text-red-700 rounded border border-red-200 font-mono">
                  {change.from}
                </span>
                <ArrowRight size={12} className="text-gray-400 shrink-0" />
                <span className="px-1.5 py-0.5 bg-green-50 text-green-700 rounded border border-green-200 font-mono">
                  {change.to}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* 常规行（CREATE/DELETE 或其他操作） */}
      {data.lines.length > 0 && (
        <div className="space-y-1.5">
          {data.lines.map((line, i) => (
            <div key={`line-${i}`} className="flex items-start gap-2">
              <span className="font-medium text-gray-500 min-w-[5rem] shrink-0">
                {line.label}:
              </span>
              <span className={`text-gray-700 break-all ${line.valueClass || ''}`}>
                {line.value}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
