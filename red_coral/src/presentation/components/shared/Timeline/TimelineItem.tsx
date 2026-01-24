import React from 'react';
import { getTagStyle } from '../CustomerInput';
import type { TimelineDisplayData } from './renderers';

const renderDetailText = (text: string) => {
    const parts = text.split(/(#\d{5})/g);
    return (
        <span>
            {parts.map((part, i) => {
                if (/^#\d{5}$/.test(part)) {
                    return (
                        <span key={i} className="mx-1 px-1 py-0.5 rounded text-[0.625rem] font-bold bg-blue-100 text-blue-800 border border-blue-200">
                            {part}
                        </span>
                    );
                }
                return <span key={i}>{part}</span>;
            })}
        </span>
    );
};

export const NoteTag = ({ text }: { text: string }) => {
    if (!text) return null;
    const parts = text.split(/[:：]/); 
    const name = parts[0].trim();
    const detail = parts.slice(1).join(':').trim();
    
    const styleClass = getTagStyle(name);

    return (
        <div className="flex items-center gap-2 text-sm">
            <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-bold border shadow-sm ${styleClass}`}>
                {name}
            </span>
            {detail && <span className="text-gray-500 text-xs">{renderDetailText(detail)}</span>}
        </div>
    );
};

interface TimelineItemProps {
    data: TimelineDisplayData;
    showNoteTags?: boolean;
}

export const TimelineItem: React.FC<TimelineItemProps> = ({ data, showNoteTags = true }) => {
    const { title, summary, details, icon: Icon, colorClass, customColor, timestamp, isHidden, tags } = data;

    if (isHidden) return null;

    const bgStyle: React.CSSProperties = customColor 
        ? { backgroundColor: customColor, borderColor: customColor }
        : {};

    const finalColorClass = customColor ? '' : colorClass;

    return (
        <div className="relative pl-6">
            <div 
                className={`absolute -left-[0.5625rem] top-0 w-5 h-5 rounded-full border-2 border-white flex items-center justify-center text-white ${finalColorClass}`}
                style={bgStyle}
            >
                {Icon && <Icon size={12} strokeWidth={2.5} />}
            </div>
            <div className="flex items-center gap-2 flex-wrap">
                <div className="text-sm font-bold text-gray-800">{title}</div>
                {tags && tags.map((tag, i) => (
                    <span key={i} className="px-1.5 py-0.5 rounded text-xs font-bold bg-blue-100 text-blue-800 border border-blue-200">
                        {tag}
                    </span>
                ))}
            </div>
            <div className="text-xs text-gray-400 font-mono">
                {new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}
            </div>
            <div className="font-medium text-gray-700 mt-0.5">{summary}</div>
            
            {details && details.length > 0 && (
                <div className="mt-1 bg-gray-50 p-2 rounded text-xs text-gray-600 space-y-0.5">
                    {details.map((detail, i) => (
                        <div key={`${timestamp}-${detail}-${i}`}>
                            {/* Check if it looks like a note tag (e.g. "Name: Note") */}
                            {showNoteTags && detail.includes(':') && !detail.includes('€') ? (
                                <NoteTag text={detail} />
                            ) : (
                                renderDetailText(detail)
                            )}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
};
