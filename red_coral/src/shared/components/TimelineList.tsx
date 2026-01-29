/**
 * TimelineList Component
 *
 * 职责：
 * - 接收 OrderEvent[]（服务端权威类型）
 * - 调用 Renderer 转换为 UI 数据
 * - 渲染 TimelineItem 组件
 */

import React from 'react';
import type { OrderEvent } from '@/core/domain/types/orderEvent';
import { useI18n } from '@/hooks/useI18n';
import { renderEvent } from './Timeline/renderers';
import { TimelineItem } from './Timeline/TimelineItem';

interface TimelineListProps {
  events: OrderEvent[];  // ✅ 使用后端类型
  showNoteTags?: boolean;
  showHidden?: boolean;
}

const TimelineItemWrapper = React.memo<{ event: OrderEvent; showNoteTags?: boolean; showHidden?: boolean }>(
  ({ event, showNoteTags, showHidden }) => {
    const { t } = useI18n();

    // ✅ 使用 Renderer 转换事件数据
    const displayData = renderEvent(event, t);

    if (!showHidden && displayData.isHidden) return null;

    return <TimelineItem data={displayData} showNoteTags={showNoteTags} />;
  }
);

TimelineItemWrapper.displayName = 'TimelineItemWrapper';

export const TimelineList = React.memo<TimelineListProps>(({ events, showNoteTags = true, showHidden = false }) => {
  const { t } = useI18n();

  // Deduplicate events based on event_id
  const uniqueEvents = React.useMemo(() => {
    const seen = new Set<string>();
    return events.filter(event => {
      if (seen.has(event.event_id)) {
        return false;
      }
      seen.add(event.event_id);
      return true;
    });
  }, [events]);

  if (uniqueEvents.length === 0) {
    return <div className="pl-6 text-gray-400 italic text-sm">{t('timeline.empty')}</div>;
  }

  return (
    <div className="relative border-l-2 border-gray-200 ml-3 space-y-6 py-2">
      {uniqueEvents.map((event, idx) => (
        <TimelineItemWrapper
          key={event.event_id || `${event.event_type}-${event.timestamp}-${idx}`}
          event={event}
          showNoteTags={showNoteTags}
          showHidden={showHidden}
        />
      ))}
    </div>
  );
});

TimelineList.displayName = 'TimelineList';
