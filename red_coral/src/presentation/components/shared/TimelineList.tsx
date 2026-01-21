import React from 'react';
import { TimelineEvent } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { useTimelineEvent } from './Timeline/useTimelineEvent';
import { TimelineItem, NoteTag } from './Timeline/TimelineItem';

export { NoteTag }; // Re-export for compatibility

interface TimelineListProps {
    events: TimelineEvent[];
    showNoteTags?: boolean;
    showHidden?: boolean;
}

const TimelineItemWrapper = React.memo<{ event: TimelineEvent; showNoteTags?: boolean; showHidden?: boolean }>(({ event, showNoteTags, showHidden }) => {
    const displayData = useTimelineEvent(event, showHidden);
    if (!showHidden && displayData.isHidden) return null;
    return <TimelineItem data={displayData} showNoteTags={showNoteTags} />;
});

TimelineItemWrapper.displayName = 'TimelineItemWrapper';

export const TimelineList = React.memo<TimelineListProps>(({ events, showNoteTags = true, showHidden = false }) => {
    const { t } = useI18n();

    // Deduplicate events based on ID to prevent "Encountered two children with the same key" error
    // and remove potentially erroneous duplicate nodes.
    const uniqueEvents = React.useMemo(() => {
        const seen = new Set<string>();
        return events.filter(event => {
            if (seen.has(event.id)) {
                return false;
            }
            seen.add(event.id);
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
                    key={event.id || `${event.type}-${event.timestamp}-${idx}`}
                    event={event}
                    showNoteTags={showNoteTags}
                    showHidden={showHidden}
                />
            ))}
        </div>
    );
});

TimelineList.displayName = 'TimelineList';
